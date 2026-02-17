use libc::{self, iovec};

use super::{RvInterpreterExecutor, StopReason};
use crate::exec::mem::GuestAddr;

/// Guest iovec structure (matching Linux RISC-V)
#[repr(C)]
struct GuestIovec {
    iov_base: u64,
    iov_len: u64,
}

impl<'a> RvInterpreterExecutor<'a> {
    pub(super) fn do_syscall(&mut self) -> StopReason {
        let nr = self.state.get_x(17); // a7
        let arg0 = self.state.get_x(10); // a0
        let arg1 = self.state.get_x(11); // a1
        let arg2 = self.state.get_x(12); // a2
        let arg3 = self.state.get_x(13); // a3
        let arg4 = self.state.get_x(14); // a4
        let arg5 = self.state.get_x(15); // a5

        if self.debug {
            println!(
                "syscall: {nr} ({arg0:#x}, {arg1:#x}, {arg2:#x}, {arg3:#x}, {arg4:#x}, {arg5:#x})"
            );
        }
        match nr {
            64 => self.do_sys_write(arg0, arg1, arg2),
            66 => self.do_sys_writev(arg0, arg1, arg2),
            79 => self.do_sys_newfstatat(arg0, arg1, arg2, arg3),
            93 => self.do_sys_exit_group(arg0),
            96 => self.do_sys_set_tid_address(arg1),
            160 => self.do_sys_uname(arg0),
            214 => self.do_sys_brk(arg0),
            226 => self.do_sys_mprotect(arg0, arg1, arg2),
            278 => self.do_sys_getrandom(arg0, arg1, arg2),

            _ => {
                println!(
                    "unimplemented syscall: {nr} ({arg0:#x}, {arg1:#x}, {arg2:#x}, {arg3:#x}, {arg4:#x}, {arg5:#x})"
                );
                self.state.set_x(10, u64::wrapping_neg(38)); // -ENOSYS
                StopReason::Next
            }
        }
    }

    fn do_sys_exit_group(&mut self, exitcode: u64) -> ! {
        unsafe {
            libc::syscall(libc::SYS_exit_group, exitcode as i64);
        }
        unreachable!();
    }

    fn do_sys_write(&mut self, fd: u64, buf_gaddr: u64, count: u64) -> StopReason {
        // Translate guest buffer address to host address
        let buf_haddr = match self.mmu.g2h(GuestAddr(buf_gaddr)) {
            Some(addr) => addr.as_ptr::<u8>(),
            None => {
                // Segfault - bad address
                return StopReason::Segv {
                    read: true,
                    gaddr: buf_gaddr,
                };
            }
        };

        let ret = unsafe { libc::syscall(libc::SYS_write, fd as i64, buf_haddr, count as i64) };
        self.sx(10, ret as u64);
        StopReason::Next
    }

    fn do_sys_writev(&mut self, fd: u64, iov_gaddr: u64, iovcnt: u64) -> StopReason {
        if iovcnt == 0 {
            self.sx(10, 0);
            return StopReason::Next;
        }

        // Translate iovec array address
        let iov_haddr = match self.mmu.g2h(GuestAddr(iov_gaddr)) {
            Some(addr) => addr.as_ptr::<GuestIovec>(),
            None => {
                return StopReason::Segv {
                    read: true,
                    gaddr: iov_gaddr,
                };
            }
        };

        // Build host iovec array
        let mut host_iov: Vec<iovec> = Vec::with_capacity(iovcnt as usize);

        unsafe {
            for i in 0..iovcnt as usize {
                let guest_iov = &*iov_haddr.add(i);

                // Translate each buffer address
                let buf_haddr = match self.mmu.g2h(GuestAddr(guest_iov.iov_base)) {
                    Some(addr) => addr.as_ptr::<u8>(),
                    None => {
                        return StopReason::Segv {
                            read: true,
                            gaddr: guest_iov.iov_base,
                        };
                    }
                };

                host_iov.push(iovec {
                    iov_base: buf_haddr as *mut libc::c_void,
                    iov_len: guest_iov.iov_len as usize,
                });
            }
        }

        let ret = unsafe {
            libc::syscall(
                libc::SYS_writev,
                fd as i64,
                host_iov.as_ptr(),
                iovcnt as i64,
            )
        };
        self.sx(10, ret as u64);
        StopReason::Next
    }

    fn do_sys_set_tid_address(&mut self, _tidptr: u64) -> StopReason {
        // For now, just return the current PID
        // In a real implementation, this would set the clear_child_tid address
        let ret = unsafe { libc::syscall(libc::SYS_getpid) };
        self.sx(10, ret as u64);
        StopReason::Next
    }

