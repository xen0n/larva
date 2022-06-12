pub mod interp;
pub mod mem;

#[derive(Debug)]
pub enum StopReason {
    Next,
    ContinueAt(u64),

    Break,
    ReservedInsn,
    Segv { read: bool, gaddr: u64 },
}

#[derive(PartialEq, Debug, Default)]
pub struct RvIsaState {
    pc: u64,
    regs_x: [u64; 31],
    regs_f: [f64; 32],
}

impl RvIsaState {
    pub fn get_pc(&self) -> u64 {
        self.pc
    }

    pub fn set_pc(&mut self, val: u64) {
        self.pc = val;
    }

    pub fn get_x(&self, idx: u8) -> u64 {
        debug_assert!(idx < 32);
        if idx == 0 {
            0
        } else {
            self.regs_x[idx as usize - 1]
        }
    }

    pub fn set_x(&mut self, idx: u8, val: u64) {
        debug_assert!(idx < 32);
        if idx == 0 {
            return;
        }
        self.regs_x[idx as usize - 1] = val;
    }

    pub fn get_f64(&self, idx: u8) -> f64 {
        debug_assert!(idx < 32);
        self.regs_f[idx as usize]
    }

    pub fn set_f64(&mut self, idx: u8, val: f64) {
        debug_assert!(idx < 32);
        self.regs_f[idx as usize] = val;
    }
}
