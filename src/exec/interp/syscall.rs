use libc;

use super::{RvInterpreterExecutor, StopReason};

impl<'a> RvInterpreterExecutor<'a> {
    pub(super) fn do_syscall(&mut self) -> StopReason {
        let nr = self.state.get_x(17); // a7
        let arg0 = self.state.get_x(10); // a0
        let arg1 = self.state.get_x(11); // a1
        let arg2 = self.state.get_x(12); // a2
        let arg3 = self.state.get_x(13); // a3
        let arg4 = self.state.get_x(14); // a4
        let arg5 = self.state.get_x(15); // a5

        match nr {
            // exit_group
            93 => self.do_sys_exit_group(arg0),

            _ => {
                println!(
                    "unimplemented syscall: {} ({:#x}, {:#x}, {:#x}, {:#x}, {:#x}, {:#x})",
                    nr, arg0, arg1, arg2, arg3, arg4, arg5
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
}
