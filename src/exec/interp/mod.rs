use super::mem::{GuestAddr, GuestMmu};
use super::{RvIsaState, StopReason};
use crate::rv::{RvDecoder, RvInsn};

mod syscall;

pub struct RvInterpreterExecutor<'a> {
    shamt_mask: u64,

    state: &'a mut RvIsaState,
    mmu: &'a mut GuestMmu,

    decoder: RvDecoder,
}

fn sext_u8(x: u8) -> u64 {
    x as i8 as i64 as u64
}

fn sext_u16(x: u16) -> u64 {
    x as i16 as i64 as u64
}

fn sext_u32(x: u32) -> u64 {
    x as i32 as i64 as u64
}

impl<'a> RvInterpreterExecutor<'a> {
    pub fn new(xlen: usize, state: &'a mut RvIsaState, mmu: &'a mut GuestMmu) -> Self {
        Self {
            shamt_mask: (xlen - 1) as u64,
            state,
            mmu,
            decoder: RvDecoder::new(xlen),
        }
    }

    pub fn stack(&mut self, len: usize) -> ::std::io::Result<()> {
        let stack_block = self.mmu.mmap(len, true)?;
        let stack_top = stack_block + len;
        self.state.set_x(2, stack_top.as_u64());
        Ok(())
    }