    fn do_sys_brk(&mut self, addr: u64) -> StopReason {
        // Simple brk implementation - use the end of the loaded ELF as the base
        // In a real implementation, we'd track this properly
        const BRK_BASE: u64 = 0x0000_0000_001c_0000; // Arbitrary high address for heap
        const BRK_MAX: u64 = 0x0000_0000_0020_0000;

        static mut CUR_BRK: u64 = BRK_BASE;

        let ret = if addr == 0 {
            // Query current brk
            unsafe { CUR_BRK }
        } else if addr < BRK_BASE {
            // Invalid - below base
            unsafe { CUR_BRK }
        } else if addr > BRK_MAX {
            // Can't extend that far
            unsafe { CUR_BRK }
        } else {
            // Extend brk
            unsafe {
                CUR_BRK = addr;
                CUR_BRK
            }
        };

        self.sx(10, ret);
        StopReason::Next
    }

    fn do_sys_uname(&mut self, buf_gaddr: u64) -> StopReason {
        // Translate guest buffer address
        let buf_haddr = match self.mmu.g2h(GuestAddr(buf_gaddr)) {
            Some(addr) => addr.as_mut_ptr::<u8>(),
            None => {
                return StopReason::Segv {
                    read: false,
                    gaddr: buf_gaddr,
                };
            }
        };

        // struct utsname is 390 bytes on Linux (6 fields of 65 bytes each)
        // sysname, nodename, release, version, machine, domainname
        const UTSNAME_LEN: usize = 65;
        const UTSNAME_SIZE: usize = UTSNAME_LEN * 6;

        unsafe {
            // Zero the buffer first
            std::ptr::write_bytes(buf_haddr, 0, UTSNAME_SIZE);

            // Fill in the fields
            let sysname = b"Linux\0";
            let nodename = b"larva\0";
            let release = b"5.15.0\0";
            let version = b"#1 LARVa\0";
            let machine = b"riscv64\0";
            let domainname = b"\0";

            std::ptr::copy_nonoverlapping(sysname.as_ptr(), buf_haddr, sysname.len());
            std::ptr::copy_nonoverlapping(nodename.as_ptr(), buf_haddr.add(UTSNAME_LEN), nodename.len());
            std::ptr::copy_nonoverlapping(release.as_ptr(), buf_haddr.add(UTSNAME_LEN * 2), release.len());
            std::ptr::copy_nonoverlapping(version.as_ptr(), buf_haddr.add(UTSNAME_LEN * 3), version.len());
            std::ptr::copy_nonoverlapping(machine.as_ptr(), buf_haddr.add(UTSNAME_LEN * 4), machine.len());
            std::ptr::copy_nonoverlapping(domainname.as_ptr(), buf_haddr.add(UTSNAME_LEN * 5), domainname.len());
        }

        self.sx(10, 0); // Success
        StopReason::Next
    }

    fn do_sys_getrandom(&mut self, buf_gaddr: u64, buflen: u64, _flags: u64) -> StopReason {
        // Translate guest buffer address
        let buf_haddr = match self.mmu.g2h(GuestAddr(buf_gaddr)) {
            Some(addr) => addr.as_mut_ptr::<u8>(),
            None => {
                return StopReason::Segv {
                    read: false,
                    gaddr: buf_gaddr,
                };
            }
        };

        // Fill with pseudo-random data (zeros for now - musl just needs this to not fail)
        let len = buflen as usize;
        unsafe {
            std::ptr::write_bytes(buf_haddr, 0x42, len);
        }

        self.sx(10, buflen); // Return number of bytes written
        StopReason::Next
    }

    fn do_sys_mprotect(&mut self, addr: u64, len: u64, prot: u64) -> StopReason {
        // For now, just pretend it worked
        // In a real implementation, we'd update the MMU permissions
        // prot bits: PROT_READ=1, PROT_WRITE=2, PROT_EXEC=4
        let _ = (addr, len, prot);
        self.sx(10, 0); // Success
        StopReason::Next
    }

    fn do_sys_newfstatat(&mut self, dirfd: u64, pathname_gaddr: u64, statbuf_gaddr: u64, flags: u64) -> StopReason {
        // Translate pathname
        let pathname_haddr = match self.mmu.g2h(GuestAddr(pathname_gaddr)) {
            Some(addr) => addr.as_ptr::<u8>(),
            None => {
                return StopReason::Segv {
                    read: true,
                    gaddr: pathname_gaddr,
                };
            }
        };

        // Get pathname as string
        let pathname = unsafe {
            let len = libc::strlen(pathname_haddr as *const libc::c_char);
            std::slice::from_raw_parts(pathname_haddr, len)
        };
        let pathname_str = String::from_utf8_lossy(pathname);

        // For /etc/busybox.conf, return ENOENT (file not found)
        // This is expected behavior - busybox probes for config file
        if pathname_str == "/etc/busybox.conf" {
            self.sx(10, u64::wrapping_neg(2)); // -ENOENT
            return StopReason::Next;
        }

        // For other files, return ENOSYS for now
        let _ = (dirfd, statbuf_gaddr, flags);
        self.sx(10, u64::wrapping_neg(38)); // -ENOSYS
        StopReason::Next
    }
}
