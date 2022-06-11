use super::{RvIsaState, StopReason};
use crate::rv::{RvDecoder, RvInsn};

mod syscall;

pub struct RvInterpreterExecutor<'a> {
    state: &'a mut RvIsaState,
    guest_base: *mut u8,

    decoder: RvDecoder,
}

impl<'a> RvInterpreterExecutor<'a> {
    pub fn new(state: &'a mut RvIsaState, guest_base: *mut u8) -> Self {
        Self {
            state,
            guest_base,
            decoder: RvDecoder::new(64),
        }
    }

    fn get_u8(&self, gaddr: isize) -> u8 {
        unsafe {
            let haddr = self.guest_base.offset(gaddr);
            (haddr as *const u8).read()
        }
    }

    fn get_u16(&self, gaddr: isize) -> u16 {
        unsafe {
            let haddr = self.guest_base.offset(gaddr);
            (haddr as *const u16).read()
        }
    }

    fn get_u32(&self, gaddr: isize) -> u32 {
        unsafe {
            let haddr = self.guest_base.offset(gaddr);
            (haddr as *const u32).read()
        }
    }

    fn get_u64(&self, gaddr: isize) -> u64 {
        unsafe {
            let haddr = self.guest_base.offset(gaddr);
            (haddr as *const u64).read()
        }
    }

    // returns None if successful exit
    pub fn exec(&mut self, entry_pc: u64) -> Option<StopReason> {
        self.state.set_pc(entry_pc);

        loop {
            let x = self.exec_one();
            match x {
                StopReason::Next | StopReason::ContinueAt(_) => {}
                _ => return Some(x),
            }
        }
    }

    fn exec_one(&mut self) -> StopReason {
        let pc = self.state.get_pc();
        // println!("pc = {:016x}", pc);
        let res = {
            let mem = unsafe { std::slice::from_raw_parts(self.guest_base.offset(pc as isize), 4) };
            // println!("looking at {:?}", mem);
            self.decoder.disas(mem)
        };
        // println!("decoded {:?}", res);
        if res.is_none() {
            panic!("cannot fetch insn");
        }
        let (insn, len) = res.unwrap();

        let res = self.interpret_one(&insn, len);

        let new_pc = if let StopReason::ContinueAt(x) = res {
            x
        } else {
            pc + (len as u64)
        };
        self.state.set_pc(new_pc);

        res
    }