    fn get_u8(&self, gaddr: GuestAddr) -> Result<u8, StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            Ok(unsafe { (haddr.as_u64() as *const u8).read() })
        } else {
            Err(StopReason::Segv {
                read: true,
                gaddr: gaddr.into(),
            })
        }
    }

    fn get_u16(&self, gaddr: GuestAddr) -> Result<u16, StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            Ok(unsafe { (haddr.as_u64() as *const u16).read() })
        } else {
            Err(StopReason::Segv {
                read: true,
                gaddr: gaddr.into(),
            })
        }
    }

    fn get_u32(&self, gaddr: GuestAddr) -> Result<u32, StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            Ok(unsafe { (haddr.as_u64() as *const u32).read() })
        } else {
            Err(StopReason::Segv {
                read: true,
                gaddr: gaddr.into(),
            })
        }
    }

    fn get_u64(&self, gaddr: GuestAddr) -> Result<u64, StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            Ok(unsafe { (haddr.as_u64() as *const u64).read() })
        } else {
            Err(StopReason::Segv {
                read: true,
                gaddr: gaddr.into(),
            })
        }
    }

    fn set_u8(&self, gaddr: GuestAddr, val: u8) -> Result<(), StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            Ok(unsafe { (haddr.as_u64() as *mut u8).write(val) })
        } else {
            Err(StopReason::Segv {
                read: false,
                gaddr: gaddr.into(),
            })
        }
    }

    fn set_u16(&self, gaddr: GuestAddr, val: u16) -> Result<(), StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            Ok(unsafe { (haddr.as_u64() as *mut u16).write(val) })
        } else {
            Err(StopReason::Segv {
                read: false,
                gaddr: gaddr.into(),
            })
        }
    }

    fn set_u32(&self, gaddr: GuestAddr, val: u32) -> Result<(), StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            Ok(unsafe { (haddr.as_u64() as *mut u32).write(val) })
        } else {
            Err(StopReason::Segv {
                read: false,
                gaddr: gaddr.into(),
            })
        }
    }

    fn set_u64(&self, gaddr: GuestAddr, val: u64) -> Result<(), StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            Ok(unsafe { (haddr.as_u64() as *mut u64).write(val) })
        } else {
            Err(StopReason::Segv {
                read: false,
                gaddr: gaddr.into(),
            })
        }
    }

    fn pcrel(&self, imm: i64) -> u64 {
        (self.state.get_pc() as i64 + imm) as u64
    }

    fn gx(&self, idx: u8) -> u64 {
        self.state.get_x(idx)
    }

    fn sx(&mut self, idx: u8, val: u64) {
        self.state.set_x(idx, val)
    }

    fn gf32(&self, idx: u8) -> f32 {
        self.state.get_f32(idx)
    }

    fn sf32(&mut self, idx: u8, val: f32) {
        self.state.set_f32(idx, val)
    }

    fn gf64(&self, idx: u8) -> f64 {
        self.state.get_f64(idx)
    }

    fn sf64(&mut self, idx: u8, val: f64) {
        self.state.set_f64(idx, val)
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

    fn fetch_insn(&self) -> Result<(RvInsn, usize), StopReason> {
        let pc = self.state.get_pc();
        // println!("pc = {:016x}", pc);

        // XXX: this is duplicating code from decoder, ideally decoder will
        // handle all of this
        let lsb_byte = self.get_u8(pc.into())?;
        if lsb_byte & 0b11 == 0b11 {
            // 32-bit
            let insn_word = self.get_u32(pc.into())?;
            Ok((self.decoder.disas_32bit(insn_word), 4))
        } else {
            // 16-bit
            let insn_word = self.get_u16(pc.into())?;
            Ok((self.decoder.disas_16bit(insn_word), 2))
        }
    }

    fn exec_one(&mut self) -> StopReason {
        let (insn, len) = match self.fetch_insn() {
            Ok((insn, len)) => (insn, len),
            Err(e) => return e,
        };
        // println!("decoded {}b: {:?}", len, insn);

        let res = self.interpret_one(&insn, len);

        let new_pc = if let StopReason::ContinueAt(x) = res {
            x
        } else {
            self.state.get_pc() + (len as u64)
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
                self.sx(a.rd, a.imm as u64);
                StopReason::Next
            }
            RvInsn::Auipc(a) => {
                self.sx(a.rd, self.pcrel(a.imm as i64));
                StopReason::Next
            }
            RvInsn::Jal(a) => {
                let pc = self.state.get_pc();
                self.sx(a.rd, pc + insn_len as u64);
                StopReason::ContinueAt((pc as i64 + a.imm as i64) as u64)
            }
            RvInsn::Jalr(a) => {
                let pc = self.state.get_pc();
                self.sx(a.rd, pc + insn_len as u64);
                StopReason::ContinueAt((self.gx(a.rs1) as i64 + a.imm as i64) as u64)
            }
            RvInsn::Beq(a) => {
                let v1 = self.gx(a.rs1);
                let v2 = self.gx(a.rs2);
                if v1 == v2 {
                    StopReason::ContinueAt(self.pcrel(a.imm as i64))
                } else {
                    StopReason::Next
                }
            }
            RvInsn::Bne(a) => {
                // TODO: dedup
                let v1 = self.gx(a.rs1);
                let v2 = self.gx(a.rs2);
                if v1 != v2 {
                    StopReason::ContinueAt(self.pcrel(a.imm as i64))
                } else {
                    StopReason::Next
                }
            }
            RvInsn::Blt(a) => {
                let v1 = self.gx(a.rs1) as i64;
                let v2 = self.gx(a.rs2) as i64;
                if v1 < v2 {
                    StopReason::ContinueAt(self.pcrel(a.imm as i64))
                } else {
                    StopReason::Next
                }
            }
            RvInsn::Bge(a) => {
                // TODO: dedup
                let v1 = self.gx(a.rs1) as i64;
                let v2 = self.gx(a.rs2) as i64;
                if v1 >= v2 {
                    StopReason::ContinueAt(self.pcrel(a.imm as i64))
                } else {
                    StopReason::Next
                }
            }
            RvInsn::Bltu(a) => {
                let v1 = self.gx(a.rs1);
                let v2 = self.gx(a.rs2);
                if v1 < v2 {
                    StopReason::ContinueAt(self.pcrel(a.imm as i64))
                } else {
                    StopReason::Next
                }
            }
            RvInsn::Bgeu(a) => {
                // TODO: dedup
                let v1 = self.gx(a.rs1) as i64;
                let v2 = self.gx(a.rs2) as i64;
                if v1 >= v2 {
                    StopReason::ContinueAt(self.pcrel(a.imm as i64))
                } else {
                    StopReason::Next
                }
            }
            RvInsn::Lb(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                match self.get_u8(addr.into()) {
                    Ok(v) => {
                        self.sx(a.rd, sext_u8(v));
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::Lh(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                match self.get_u16(addr.into()) {
                    Ok(v) => {
                        self.sx(a.rd, sext_u16(v));
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::Lw(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                match self.get_u32(addr.into()) {
                    Ok(v) => {
                        self.sx(a.rd, sext_u32(v));
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::Lbu(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                match self.get_u8(addr.into()) {
                    Ok(v) => {
                        self.sx(a.rd, v as u64);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::Lhu(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                match self.get_u16(addr.into()) {
                    Ok(v) => {
                        self.sx(a.rd, v as u64);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::Sb(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                self.set_u8(addr.into(), self.gx(a.rs2) as u8)
                    .err()
                    .unwrap_or(StopReason::Next)
            }
            RvInsn::Sh(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                self.set_u16(addr.into(), self.gx(a.rs2) as u16)
                    .err()
                    .unwrap_or(StopReason::Next)
            }
            RvInsn::Sw(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                self.set_u32(addr.into(), self.gx(a.rs2) as u32)
                    .err()
                    .unwrap_or(StopReason::Next)
            }
            RvInsn::Addi(a) => {
                let v = self.gx(a.rs1) as i64 + a.imm as i64;
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::Slti(a) => {
                let v = if (self.gx(a.rs1) as i64) < a.imm as i64 {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Sltiu(a) => {
                let v = if self.gx(a.rs1) < a.imm as i64 as u64 {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Xori(a) => {
                let v = self.gx(a.rs1) ^ (a.imm as i64 as u64);
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Ori(a) => {
                let v = self.gx(a.rs1) | (a.imm as i64 as u64);
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Andi(a) => {
                let v = self.gx(a.rs1) & (a.imm as i64 as u64);
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Slli(a) => {
                let v = self.gx(a.rs1) << a.shamt;
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Srli(a) => {
                let v = self.gx(a.rs1) >> a.shamt;
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Srai(a) => {
                let v = self.gx(a.rs1) as i64 >> a.shamt;
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::Add(a) => {
                let v = self.gx(a.rs1) as i64 + self.gx(a.rs2) as i64;
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::Sub(a) => {
                let v = self.gx(a.rs1) as i64 - self.gx(a.rs2) as i64;
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::Sll(a) => {
                let v = self.gx(a.rs1) << (self.gx(a.rs2) & self.shamt_mask);
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Slt(a) => {
                let v = if (self.gx(a.rs1) as i64) < self.gx(a.rs2) as i64 {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Sltu(a) => {
                let v = if self.gx(a.rs1) < self.gx(a.rs2) {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Xor(a) => {
                let v = self.gx(a.rs1) ^ self.gx(a.rs2);
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Srl(a) => {
                let v = self.gx(a.rs1) >> (self.gx(a.rs2) & self.shamt_mask);
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Sra(a) => {
                let v = self.gx(a.rs1) as i64 >> (self.gx(a.rs2) & self.shamt_mask);
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::Or(a) => {
                let v = self.gx(a.rs1) | self.gx(a.rs2);
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::And(a) => {
                let v = self.gx(a.rs1) & self.gx(a.rs2);
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::Fence(a) => {
                // TODO: currently all treated as full fences
                std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
                StopReason::Next
            }
            RvInsn::FenceI(_) => todo!(),
            RvInsn::Lwu(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                match self.get_u32(addr.into()) {
                    Ok(v) => {
                        self.sx(a.rd, v as u64);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::Ld(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                match self.get_u64(addr.into()) {
                    Ok(v) => {
                        self.sx(a.rd, v);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::Sd(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                self.set_u64(addr.into(), self.gx(a.rs2))
                    .err()
                    .unwrap_or(StopReason::Next)
            }
            RvInsn::Addiw(a) => {
                let v = self.gx(a.rs1) as i32 + a.imm as i32;
                self.sx(a.rd, v as i64 as u64);
                StopReason::Next
            }
            RvInsn::Slliw(a) => {
                let v = (self.gx(a.rs1) as u32) << a.shamt;
                self.sx(a.rd, v as i32 as i64 as u64);
                StopReason::Next
            }
            RvInsn::Srliw(a) => {
                let v = self.gx(a.rs1) as u32 >> a.shamt;
                self.sx(a.rd, v as i32 as i64 as u64);
                StopReason::Next
            }
            RvInsn::Sraiw(a) => {
                let v = self.gx(a.rs1) as i32 >> a.shamt;
                self.sx(a.rd, v as i64 as u64);
                StopReason::Next
            }
            RvInsn::Addw(a) => {
                let v = self.gx(a.rs1) as i32 + self.gx(a.rs2) as i32;
                self.sx(a.rd, v as i64 as u64);
                StopReason::Next
            }
            RvInsn::Subw(a) => {
                let v = self.gx(a.rs1) as i32 - self.gx(a.rs2) as i32;
                self.sx(a.rd, v as i64 as u64);
                StopReason::Next
            }
            RvInsn::Sllw(a) => {
                let v = (self.gx(a.rs1) as u32) << (self.gx(a.rs2) & 31);
                self.sx(a.rd, v as i32 as i64 as u64);
                StopReason::Next
            }
            RvInsn::Srlw(a) => {
                let v = self.gx(a.rs1) as u32 >> (self.gx(a.rs2) & 31);
                self.sx(a.rd, v as i32 as i64 as u64);
                StopReason::Next
            }
            RvInsn::Sraw(a) => {
                let v = self.gx(a.rs1) as i32 >> (self.gx(a.rs2) & 31);
                self.sx(a.rd, v as i64 as u64);
                StopReason::Next
            }
            RvInsn::Mul(a) => {
                let v1 = self.gx(a.rs1) as i64;
                let v2 = self.gx(a.rs2) as i64;
                self.sx(a.rd, v1.wrapping_mul(v2) as u64);
                StopReason::Next
            }
            RvInsn::Mulh(a) => {
                let v1 = self.gx(a.rs1) as i64 as i128;
                let v2 = self.gx(a.rs2) as i64 as i128;
                let v = (v1 * v2) >> 64;
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::Mulhsu(a) => {
                let v1 = self.gx(a.rs1) as i64 as i128;
                let v2 = self.gx(a.rs1) as u64 as i128;
                let v = (v1 * v2) >> 64;
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::Mulhu(a) => {
                let v1 = self.gx(a.rs1);
                let v2 = self.gx(a.rs2);
                // let (_, v) = v1.widening_mul(v2);
                let v = (v1 as u128 * v2 as u128) >> 64;
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::Div(a) => {
                let v1 = self.gx(a.rs1) as i64;
                let v2 = self.gx(a.rs2) as i64;
                self.sx(a.rd, v1.wrapping_div(v2) as u64);
                StopReason::Next
            }
            RvInsn::Divu(a) => {
                let v1 = self.gx(a.rs1);
                let v2 = self.gx(a.rs2);
                self.sx(a.rd, v1.wrapping_div(v2));
                StopReason::Next
            }
            RvInsn::Rem(a) => {
                let v1 = self.gx(a.rs1) as i64;
                let v2 = self.gx(a.rs2) as i64;
                self.sx(a.rd, v1.wrapping_rem(v2) as u64);
                StopReason::Next
            }
            RvInsn::Remu(a) => {
                let v1 = self.gx(a.rs1);
                let v2 = self.gx(a.rs2);
                self.sx(a.rd, v1.wrapping_rem(v2));
                StopReason::Next
            }
            RvInsn::Mulw(a) => {
                let v1 = self.gx(a.rs1) as i32;
                let v2 = self.gx(a.rs2) as i32;
                self.sx(a.rd, v1.wrapping_mul(v2) as i64 as u64);
                StopReason::Next
            }
            RvInsn::Divw(a) => {
                let v1 = self.gx(a.rs1) as i32;
                let v2 = self.gx(a.rs2) as i32;
                self.sx(a.rd, v1.wrapping_div(v2) as i64 as u64);
                StopReason::Next
            }
            RvInsn::Divuw(a) => {
                let v1 = self.gx(a.rs1) as u32;
                let v2 = self.gx(a.rs2) as u32;
                self.sx(a.rd, v1.wrapping_div(v2) as u64);
                StopReason::Next
            }
            RvInsn::Remw(a) => {
                let v1 = self.gx(a.rs1) as i32;
                let v2 = self.gx(a.rs2) as i32;
                self.sx(a.rd, v1.wrapping_rem(v2) as i64 as u64);
                StopReason::Next
            }
            RvInsn::Remuw(a) => {
                let v1 = self.gx(a.rs1) as u32;
                let v2 = self.gx(a.rs2) as u32;
                self.sx(a.rd, v1.wrapping_rem(v2) as u64);
                StopReason::Next
            }
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
            RvInsn::Flw(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                match self.get_u32(addr.into()) {
                    Ok(v) => {
                        self.sf32(a.rd, v as f32);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::Fsw(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                self.set_u32(addr.into(), self.gf32(a.rs2) as u32)
                    .err()
                    .unwrap_or(StopReason::Next)
            }
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
            RvInsn::FmvXW(a) => {
                self.sx(a.rd, self.gf32(a.rs1) as i32 as i64 as u64);
                StopReason::Next
            }
            RvInsn::FeqS(_) => todo!(),
            RvInsn::FltS(_) => todo!(),
            RvInsn::FleS(_) => todo!(),
            RvInsn::FclassS(_) => todo!(),
            RvInsn::FcvtSW(_) => todo!(),
            RvInsn::FcvtSWu(_) => todo!(),
            RvInsn::FmvWX(a) => {
                self.sf32(a.rd, self.gx(a.rs1) as u32 as f32);
                StopReason::Next
            }
            RvInsn::FcvtLS(_) => todo!(),
            RvInsn::FcvtLuS(_) => todo!(),
            RvInsn::FcvtSL(_) => todo!(),
            RvInsn::FcvtSLu(_) => todo!(),
            RvInsn::Fld(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                match self.get_u64(addr.into()) {
                    Ok(v) => {
                        self.sf64(a.rd, v as f64);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::Fsd(a) => {
                let addr = (self.gx(a.rs1) as i64 + a.imm as i64) as u64;
                self.set_u64(addr.into(), self.gf64(a.rs2) as u64)
                    .err()
                    .unwrap_or(StopReason::Next)
            }
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
            RvInsn::FmvXD(a) => {
                self.sx(a.rd, self.gf64(a.rs1) as u64);
                StopReason::Next
            }
            RvInsn::FcvtDL(_) => todo!(),
            RvInsn::FcvtDLu(_) => todo!(),
            RvInsn::FmvDX(a) => {
                self.sf64(a.rd, self.gx(a.rs1) as f64);
                StopReason::Next
            }
        }
    }
}
