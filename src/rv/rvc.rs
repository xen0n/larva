use super::disas_helper::simm_from_uimm;
use super::{ITypeArgs, RTypeArgs, RvInsn, SBTypeArgs, ShiftArgs, UJTypeArgs};

#[derive(PartialEq, Debug)]
pub(super) enum RvCInsn {
    Invalid(u16),
    Addi4spn { rd: u8, imm: i32 },
    Fld { rd: u8, rs1: u8, imm: i32 },
    Lw { rd: u8, rs1: u8, imm: i32 },
    Flw { rd: u8, rs1: u8, imm: i32 },
    Ld { rd: u8, rs1: u8, imm: i32 },
    Fsd { rs1: u8, rs2: u8, imm: i32 },
    Sw { rs1: u8, rs2: u8, imm: i32 },
    Fsw { rs1: u8, rs2: u8, imm: i32 },
    Sd { rs1: u8, rs2: u8, imm: i32 },
    Addi { rd: u8, imm: i32 },
    Jal { imm: i32 },
    Addiw { rd: u8, imm: i32 },
    Li { rd: u8, imm: i32 },
    Lui { rd: u8, imm: i32 },
    Addi16sp { imm: i32 },
    Srli { rd: u8, imm: u8 },
    Srai { rd: u8, imm: u8 },
    Andi { rd: u8, imm: i32 },
    Sub { rd: u8, rs2: u8 },
    Xor { rd: u8, rs2: u8 },
    Or { rd: u8, rs2: u8 },
    And { rd: u8, rs2: u8 },
    Subw { rd: u8, rs2: u8 },
    Addw { rd: u8, rs2: u8 },
    J { imm: i32 },
    Beqz { rs1: u8, imm: i32 },
    Bnez { rs1: u8, imm: i32 },
    Slli,
    Fldsp,
    Lwsp,
    Flwsp,
    Ldsp,
    Jr,
    Jalr,
    Mv,
    Add,
    Fsdsp,
    Swsp,
    Fswsp,
    Sdsp,
}

impl From<RvCInsn> for RvInsn {
    fn from(insn: RvCInsn) -> Self {
        match insn {
            RvCInsn::Invalid(x) => Self::Invalid(x as u32),
            RvCInsn::Addi4spn { rd, imm } => Self::Addi(ITypeArgs { rd, rs1: 2, imm }),
            RvCInsn::Fld { rd, rs1, imm } => Self::Fld(ITypeArgs { rd, rs1, imm }),
            RvCInsn::Lw { rd, rs1, imm } => Self::Lw(ITypeArgs { rd, rs1, imm }),
            RvCInsn::Flw { rd, rs1, imm } => Self::Flw(ITypeArgs { rd, rs1, imm }),
            RvCInsn::Ld { rd, rs1, imm } => Self::Ld(ITypeArgs { rd, rs1, imm }),
            RvCInsn::Fsd { rs1, rs2, imm } => Self::Fsd(SBTypeArgs { rs1, rs2, imm }),
            RvCInsn::Sw { rs1, rs2, imm } => Self::Sw(SBTypeArgs { rs1, rs2, imm }),
            RvCInsn::Fsw { rs1, rs2, imm } => Self::Fsw(SBTypeArgs { rs1, rs2, imm }),
            RvCInsn::Sd { rs1, rs2, imm } => Self::Sd(SBTypeArgs { rs1, rs2, imm }),
            RvCInsn::Addi { rd, imm } => Self::Addi(ITypeArgs { rd, rs1: rd, imm }),
            RvCInsn::Jal { imm } => Self::Jal(UJTypeArgs { rd: 1, imm }),
            RvCInsn::Addiw { rd, imm } => Self::Addiw(ITypeArgs { rd, rs1: rd, imm }),
            RvCInsn::Li { rd, imm } => Self::Addi(ITypeArgs { rd, rs1: 0, imm }),
            RvCInsn::Lui { rd, imm } => Self::Lui(UJTypeArgs { rd, imm }),
            RvCInsn::Addi16sp { imm } => Self::Addi(ITypeArgs { rd: 2, rs1: 2, imm }),
            RvCInsn::Srli { rd, imm } => Self::Srli(ShiftArgs {
                rd,
                rs1: rd,
                shamt: imm,
            }),
            RvCInsn::Srai { rd, imm } => Self::Srai(ShiftArgs {
                rd,
                rs1: rd,
                shamt: imm,
            }),
            RvCInsn::Andi { rd, imm } => Self::Andi(ITypeArgs { rd, rs1: rd, imm }),
            RvCInsn::Sub { rd, rs2 } => Self::Sub(RTypeArgs { rd, rs1: rd, rs2 }),
            RvCInsn::Xor { rd, rs2 } => Self::Xor(RTypeArgs { rd, rs1: rd, rs2 }),
            RvCInsn::Or { rd, rs2 } => Self::Or(RTypeArgs { rd, rs1: rd, rs2 }),
            RvCInsn::And { rd, rs2 } => Self::And(RTypeArgs { rd, rs1: rd, rs2 }),
            RvCInsn::Subw { rd, rs2 } => Self::Subw(RTypeArgs { rd, rs1: rd, rs2 }),
            RvCInsn::Addw { rd, rs2 } => Self::Addw(RTypeArgs { rd, rs1: rd, rs2 }),
            RvCInsn::J { imm } => Self::Jal(UJTypeArgs { rd: 0, imm }),
            RvCInsn::Beqz { rs1, imm } => Self::Beq(SBTypeArgs { rs1, rs2: 0, imm }),
            RvCInsn::Bnez { rs1, imm } => Self::Bne(SBTypeArgs { rs1, rs2: 0, imm }),
            _ => todo!(),
        }
    }
}

