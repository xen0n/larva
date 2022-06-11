use super::args::*;

// funct7, rs2, rs1, funct3, rd
pub(super) struct RTypeSlots(u8, u8, u8, u8, u8);

impl RTypeSlots {
    pub(super) fn funct7(&self) -> u8 {
        self.0
    }

    pub(super) fn funct3(&self) -> u8 {
        self.3
    }

    pub(super) fn rs2(&self) -> u8 {
        self.1
    }

    pub(super) fn amo_funct(&self) -> u8 {
        self.funct7() >> 2
    }

    pub(super) fn amo_aq(&self) -> bool {
        self.funct7() & 0b10 != 0
    }

    pub(super) fn amo_rl(&self) -> bool {
        self.funct7() & 0b01 != 0
    }
}

impl From<RTypeSlots> for RTypeArgs {
    fn from(x: RTypeSlots) -> Self {
        Self {
            rd: x.4,
            rs1: x.2,
            rs2: x.1,
        }
    }
}

impl From<RTypeSlots> for AmoArgs {
    fn from(x: RTypeSlots) -> Self {
        Self {
            aq: x.amo_aq(),
            rl: x.amo_rl(),
            rd: x.4,
            rs1: x.2,
            rs2: x.1,
        }
    }
}

impl From<RTypeSlots> for AmoLrArgs {
    fn from(x: RTypeSlots) -> Self {
        Self {
            aq: x.amo_aq(),
            rl: x.amo_rl(),
            rd: x.4,
            rs1: x.2,
        }
    }
}

impl From<RTypeSlots> for RFTypeArgs {
    fn from(x: RTypeSlots) -> Self {
        Self {
            rm: x.funct3().into(),
            rd: x.4,
            rs1: x.2,
            rs2: x.1,
        }
    }
}

impl From<RTypeSlots> for R2TypeArgs {
    fn from(x: RTypeSlots) -> Self {
        Self { rd: x.4, rs1: x.2 }
    }
}

impl From<RTypeSlots> for R2FTypeArgs {
    fn from(x: RTypeSlots) -> Self {
        Self {
            rm: x.funct3().into(),
            rd: x.4,
            rs1: x.2,
        }
    }
}

// I-type: imm, rs1, funct3, rd
// S-type & B-type: imm, rs2, rs1, funct3
pub(super) struct ISBTypeSlots(i32, u8, u8, u8);

impl ISBTypeSlots {
    pub(super) fn i_funct3(&self) -> u8 {
        self.2
    }

    pub(super) fn sb_funct3(&self) -> u8 {
        self.3
    }

    pub(super) fn rv32_shift_funct(&self) -> u8 {
        (self.0 >> 5) as u8
    }

    pub(super) fn rv64_shift_funct(&self) -> u8 {
        (self.0 >> 6) as u8
    }

