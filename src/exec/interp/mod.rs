use super::mem::{GuestAddr, GuestMmu, MemPerms};
use super::{RvIsaState, StopReason};
use crate::rv::{RvDecoder, RvInsn};

mod syscall;

pub struct RvInterpreterExecutor<'a> {
    debug: bool,
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

/// Classify a float value according to RISC-V FCLASS format.
/// Returns a bit mask where bit N is set if the value matches class N.
fn classify_float(val: f32) -> u64 {
    let bits = val.to_bits();
    let sign = bits >> 31;
    let exp = (bits >> 23) & 0xFF;
    let mant = bits & 0x7FFFFF;

    if exp == 0 && mant == 0 {
        // Zero
        if sign == 0 {
            1 << 4 // +0
        } else {
            1 << 3 // -0
        }
    } else if exp == 0 {
        // Subnormal
        if sign == 0 {
            1 << 5 // +subnormal
        } else {
            1 << 2 // -subnormal
        }
    } else if exp == 0xFF {
        if mant == 0 {
            // Infinity
            if sign == 0 {
                1 << 7 // +inf
            } else {
                1 << 0 // -inf
            }
        } else {
            // NaN
            if mant & 0x400000 != 0 {
                1 << 9 // quiet NaN
            } else {
                1 << 8 // signaling NaN
            }
        }
    } else {
        // Normal
        if sign == 0 {
            1 << 6 // +normal
        } else {
            1 << 1 // -normal
        }
    }
}

/// Classify a double value according to RISC-V FCLASS format.
/// Returns a bit mask where bit N is set if the value matches class N.
fn classify_double(val: f64) -> u64 {
    let bits = val.to_bits();
    let sign = bits >> 63;
    let exp = (bits >> 52) & 0x7FF;
    let mant = bits & 0xF_FFFF_FFFF_FFFFu64;

    if exp == 0 && mant == 0 {
        // Zero
        if sign == 0 {
            1 << 4 // +0
        } else {
            1 << 3 // -0
        }
    } else if exp == 0 {
        // Subnormal
        if sign == 0 {
            1 << 5 // +subnormal
        } else {
            1 << 2 // -subnormal
        }
    } else if exp == 0x7FF {
        if mant == 0 {
            // Infinity
            if sign == 0 {
                1 << 7 // +inf
            } else {
                1 << 0 // -inf
            }
        } else {
            // NaN
            if mant & 0x8_0000_0000_0000u64 != 0 {
                1 << 9 // quiet NaN
            } else {
                1 << 8 // signaling NaN
            }
        }
    } else {
        // Normal
        if sign == 0 {
            1 << 6 // +normal
        } else {
            1 << 1 // -normal
        }
    }
}

impl<'a> RvInterpreterExecutor<'a> {
    pub fn new(xlen: usize, state: &'a mut RvIsaState, mmu: &'a mut GuestMmu) -> Self {
        Self {
            debug: false,
            shamt_mask: (xlen - 1) as u64,
            state,
            mmu,
            decoder: RvDecoder::new(xlen),
        }
    }