// Compressed register slot.
struct CReg(u8);

impl From<CReg> for u8 {
    fn from(x: CReg) -> Self {
        x.0 + 8
    }
}

pub(super) struct RvCDecoder {
    xlen: usize,
}

impl RvCDecoder {
    pub(super) fn new(xlen: usize) -> Self {
        Self { xlen }
    }

    pub(super) fn disas(&self, insn: u16) -> RvCInsn {
        match insn & 0b11 {
            0b00 => self.disas_00(insn),
            0b01 => self.disas_01(insn),
            0b10 => self.disas_10(insn),
            _ => panic!("should never happen"),
        }
    }

    fn disas_00(&self, insn: u16) -> RvCInsn {
        let rs1: u8 = CReg(((insn >> 7) & 0b111) as u8).into();
        let rd: u8 = CReg(((insn >> 2) & 0b111) as u8).into();

        let imm_slot = (insn >> 5) & 0b11111111;

        let nzuimm54_96_2_3 = {
            let a = (imm_slot >> 6) & 0b11; // nzuimm[5:4]
            let b = (imm_slot >> 2) & 0b1111; // nzuimm[9:6]
            let c = (imm_slot >> 1) & 0b1; // nzuimm[2]
            let d = imm_slot & 0b1; // nzuimm[3]
            (a << 4) | (b << 6) | (c << 2) | (d << 3)
        };

        let uimm53_76 = {
            let a = (imm_slot >> 5) & 0b111; // uimm[5:3]
            let b = imm_slot & 0b11; // uimm[7:6]
            (a << 3) | (b << 6)
        };

        let uimm53_2_6 = {
            let a = (imm_slot >> 5) & 0b111; // uimm[5:3]
            let b = (imm_slot >> 1) & 0b1; // uimm[2]
            let c = imm_slot & 0b1; // uimm[6]
            (a << 3) | (b << 2) | (c << 6)
        };

        match (self.xlen, insn >> 13) {
            (_, 0b000) => {
                if nzuimm54_96_2_3 != 0 {
                    RvCInsn::Addi4spn {
                        rd,
                        imm: nzuimm54_96_2_3 as i32,
                    }
                } else {
                    RvCInsn::Invalid(insn)
                }
            }
            (32 | 64, 0b001) => RvCInsn::Fld {
                rd,
                rs1,
                imm: uimm53_76 as i32,
            },
            (128, 0b001) => unimplemented!(),
            (_, 0b010) => RvCInsn::Lw {
                rd,
                rs1,
                imm: uimm53_2_6 as i32,
            },
            (32, 0b011) => RvCInsn::Flw {
                rd,
                rs1,
                imm: uimm53_2_6 as i32,
            },
            (64 | 128, 0b011) => RvCInsn::Ld {
                rd,
                rs1,
                imm: uimm53_76 as i32,
            },
            (32 | 64, 0b101) => RvCInsn::Fsd {
                rs1,
                rs2: rd,
                imm: uimm53_76 as i32,
            },
            (128, 0b101) => unimplemented!(),
            (_, 0b110) => RvCInsn::Sw {
                rs1,
                rs2: rd,
                imm: uimm53_2_6 as i32,
            },
            (32, 0b111) => RvCInsn::Fsw {
                rs1,
                rs2: rd,
                imm: uimm53_2_6 as i32,
            },
            (64 | 128, 0b111) => RvCInsn::Sd {
                rs1,
                rs2: rd,
                imm: uimm53_76 as i32,
            },

            _ => RvCInsn::Invalid(insn),
        }
    }