    // returns next pc
    fn interpret_one(&mut self, insn: &RvInsn, insn_len: usize) -> StopReason {
        match insn {
            RvInsn::Invalid(_) => StopReason::ReservedInsn,
            RvInsn::Ecall => self.do_syscall(),
            RvInsn::Ebreak => StopReason::Break,
            RvInsn::Lui(a) => {
                self.state.set_x(a.rd, a.imm as u64);
                StopReason::Next
            }
            RvInsn::Auipc(a) => {
                self.state
                    .set_x(a.rd, (self.state.get_pc() as i64 + a.imm as i64) as u64);
                StopReason::Next
            }
            RvInsn::Jal(a) => {
                let pc = self.state.get_pc();
                self.state.set_x(a.rd, pc + insn_len as u64);
                StopReason::ContinueAt((pc as i64 + a.imm as i64) as u64)
            }
            RvInsn::Jalr(a) => {
                let pc = self.state.get_pc();
                self.state.set_x(a.rd, pc + insn_len as u64);
                StopReason::ContinueAt((self.state.get_x(a.rs1) as i64 + a.imm as i64) as u64)
            }
            RvInsn::Beq(_) => todo!(),
            RvInsn::Bne(_) => todo!(),
            RvInsn::Blt(_) => todo!(),
            RvInsn::Bge(_) => todo!(),
            RvInsn::Bltu(_) => todo!(),
            RvInsn::Bgeu(_) => todo!(),
            RvInsn::Lb(_) => todo!(),
            RvInsn::Lh(_) => todo!(),
            RvInsn::Lw(_) => todo!(),
            RvInsn::Lbu(_) => todo!(),
            RvInsn::Lhu(_) => todo!(),
            RvInsn::Sb(_) => todo!(),
            RvInsn::Sh(_) => todo!(),
            RvInsn::Sw(_) => todo!(),
            RvInsn::Addi(a) => {
                let v = self.state.get_x(a.rs1) as i64 + a.imm as i64;
                self.state.set_x(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::Slti(_) => todo!(),
            RvInsn::Sltiu(_) => todo!(),
            RvInsn::Xori(_) => todo!(),
            RvInsn::Ori(_) => todo!(),
            RvInsn::Andi(_) => todo!(),
            RvInsn::Slli(_) => todo!(),
            RvInsn::Srli(_) => todo!(),
            RvInsn::Srai(_) => todo!(),
            RvInsn::Add(_) => todo!(),
            RvInsn::Sub(_) => todo!(),
            RvInsn::Sll(_) => todo!(),
            RvInsn::Slt(_) => todo!(),
            RvInsn::Sltu(_) => todo!(),
            RvInsn::Xor(_) => todo!(),
            RvInsn::Srl(_) => todo!(),
            RvInsn::Sra(_) => todo!(),
            RvInsn::Or(_) => todo!(),
            RvInsn::And(_) => todo!(),
            RvInsn::Fence(_) => todo!(),
            RvInsn::FenceI(_) => todo!(),
            RvInsn::Lwu(_) => todo!(),
            RvInsn::Ld(_) => todo!(),
            RvInsn::Sd(_) => todo!(),
            RvInsn::Addiw(_) => todo!(),
            RvInsn::Slliw(_) => todo!(),
            RvInsn::Srliw(_) => todo!(),
            RvInsn::Sraiw(_) => todo!(),
            RvInsn::Addw(_) => todo!(),
            RvInsn::Subw(_) => todo!(),
            RvInsn::Sllw(_) => todo!(),
            RvInsn::Srlw(_) => todo!(),
            RvInsn::Sraw(_) => todo!(),
            RvInsn::Mul(_) => todo!(),
            RvInsn::Mulh(_) => todo!(),
            RvInsn::Mulhsu(_) => todo!(),
            RvInsn::Mulhu(_) => todo!(),
            RvInsn::Div(_) => todo!(),
            RvInsn::Divu(_) => todo!(),
            RvInsn::Rem(_) => todo!(),
            RvInsn::Remu(_) => todo!(),
            RvInsn::Mulw(_) => todo!(),
            RvInsn::Divw(_) => todo!(),
            RvInsn::Divuw(_) => todo!(),
            RvInsn::Remw(_) => todo!(),
            RvInsn::Remuw(_) => todo!(),
            RvInsn::LrW(_) => todo!(),
            RvInsn::ScW(_) => todo!(),
            RvInsn::AmoSwapW(_) => todo!(),
            RvInsn::AmoAddW(_) => todo!(),
            RvInsn::AmoXorW(_) => todo!(),
            RvInsn::AmoAndW(_) => todo!(),
            RvInsn::AmoOrW(_) => todo!(),
            RvInsn::AmoMinW(_) => todo!(),
            RvInsn::AmoMaxW(_) => todo!(),
            RvInsn::AmoMinuW(_) => todo!(),
            RvInsn::AmoMaxuW(_) => todo!(),
            RvInsn::LrD(_) => todo!(),
            RvInsn::ScD(_) => todo!(),
            RvInsn::AmoSwapD(_) => todo!(),
            RvInsn::AmoAddD(_) => todo!(),
            RvInsn::AmoXorD(_) => todo!(),
            RvInsn::AmoAndD(_) => todo!(),
            RvInsn::AmoOrD(_) => todo!(),
            RvInsn::AmoMinD(_) => todo!(),
            RvInsn::AmoMaxD(_) => todo!(),
            RvInsn::AmoMinuD(_) => todo!(),
            RvInsn::AmoMaxuD(_) => todo!(),
            RvInsn::Flw(_) => todo!(),
            RvInsn::Fsw(_) => todo!(),
            RvInsn::FmaddS(_) => todo!(),
            RvInsn::FmsubS(_) => todo!(),
            RvInsn::FnmsubS(_) => todo!(),
            RvInsn::FnmaddS(_) => todo!(),
            RvInsn::FaddS(_) => todo!(),
            RvInsn::FsubS(_) => todo!(),
            RvInsn::FmulS(_) => todo!(),
            RvInsn::FdivS(_) => todo!(),
            RvInsn::FsqrtS(_) => todo!(),
            RvInsn::FsgnjS(_) => todo!(),
            RvInsn::FsgnjnS(_) => todo!(),
            RvInsn::FsgnjxS(_) => todo!(),
            RvInsn::FminS(_) => todo!(),
            RvInsn::FmaxS(_) => todo!(),
            RvInsn::FcvtWS(_) => todo!(),
            RvInsn::FcvtWuS(_) => todo!(),
            RvInsn::FmvXW(_) => todo!(),
            RvInsn::FeqS(_) => todo!(),
            RvInsn::FltS(_) => todo!(),
            RvInsn::FleS(_) => todo!(),
            RvInsn::FclassS(_) => todo!(),
            RvInsn::FcvtSW(_) => todo!(),
            RvInsn::FcvtSWu(_) => todo!(),
            RvInsn::FmvWX(_) => todo!(),
            RvInsn::FcvtLS(_) => todo!(),
            RvInsn::FcvtLuS(_) => todo!(),
            RvInsn::FcvtSL(_) => todo!(),
            RvInsn::FcvtSLu(_) => todo!(),
            RvInsn::Fld(_) => todo!(),
            RvInsn::Fsd(_) => todo!(),
            RvInsn::FmaddD(_) => todo!(),
            RvInsn::FmsubD(_) => todo!(),
            RvInsn::FnmsubD(_) => todo!(),
            RvInsn::FnmaddD(_) => todo!(),
            RvInsn::FaddD(_) => todo!(),
            RvInsn::FsubD(_) => todo!(),
            RvInsn::FmulD(_) => todo!(),
            RvInsn::FdivD(_) => todo!(),
            RvInsn::FsqrtD(_) => todo!(),
            RvInsn::FsgnjD(_) => todo!(),
            RvInsn::FsgnjnD(_) => todo!(),
            RvInsn::FsgnjxD(_) => todo!(),
            RvInsn::FminD(_) => todo!(),
            RvInsn::FmaxD(_) => todo!(),
            RvInsn::FcvtSD(_) => todo!(),
            RvInsn::FcvtDS(_) => todo!(),
            RvInsn::FeqD(_) => todo!(),
            RvInsn::FltD(_) => todo!(),
            RvInsn::FleD(_) => todo!(),
            RvInsn::FclassD(_) => todo!(),
            RvInsn::FcvtWD(_) => todo!(),
            RvInsn::FcvtWuD(_) => todo!(),
            RvInsn::FcvtDW(_) => todo!(),
            RvInsn::FcvtDWu(_) => todo!(),
            RvInsn::FcvtLD(_) => todo!(),
            RvInsn::FcvtLuD(_) => todo!(),
            RvInsn::FmvXD(_) => todo!(),
            RvInsn::FcvtDL(_) => todo!(),
            RvInsn::FcvtDLu(_) => todo!(),
            RvInsn::FmvDX(_) => todo!(),
        }
    }
}
