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

        // Log syscall to file if LARVA_SYSCALL_LOG is set
        let syscall_log_file = std::env::var("LARVA_SYSCALL_LOG").ok();

        let result = match nr {
            64 => self.do_sys_write(arg0, arg1, arg2),
            66 => self.do_sys_writev(arg0, arg1, arg2),
            79 => self.do_sys_newfstatat(arg0, arg1, arg2, arg3),
            93 => self.do_sys_exit(arg0),
            94 => self.do_sys_exit_group(arg0),
            96 => self.do_sys_set_tid_address(arg1),
            135 => self.do_sys_rt_sigprocmask(arg0, arg1, arg2, arg3),
            160 => self.do_sys_uname(arg0),
            214 => self.do_sys_brk(arg0),
            222 => self.do_sys_mmap(arg0, arg1, arg2, arg3, arg4, arg5),
            226 => self.do_sys_mprotect(arg0, arg1, arg2),
            261 => self.do_sys_prlimit64(arg0, arg1, arg2, arg3),
            278 => self.do_sys_getrandom(arg0, arg1, arg2),

            _ => {
                println!(
                    "unimplemented syscall: {nr} ({arg0:#x}, {arg1:#x}, {arg2:#x}, {arg3:#x}, {arg4:#x}, {arg5:#x})"
                );
                self.state.set_x(10, u64::wrapping_neg(38)); // -ENOSYS
                StopReason::Next
            }
        };

        // Log syscall result if logging is enabled
        if let Some(path) = syscall_log_file {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
                let ret = self.state.get_x(10);
                let ret_str = if ret >= u64::MAX - 4096 {
                    format!("-{}", u64::MAX - ret + 1)
                } else {
                    format!("{ret}")
                };
                let _ = writeln!(file, "{nr}({arg0:#x},{arg1:#x},{arg2:#x},{arg3:#x},{arg4:#x},{arg5:#x}) = {ret_str}");
            }
        }

        result
    }

    fn do_sys_exit(&mut self, exitcode: u64) -> ! {
        // Exit the current thread (in our emulator, just exit the process)
        unsafe {
            libc::syscall(libc::SYS_exit, exitcode as i64);
        }
        unreachable!();
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

    fn do_sys_rt_sigprocmask(&mut self, how: u64, set_gaddr: u64, oldset_gaddr: u64, sigsetsize: u64) -> StopReason {
        // rt_sigprocmask - examine and change blocked signals
        // For now, just return success
        // how: SIG_BLOCK=0, SIG_UNBLOCK=1, SIG_SETMASK=2
        let _ = (how, set_gaddr, oldset_gaddr, sigsetsize);
        self.sx(10, 0); // Success
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

    fn do_sys_mmap(&mut self, addr: u64, len: u64, prot: u64, flags: u64, fd: u64, _offset: u64) -> StopReason {
        // mmap - map files or devices into memory
        // For now, support anonymous mappings (MAP_ANONYMOUS)
        // flags & 0x20 = MAP_ANONYMOUS
        
        const MAP_ANONYMOUS: u64 = 0x20;
        const MAP_FIXED: u64 = 0x10;
        
        let is_anon = (flags & MAP_ANONYMOUS) != 0;
        let is_fixed = (flags & MAP_FIXED) != 0;
        
        if is_anon {
            // Anonymous mapping - allocate memory in our MMU
            use crate::exec::mem::{GuestAddr, MemPerms};
            
            // Convert prot to MemPerms
            let perms = MemPerms {
                read: (prot & 1) != 0,
                write: (prot & 2) != 0,
                exec: (prot & 4) != 0,
            };
            
            let result = if is_fixed && addr != 0 {
                // MAP_FIXED - map at specific address
                self.mmu.mmap_fixed(GuestAddr(addr), len as usize, perms, false)
                    .map(|_| addr)
            } else {
                // Regular anonymous mapping - let MMU choose address
                self.mmu.mmap(len as usize, perms, false)
                    .map(|gaddr| gaddr.as_u64())
            };
            
            match result {
                Ok(gaddr) => {
                    self.sx(10, gaddr);
                }
                Err(_) => {
                    self.sx(10, u64::wrapping_neg(12)); // -ENOMEM
                }
            }
        } else {
            // File-backed mapping - not yet supported
            eprintln!("mmap: file-backed mapping not supported (fd={})", fd);
            self.sx(10, u64::wrapping_neg(38)); // -ENOSYS
        }
        
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

    fn do_sys_prlimit64(&mut self, pid: u64, resource: u64, new_limit_gaddr: u64, old_limit_gaddr: u64) -> StopReason {
        // prlimit64 - get/set resource limits
        // For now, just return success with default values for RLIMIT_STACK
        // resource 3 = RLIMIT_STACK
        
        if resource == 3 && old_limit_gaddr != 0 {
            // Write default stack limit to old_limit
            // struct rlimit64 { rlim64_t rlim_cur; rlim64_t rlim_max; }
            // Each is 8 bytes, total 16 bytes
            if let Some(haddr) = self.mmu.g2h(GuestAddr(old_limit_gaddr)) {
                unsafe {
                    // rlim_cur = 8MB (default stack size)
                    (haddr.as_mut_ptr::<u64>()).write(8 * 1024 * 1024);
                    // rlim_max = RLIM_INFINITY (~0)
                    (haddr.as_mut_ptr::<u64>().add(1)).write(u64::MAX);
                }
            }
        }
        
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