    fn disas_01(&self, insn: u16) -> RvCInsn {
        let b15_13 = insn >> 13; // insn[15:13]
        let b12 = (insn >> 12) & 0b1; // insn[12]
        let b11_7 = (insn >> 7) & 0b11111; // insn[11:7] -- full rd slot
        let b6_2 = (insn >> 2) & 0b11111; // insn[6:2]

        let full_rd = b11_7 as u8;
        let rd: u8 = CReg(((insn >> 7) & 0b111) as u8).into(); // insn[9:7] - rs1'/rd'

        let imm5_40 = simm_from_uimm((b12 << 5 | b6_2) as u32, 6);
        let imm17_1612 = imm5_40 << 12;

        let imm9_4_6_87_5 = {
            let a = b12;
            let b = (insn >> 6) & 0b1;
            let c = (insn >> 5) & 0b1;
            let d = (insn >> 3) & 0b11;
            let e = (insn >> 2) & 0b1;
            simm_from_uimm((a << 9 | b << 4 | c << 6 | d << 7 | e << 5) as u32, 10)
        };

        let imm_11_4_98_10_6_7_31_5 = {
            let a = b12;
            let b = (insn >> 11) & 0b1;
            let c = (insn >> 9) & 0b11;
            let d = (insn >> 8) & 0b1;
            let e = (insn >> 7) & 0b1;
            let f = (insn >> 6) & 0b1;
            let g = (insn >> 3) & 0b111;
            let h = (insn >> 2) & 0b1;
            simm_from_uimm(
                (a << 11 | b << 4 | c << 8 | d << 10 | e << 6 | f << 7 | g << 1 | h << 5) as u32,
                12,
            )
        };

        let imm_8_43_76_21_5 = {
            let a = b12;
            let b = (insn >> 10) & 0b11;
            let c = (insn >> 5) & 0b11;
            let d = (insn >> 3) & 0b11;
            let e = (insn >> 2) & 0b1;
            simm_from_uimm((a << 8 | b << 3 | c << 6 | d << 1 | e << 5) as u32, 9)
        };

        match (self.xlen, b15_13) {
            // c.nop and HINT of c.addi are not handled
            (_, 0b000) => RvCInsn::Addi {
                rd: full_rd,
                imm: imm5_40,
            },
            (32, 0b001) => RvCInsn::Jal {
                imm: imm_11_4_98_10_6_7_31_5,
            },
            (64 | 128, 0b001) => RvCInsn::Addiw {
                rd: full_rd,
                imm: imm5_40,
            },
            (_, 0b010) => RvCInsn::Li {
                rd: full_rd,
                imm: imm5_40,
            },
            (_, 0b011) => match full_rd {
                2 => RvCInsn::Addi16sp { imm: imm9_4_6_87_5 },
                // HINT (rd=0) not handled
                _ => RvCInsn::Lui {
                    rd,
                    imm: imm17_1612,
                },
            },
            (_, 0b100) => self.disas_01_100(insn),
            (_, 0b101) => RvCInsn::J {
                imm: imm_11_4_98_10_6_7_31_5,
            },
            (_, 0b110) => RvCInsn::Beqz {
                rs1: rd,
                imm: imm_8_43_76_21_5,
            },
            (_, 0b111) => RvCInsn::Bnez {
                rs1: rd,
                imm: imm_8_43_76_21_5,
            },
            _ => RvCInsn::Invalid(insn),
        }
    }