    pub(super) fn fence_fm(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub(super) fn fence_pred(&self) -> FenceSet {
        (((self.0 >> 4) & 0b1111) as u8).into()
    }

    pub(super) fn fence_succ(&self) -> FenceSet {
        ((self.0 & 0b1111) as u8).into()
    }
}

impl From<ISBTypeSlots> for ITypeArgs {
    fn from(x: ISBTypeSlots) -> Self {
        Self {
            rd: x.3,
            rs1: x.1,
            imm: x.0,
        }
    }
}

impl From<ISBTypeSlots> for SBTypeArgs {
    fn from(x: ISBTypeSlots) -> Self {
        Self {
            rs1: x.2,
            rs2: x.1,
            imm: x.0,
        }
    }
}

impl From<ISBTypeSlots> for ShiftArgs {
    fn from(x: ISBTypeSlots) -> Self {
        Self {
            rd: x.3,
            rs1: x.1,
            shamt: (x.0 & 0xff) as u8,
        }
    }
}

impl From<ISBTypeSlots> for FenceArgs {
    fn from(x: ISBTypeSlots) -> Self {
        Self {
            fm: x.fence_fm(),
            pred: x.fence_pred(),
            succ: x.fence_succ(),
        }
    }
}

// U-type & J-type: imm, rd
pub(super) struct UJTypeSlots(i32, u8);

impl From<UJTypeSlots> for UJTypeArgs {
    fn from(x: UJTypeSlots) -> Self {
        Self { rd: x.1, imm: x.0 }
    }
}

// rs3, funct2, rs2, rs1, funct3, rd
pub(super) struct R4TypeSlots(u8, u8, u8, u8, u8, u8);

impl R4TypeSlots {
    pub(super) fn funct2(&self) -> u8 {
        self.1
    }
}

impl From<R4TypeSlots> for R4TypeArgs {
    fn from(x: R4TypeSlots) -> Self {
        Self {
            rm: x.4.into(),
            rd: x.5,
            rs1: x.3,
            rs2: x.2,
            rs3: x.0,
        }
    }
}

fn simm_from_uimm(uimm: u32, width: u8) -> i32 {
    // example with width = 6, uimm = 0b100111:
    //
    // a = 1 << width = 0b1000000
    // b = a >> 1 = 0b100000 (sign bit)
    // uimm >= b, abs(simm) = a - uimm
    let a = 1 << width;
    let b = a >> 1;
    if uimm < b {
        uimm as i32
    } else {
        -((a - uimm) as i32)
    }
}

pub(super) fn disas_r(insn: u32) -> RTypeSlots {
    let funct7 = (insn >> 25) as u8;
    let rs2 = ((insn >> 20) & 0b11111) as u8;
    let rs1 = ((insn >> 15) & 0b11111) as u8;
    let funct3 = ((insn >> 12) & 0b111) as u8;
    let rd = ((insn >> 7) & 0b11111) as u8;
    RTypeSlots(funct7, rs2, rs1, funct3, rd)
}

pub(super) fn disas_i(insn: u32) -> ISBTypeSlots {
    let imm = simm_from_uimm(insn >> 20, 12);
    let rs1 = ((insn >> 15) & 0b11111) as u8;
    let funct3 = ((insn >> 12) & 0b111) as u8;
    let rd = ((insn >> 7) & 0b11111) as u8;
    ISBTypeSlots(imm, rs1, funct3, rd)
}

pub(super) fn disas_s(insn: u32) -> ISBTypeSlots {
    let imm = (insn >> (25 - 5)) | ((insn >> 7) & 0b11111);
    let imm = simm_from_uimm(imm, 12);
    let rs2 = ((insn >> 20) & 0b11111) as u8;
    let rs1 = ((insn >> 15) & 0b11111) as u8;
    let funct3 = ((insn >> 12) & 0b111) as u8;
    ISBTypeSlots(imm, rs2, rs1, funct3)
}

pub(super) fn disas_b(insn: u32) -> ISBTypeSlots {
    let imm = {
        let a = insn >> 31;
        let b = (insn >> 7) & 1;
        let c = (insn >> 25) & 0b111111;
        let d = (insn >> 8) & 0b1111;
        (a << 12) | (b << 11) | (c << 5) | (d << 1)
    };
    let imm = simm_from_uimm(imm, 13);
    let rs2 = ((insn >> 20) & 0b11111) as u8;
    let rs1 = ((insn >> 15) & 0b11111) as u8;
    let funct3 = ((insn >> 12) & 0b111) as u8;
    ISBTypeSlots(imm, rs2, rs1, funct3)
}

pub(super) fn disas_u(insn: u32) -> UJTypeSlots {
    let imm = simm_from_uimm(insn >> 12, 20) << 12;
    let rd = ((insn >> 7) & 0b11111) as u8;
    UJTypeSlots(imm, rd)
}

pub(super) fn disas_j(insn: u32) -> UJTypeSlots {
    let imm = {
        let a = insn >> 31;
        let b = (insn >> 12) & 0b11111111;
        let c = (insn >> 20) & 1;
        let d = (insn >> 21) & 0b1111111111;
        (a << 20) | (b << 12) | (c << 11) | (d << 1)
    };
    let imm = simm_from_uimm(imm, 21);
    let rd = ((insn >> 7) & 0b11111) as u8;
    UJTypeSlots(imm, rd)
}

pub(super) fn disas_r4(insn: u32) -> R4TypeSlots {
    let rs3 = (insn >> 27) as u8;
    let funct2 = ((insn >> 25) & 0b11) as u8;
    let rs2 = ((insn >> 20) & 0b11111) as u8;
    let rs1 = ((insn >> 15) & 0b11111) as u8;
    let funct3 = ((insn >> 12) & 0b111) as u8;
    let rd = ((insn >> 7) & 0b11111) as u8;
    R4TypeSlots(rs3, funct2, rs2, rs1, funct3, rd)
}
