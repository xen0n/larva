use libc;

use super::{RvInterpreterExecutor, StopReason};
use crate::exec::mem::GuestAddr;

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
            // exit_group
            93 => self.do_sys_exit_group(arg0),

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
}