    fn disas_01_100(&self, insn: u16) -> RvCInsn {
        let b12 = (insn >> 12) & 0b1; // insn[12]
        let b11_10 = (insn >> 10) & 0b11; // insn[11:10]
        let b6_5 = (insn >> 5) & 0b11; // insn[6:5]
        let b6_2 = (insn >> 2) & 0b11111; // insn[6:2]

        let rd: u8 = CReg(((insn >> 7) & 0b111) as u8).into(); // insn[9:7] - rs1'/rd'
        let rs2: u8 = CReg(((insn >> 2) & 0b111) as u8).into(); // insn[4:2] - rs2'

        let uimm5_40 = (b12 << 5 | b6_2) as u8;
        let imm5_40 = simm_from_uimm(uimm5_40 as u32, 6);

        match (self.xlen, b12, b11_10, b6_5) {
            (32, 1, 0b00, _) => RvCInsn::Invalid(insn), // RV32 NSE
            (_, _, 0b00, _) => RvCInsn::Srli { rd, imm: uimm5_40 },
            (32, 1, 0b01, _) => RvCInsn::Invalid(insn), // RV32 NSE
            (_, _, 0b01, _) => RvCInsn::Srai { rd, imm: uimm5_40 },
            (_, _, 0b10, _) => RvCInsn::Andi { rd, imm: imm5_40 },
            (_, 0, 0b11, 0b00) => RvCInsn::Sub { rd, rs2 },
            (_, 0, 0b11, 0b01) => RvCInsn::Xor { rd, rs2 },
            (_, 0, 0b11, 0b10) => RvCInsn::Or { rd, rs2 },
            (_, 0, 0b11, 0b11) => RvCInsn::And { rd, rs2 },
            (64 | 128, 1, 0b11, 0b00) => RvCInsn::Subw { rd, rs2 },
            (64 | 128, 1, 0b11, 0b01) => RvCInsn::Addw { rd, rs2 },
            _ => RvCInsn::Invalid(insn),
        }
    }

    fn disas_10(&self, _: u16) -> RvCInsn {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rv64c_quadrant_00() {
        let d = RvCDecoder::new(64);

        // unimp
        assert_eq!(d.disas(0x0000), RvCInsn::Invalid(0));

        // c.ld a5,0(s0)
        assert_eq!(
            d.disas(0x601c),
            RvCInsn::Ld {
                rd: 15,
                rs1: 8,
                imm: 0
            }
        );

        // c.addi4spn s1,sp,168
        assert_eq!(d.disas(0x1124), RvCInsn::Addi4spn { rd: 9, imm: 168 });

        // c.sd a0,8(a5)
        assert_eq!(
            d.disas(0xe788),
            RvCInsn::Sd {
                rs1: 15,
                rs2: 10,
                imm: 8
            }
        )
    }

    #[test]
    fn test_rv64c_quadrant_01() {
        let d = RvCDecoder::new(64);

        // c.addi sp,-32
        assert_eq!(d.disas(0x1101), RvCInsn::Addi { rd: 2, imm: -32 });

        // c.lui a5,0x10
        assert_eq!(
            d.disas(0x67c1),
            RvCInsn::Lui {
                rd: 15,
                imm: 0x10 << 12
            }
        );

        // c.li a5,0
        assert_eq!(d.disas(0x4781), RvCInsn::Li { rd: 15, imm: 0 });

        // c.li s4,-1
        assert_eq!(d.disas(0x5a7d), RvCInsn::Li { rd: 20, imm: -1 });

        // c.li a0,12
        assert_eq!(d.disas(0x4531), RvCInsn::Li { rd: 10, imm: 12 });

        // c.sub a2,s0
        assert_eq!(d.disas(0x8e01), RvCInsn::Sub { rd: 12, rs2: 8 });

        // c.xor a4,a3
        assert_eq!(d.disas(0x8f35), RvCInsn::Xor { rd: 14, rs2: 13 });

        // c.beqz a0, +38
        assert_eq!(d.disas(0xc11d), RvCInsn::Beqz { rs1: 10, imm: 38 })
    }
}