    /// Perform an atomic read-modify-write operation on a 32-bit memory location.
    /// The `op` closure receives the old value and returns the new value.
    fn amo_32<F>(&mut self, addr: GuestAddr, rd: u8, op: F) -> StopReason
    where
        F: FnOnce(u32) -> u32,
    {
        match self.get_u32(addr) {
            Ok(old) => {
                let new_val = op(old);
                match self.set_u32(addr, new_val) {
                    Ok(_) => {
                        self.sx(rd, sext_u32(old));
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            Err(e) => e,
        }
    }

    /// Perform an atomic read-modify-write operation on a 64-bit memory location.
    /// The `op` closure receives the old value and returns the new value.
    fn amo_64<F>(&mut self, addr: GuestAddr, rd: u8, op: F) -> StopReason
    where
        F: FnOnce(u64) -> u64,
    {
        match self.get_u64(addr) {
            Ok(old) => {
                let new_val = op(old);
                match self.set_u64(addr, new_val) {
                    Ok(_) => {
                        self.sx(rd, old);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            Err(e) => e,
        }
    }

    pub fn debug(&mut self, val: bool) {
        self.debug = val;
    }

    pub fn stack(&mut self, len: usize) -> ::std::io::Result<()> {
        let stack_block = self.mmu.mmap(len, MemPerms::rw(), true)?;
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
            unsafe { (haddr.as_u64() as *mut u8).write(val) };
            Ok(())
        } else {
            Err(StopReason::Segv {
                read: false,
                gaddr: gaddr.into(),
            })
        }
    }

    fn set_u16(&self, gaddr: GuestAddr, val: u16) -> Result<(), StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            unsafe { (haddr.as_u64() as *mut u16).write(val) };
            Ok(())
        } else {
            Err(StopReason::Segv {
                read: false,
                gaddr: gaddr.into(),
            })
        }
    }

    fn set_u32(&self, gaddr: GuestAddr, val: u32) -> Result<(), StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            unsafe { (haddr.as_u64() as *mut u32).write(val) };
            Ok(())
        } else {
            Err(StopReason::Segv {
                read: false,
                gaddr: gaddr.into(),
            })
        }
    }

    fn set_u64(&self, gaddr: GuestAddr, val: u64) -> Result<(), StopReason> {
        if let Some(haddr) = self.mmu.g2h(gaddr) {
            unsafe { (haddr.as_u64() as *mut u64).write(val) };
            Ok(())
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

        // Open trace log file if LARVA_TRACE is set
        let mut trace_file: Option<std::fs::File> = std::env::var("LARVA_TRACE")
            .ok()
            .and_then(|path| {
                eprintln!("Tracing execution to: {path}");
                std::fs::File::create(&path).ok()
            });

        loop {
            let pc = self.state.get_pc();
            let x = self.exec_one();

            // Log to trace file if enabled
            if let Some(file) = &mut trace_file {
                use std::io::Write;
                let _ = writeln!(file, "pc={:016x}", pc);
            }

            match x {
                StopReason::Next | StopReason::ContinueAt(_) => {}
                _ => return Some(x),
            }
        }
    }

    fn fetch_insn(&self) -> Result<(RvInsn, usize), StopReason> {
        let pc = self.state.get_pc();
        if self.debug {
            println!("pc = {pc:016x}");
        }

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
        let pc = self.state.get_pc();
        let (insn, len) = match self.fetch_insn() {
            Ok((insn, len)) => (insn, len),
            Err(e) => return e,
        };
        if self.debug {
            println!("decoded {len}b: {insn:?}");
        }

        // Track when x8 or x9 changes
        static mut LAST_X8: u64 = 0;
        static mut LAST_X9: u64 = 0;
        let x8_current = self.gx(8);
        let x9_current = self.gx(9);
        unsafe {
            let last_x8 = LAST_X8;
            let last_x9 = LAST_X9;
            if x8_current != last_x8 {
                println!("  [x8-change] pc={pc:016x} x8={x8_current:016x} (was {:016x})", last_x8);
                LAST_X8 = x8_current;
            }
            if x9_current != last_x9 {
                println!("  [x9-change] pc={pc:016x} x9={x9_current:016x} (was {:016x})", last_x9);
                LAST_X9 = x9_current;
            }
        }

        // Detailed trace for crash site regions
        if (pc >= 0x1216f0 && pc <= 0x121710) || (pc >= 0xe4f40 && pc <= 0xe4f60) {
            let x2 = self.gx(2);
            let x8 = self.gx(8);
            let x9 = self.gx(9);
            let x10 = self.gx(10);
            let x14 = self.gx(14);
            let x15 = self.gx(15);
            println!("  [regs] x2(sp)={x2:016x} x8={x8:016x} x9={x9:016x} x10={x10:016x} x14={x14:016x} x15={x15:016x}");
            
            // Read stack values
            if let Ok(val) = self.get_u64((x2 + 24).into()) {
                println!("  [stack] sp+24={val:016x} (saved x8)");
            }
            if let Ok(val) = self.get_u64((x2 + 40).into()) {
                println!("  [stack] sp+40={val:016x} (saved ra)");
            }
            // Check x8-112 area
            if x8 >= 112 {
                if let Ok(val) = self.get_u64((x8 - 112).into()) {
                    println!("  [data] x8-112={val:016x}");
                }
            }
        }

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
            RvInsn::Fence(_) => {
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
                let v = self.gx(a.rs1) as i32 + a.imm;
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
                let v2 = self.gx(a.rs1) as i128;
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
            // RV32A/RV64A atomic operations
            //
            // These are implemented as non-atomic read-modify-write sequences
            // because the interpreter is single-threaded. In a multi-threaded
            // context, these would need proper atomic synchronization.
            //
            // LR/SC (Load-Reserved/Store-Conditional) pairs are not tracked;
            // SC always succeeds (returns 0) in this implementation.
            RvInsn::LrW(a) => {
                let addr = self.gx(a.rs1);
                match self.get_u32(addr.into()) {
                    Ok(v) => {
                        self.sx(a.rd, sext_u32(v));
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::ScW(a) => {
                let addr = self.gx(a.rs1);
                let val = self.gx(a.rs2) as u32;
                match self.set_u32(addr.into(), val) {
                    Ok(_) => {
                        self.sx(a.rd, 0);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            // AMO operations: atomic read-modify-write
            // Each operation follows the pattern: load, compute, store, return old value
            RvInsn::AmoSwapW(a) => {
                let addr = self.gx(a.rs1).into();
                let new_val = self.gx(a.rs2) as u32;
                self.amo_32(addr, a.rd, |_old| new_val)
            }
            RvInsn::AmoAddW(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as u32;
                self.amo_32(addr, a.rd, |old| old.wrapping_add(rs2))
            }
            RvInsn::AmoXorW(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as u32;
                self.amo_32(addr, a.rd, |old| old ^ rs2)
            }
            RvInsn::AmoAndW(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as u32;
                self.amo_32(addr, a.rd, |old| old & rs2)
            }
            RvInsn::AmoOrW(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as u32;
                self.amo_32(addr, a.rd, |old| old | rs2)
            }
            RvInsn::AmoMinW(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as i32;
                self.amo_32(addr, a.rd, |old| (old as i32).min(rs2) as u32)
            }
            RvInsn::AmoMaxW(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as i32;
                self.amo_32(addr, a.rd, |old| (old as i32).max(rs2) as u32)
            }
            RvInsn::AmoMinuW(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as u32;
                self.amo_32(addr, a.rd, |old| old.min(rs2))
            }
            RvInsn::AmoMaxuW(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as u32;
                self.amo_32(addr, a.rd, |old| old.max(rs2))
            }
            RvInsn::LrD(a) => {
                let addr = self.gx(a.rs1);
                match self.get_u64(addr.into()) {
                    Ok(v) => {
                        self.sx(a.rd, v);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::ScD(a) => {
                let addr = self.gx(a.rs1);
                let val = self.gx(a.rs2);
                match self.set_u64(addr.into(), val) {
                    Ok(_) => {
                        self.sx(a.rd, 0);
                        StopReason::Next
                    }
                    Err(e) => e,
                }
            }
            RvInsn::AmoSwapD(a) => {
                let addr = self.gx(a.rs1).into();
                let new_val = self.gx(a.rs2);
                self.amo_64(addr, a.rd, |_old| new_val)
            }
            RvInsn::AmoAddD(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2);
                self.amo_64(addr, a.rd, |old| old.wrapping_add(rs2))
            }
            RvInsn::AmoXorD(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2);
                self.amo_64(addr, a.rd, |old| old ^ rs2)
            }
            RvInsn::AmoAndD(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2);
                self.amo_64(addr, a.rd, |old| old & rs2)
            }
            RvInsn::AmoOrD(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2);
                self.amo_64(addr, a.rd, |old| old | rs2)
            }
            RvInsn::AmoMinD(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as i64;
                self.amo_64(addr, a.rd, |old| (old as i64).min(rs2) as u64)
            }
            RvInsn::AmoMaxD(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2) as i64;
                self.amo_64(addr, a.rd, |old| (old as i64).max(rs2) as u64)
            }
            RvInsn::AmoMinuD(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2);
                self.amo_64(addr, a.rd, |old| old.min(rs2))
            }
            RvInsn::AmoMaxuD(a) => {
                let addr = self.gx(a.rs1).into();
                let rs2 = self.gx(a.rs2);
                self.amo_64(addr, a.rd, |old| old.max(rs2))
            }
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
            // RVF floating-point operations
            // Note: These assume the host has a conformant IEEE 754-2008 implementation
            // with nan2008 NaN signaling/quiet semantics. Legacy MIPS implementations
            // may behave differently.
            RvInsn::FmaddS(a) => {
                let rs1 = self.gf32(a.rs1);
                let rs2 = self.gf32(a.rs2);
                let rs3 = self.gf32(a.rs3);
                self.sf32(a.rd, rs1.mul_add(rs2, rs3));
                StopReason::Next
            }
            RvInsn::FmsubS(a) => {
                let rs1 = self.gf32(a.rs1);
                let rs2 = self.gf32(a.rs2);
                let rs3 = self.gf32(a.rs3);
                self.sf32(a.rd, rs1.mul_add(rs2, -rs3));
                StopReason::Next
            }
            RvInsn::FnmsubS(a) => {
                let rs1 = self.gf32(a.rs1);
                let rs2 = self.gf32(a.rs2);
                let rs3 = self.gf32(a.rs3);
                self.sf32(a.rd, -(rs1.mul_add(rs2, -rs3)));
                StopReason::Next
            }
            RvInsn::FnmaddS(a) => {
                let rs1 = self.gf32(a.rs1);
                let rs2 = self.gf32(a.rs2);
                let rs3 = self.gf32(a.rs3);
                self.sf32(a.rd, -(rs1.mul_add(rs2, rs3)));
                StopReason::Next
            }
            RvInsn::FaddS(a) => {
                let v = self.gf32(a.rs1) + self.gf32(a.rs2);
                self.sf32(a.rd, v);
                StopReason::Next
            }
            RvInsn::FsubS(a) => {
                let v = self.gf32(a.rs1) - self.gf32(a.rs2);
                self.sf32(a.rd, v);
                StopReason::Next
            }
            RvInsn::FmulS(a) => {
                let v = self.gf32(a.rs1) * self.gf32(a.rs2);
                self.sf32(a.rd, v);
                StopReason::Next
            }
            RvInsn::FdivS(a) => {
                let v = self.gf32(a.rs1) / self.gf32(a.rs2);
                self.sf32(a.rd, v);
                StopReason::Next
            }
            RvInsn::FsqrtS(a) => {
                let v = self.gf32(a.rs1).sqrt();
                self.sf32(a.rd, v);
                StopReason::Next
            }
            RvInsn::FsgnjS(a) => {
                let rs1 = self.gf32(a.rs1);
                let rs2 = self.gf32(a.rs2);
                let sign = rs2.to_bits() & 0x8000_0000;
                let val = f32::from_bits((rs1.to_bits() & 0x7FFF_FFFF) | sign);
                self.sf32(a.rd, val);
                StopReason::Next
            }
            RvInsn::FsgnjnS(a) => {
                let rs1 = self.gf32(a.rs1);
                let rs2 = self.gf32(a.rs2);
                let sign = (rs2.to_bits() & 0x8000_0000) ^ 0x8000_0000;
                let val = f32::from_bits((rs1.to_bits() & 0x7FFF_FFFF) | sign);
                self.sf32(a.rd, val);
                StopReason::Next
            }
            RvInsn::FsgnjxS(a) => {
                let rs1 = self.gf32(a.rs1);
                let rs2 = self.gf32(a.rs2);
                let sign = (rs1.to_bits() & 0x8000_0000) ^ (rs2.to_bits() & 0x8000_0000);
                let val = f32::from_bits((rs1.to_bits() & 0x7FFF_FFFF) | sign);
                self.sf32(a.rd, val);
                StopReason::Next
            }
            RvInsn::FminS(a) => {
                let rs1 = self.gf32(a.rs1);
                let rs2 = self.gf32(a.rs2);
                self.sf32(a.rd, rs1.min(rs2));
                StopReason::Next
            }
            RvInsn::FmaxS(a) => {
                let rs1 = self.gf32(a.rs1);
                let rs2 = self.gf32(a.rs2);
                self.sf32(a.rd, rs1.max(rs2));
                StopReason::Next
            }
            RvInsn::FcvtWS(a) => {
                let v = self.gf32(a.rs1) as i32 as i64 as u64;
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FcvtWuS(a) => {
                let v = self.gf32(a.rs1) as u32 as u64;
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FmvXW(a) => {
                self.sx(a.rd, self.gf32(a.rs1) as i32 as i64 as u64);
                StopReason::Next
            }
            RvInsn::FeqS(a) => {
                let v = if self.gf32(a.rs1) == self.gf32(a.rs2) {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FltS(a) => {
                let v = if self.gf32(a.rs1) < self.gf32(a.rs2) {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FleS(a) => {
                let v = if self.gf32(a.rs1) <= self.gf32(a.rs2) {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FclassS(a) => {
                let val = self.gf32(a.rs1);
                let class = classify_float(val);
                self.sx(a.rd, class);
                StopReason::Next
            }
            RvInsn::FcvtSW(a) => {
                let v = self.gx(a.rs1) as i32 as f32;
                self.sf32(a.rd, v);
                StopReason::Next
            }
            RvInsn::FcvtSWu(a) => {
                let v = self.gx(a.rs1) as u32 as f32;
                self.sf32(a.rd, v);
                StopReason::Next
            }
            RvInsn::FmvWX(a) => {
                self.sf32(a.rd, self.gx(a.rs1) as u32 as f32);
                StopReason::Next
            }
            RvInsn::FcvtLS(a) => {
                let v = self.gf32(a.rs1) as i64;
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::FcvtLuS(a) => {
                let v = self.gf32(a.rs1) as u64;
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FcvtSL(a) => {
                let v = self.gx(a.rs1) as i64 as f32;
                self.sf32(a.rd, v);
                StopReason::Next
            }
            RvInsn::FcvtSLu(a) => {
                let v = self.gx(a.rs1) as f32;
                self.sf32(a.rd, v);
                StopReason::Next
            }
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
            // RVD double-precision floating-point operations
            // Note: Like RVF, these assume IEEE 754-2008 with nan2008 semantics
            RvInsn::FmaddD(a) => {
                let rs1 = self.gf64(a.rs1);
                let rs2 = self.gf64(a.rs2);
                let rs3 = self.gf64(a.rs3);
                self.sf64(a.rd, rs1.mul_add(rs2, rs3));
                StopReason::Next
            }
            RvInsn::FmsubD(a) => {
                let rs1 = self.gf64(a.rs1);
                let rs2 = self.gf64(a.rs2);
                let rs3 = self.gf64(a.rs3);
                self.sf64(a.rd, rs1.mul_add(rs2, -rs3));
                StopReason::Next
            }
            RvInsn::FnmsubD(a) => {
                let rs1 = self.gf64(a.rs1);
                let rs2 = self.gf64(a.rs2);
                let rs3 = self.gf64(a.rs3);
                self.sf64(a.rd, -(rs1.mul_add(rs2, -rs3)));
                StopReason::Next
            }
            RvInsn::FnmaddD(a) => {
                let rs1 = self.gf64(a.rs1);
                let rs2 = self.gf64(a.rs2);
                let rs3 = self.gf64(a.rs3);
                self.sf64(a.rd, -(rs1.mul_add(rs2, rs3)));
                StopReason::Next
            }
            RvInsn::FaddD(a) => {
                let v = self.gf64(a.rs1) + self.gf64(a.rs2);
                self.sf64(a.rd, v);
                StopReason::Next
            }
            RvInsn::FsubD(a) => {
                let v = self.gf64(a.rs1) - self.gf64(a.rs2);
                self.sf64(a.rd, v);
                StopReason::Next
            }
            RvInsn::FmulD(a) => {
                let v = self.gf64(a.rs1) * self.gf64(a.rs2);
                self.sf64(a.rd, v);
                StopReason::Next
            }
            RvInsn::FdivD(a) => {
                let v = self.gf64(a.rs1) / self.gf64(a.rs2);
                self.sf64(a.rd, v);
                StopReason::Next
            }
            RvInsn::FsqrtD(a) => {
                let v = self.gf64(a.rs1).sqrt();
                self.sf64(a.rd, v);
                StopReason::Next
            }
            RvInsn::FsgnjD(a) => {
                let rs1 = self.gf64(a.rs1);
                let rs2 = self.gf64(a.rs2);
                let sign = rs2.to_bits() & 0x8000_0000_0000_0000u64;
                let val = f64::from_bits((rs1.to_bits() & 0x7FFF_FFFF_FFFF_FFFFu64) | sign);
                self.sf64(a.rd, val);
                StopReason::Next
            }
            RvInsn::FsgnjnD(a) => {
                let rs1 = self.gf64(a.rs1);
                let rs2 = self.gf64(a.rs2);
                let sign = (rs2.to_bits() & 0x8000_0000_0000_0000u64) ^ 0x8000_0000_0000_0000u64;
                let val = f64::from_bits((rs1.to_bits() & 0x7FFF_FFFF_FFFF_FFFFu64) | sign);
                self.sf64(a.rd, val);
                StopReason::Next
            }
            RvInsn::FsgnjxD(a) => {
                let rs1 = self.gf64(a.rs1);
                let rs2 = self.gf64(a.rs2);
                let sign = (rs1.to_bits() & 0x8000_0000_0000_0000u64)
                    ^ (rs2.to_bits() & 0x8000_0000_0000_0000u64);
                let val = f64::from_bits((rs1.to_bits() & 0x7FFF_FFFF_FFFF_FFFFu64) | sign);
                self.sf64(a.rd, val);
                StopReason::Next
            }
            RvInsn::FminD(a) => {
                let rs1 = self.gf64(a.rs1);
                let rs2 = self.gf64(a.rs2);
                self.sf64(a.rd, rs1.min(rs2));
                StopReason::Next
            }
            RvInsn::FmaxD(a) => {
                let rs1 = self.gf64(a.rs1);
                let rs2 = self.gf64(a.rs2);
                self.sf64(a.rd, rs1.max(rs2));
                StopReason::Next
            }
            RvInsn::FcvtSD(a) => {
                let v = self.gf64(a.rs1) as f32;
                self.sf32(a.rd, v);
                StopReason::Next
            }
            RvInsn::FcvtDS(a) => {
                let v = self.gf32(a.rs1) as f64;
                self.sf64(a.rd, v);
                StopReason::Next
            }
            RvInsn::FeqD(a) => {
                let v = if self.gf64(a.rs1) == self.gf64(a.rs2) {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FltD(a) => {
                let v = if self.gf64(a.rs1) < self.gf64(a.rs2) {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FleD(a) => {
                let v = if self.gf64(a.rs1) <= self.gf64(a.rs2) {
                    1
                } else {
                    0
                };
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FclassD(a) => {
                let val = self.gf64(a.rs1);
                let class = classify_double(val);
                self.sx(a.rd, class);
                StopReason::Next
            }
            RvInsn::FcvtWD(a) => {
                let v = self.gf64(a.rs1) as i32 as i64 as u64;
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FcvtWuD(a) => {
                let v = self.gf64(a.rs1) as u32 as u64;
                self.sx(a.rd, v);
                StopReason::Next
            }
            RvInsn::FcvtDW(a) => {
                let v = self.gx(a.rs1) as i32 as f64;
                self.sf64(a.rd, v);
                StopReason::Next
            }
            RvInsn::FcvtDWu(a) => {
                let v = self.gx(a.rs1) as u32 as f64;
                self.sf64(a.rd, v);
                StopReason::Next
            }
            RvInsn::FcvtLD(a) => {
                let v = self.gf64(a.rs1) as i64;
                self.sx(a.rd, v as u64);
                StopReason::Next
            }
            RvInsn::FcvtLuD(a) => {
                let v = self.gf64(a.rs1) as u64;
                self.sx(a.rd, v);
                StopReason::Next
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_float() {
        // Zero
        assert_eq!(classify_float(0.0), 1 << 4); // +0
        assert_eq!(classify_float(-0.0), 1 << 3); // -0

        // Infinity
        assert_eq!(classify_float(f32::INFINITY), 1 << 7); // +inf
        assert_eq!(classify_float(f32::NEG_INFINITY), 1 << 0); // -inf

        // Normal numbers
        assert_eq!(classify_float(1.0), 1 << 6); // +normal
        assert_eq!(classify_float(-1.0), 1 << 1); // -normal

        // Subnormal
        let subnormal = f32::from_bits(0x00000001);
        assert_eq!(classify_float(subnormal), 1 << 5); // +subnormal
        assert_eq!(classify_float(-subnormal), 1 << 2); // -subnormal

        // NaN - assumes nan2008 semantics (Loongson 3A4000+ compatible)
        // Legacy MIPS implementations may differ in NaN signaling behavior
        assert_eq!(classify_float(f32::NAN), 1 << 9); // quiet NaN (most NaNs are quiet)
    }

    #[test]
    fn test_classify_double() {
        // Zero
        assert_eq!(classify_double(0.0), 1 << 4); // +0
        assert_eq!(classify_double(-0.0), 1 << 3); // -0

        // Infinity
        assert_eq!(classify_double(f64::INFINITY), 1 << 7); // +inf
        assert_eq!(classify_double(f64::NEG_INFINITY), 1 << 0); // -inf

        // Normal numbers
        assert_eq!(classify_double(1.0), 1 << 6); // +normal
        assert_eq!(classify_double(-1.0), 1 << 1); // -normal

        // Subnormal
        let subnormal = f64::from_bits(0x0000000000000001u64);
        assert_eq!(classify_double(subnormal), 1 << 5); // +subnormal
        assert_eq!(classify_double(-subnormal), 1 << 2); // -subnormal

        // NaN - assumes nan2008 semantics
        assert_eq!(classify_double(f64::NAN), 1 << 9); // quiet NaN
    }
}
