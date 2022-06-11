#[derive(PartialEq, Debug)]
enum RvCInsn {
    Invalid(u16),
    Addi4spn { rd: u8, imm: u16 },
    Fld { rd: u8, rs1: u8, imm: u16 },
    Lw { rd: u8, rs1: u8, imm: u16 },
    Flw { rd: u8, rs1: u8, imm: u16 },
    Ld { rd: u8, rs1: u8, imm: u16 },
    Fsd { rd: u8, rs1: u8, imm: u16 },
    Sw { rd: u8, rs1: u8, imm: u16 },
    Fsw { rd: u8, rs1: u8, imm: u16 },
    Sd { rd: u8, rs1: u8, imm: u16 },
    Addi,
    Jal,
    Addiw,
    Li,
    Lui,
    Addi16sp,
    MiscAlu,
    J,
    Beqz,
    Bnez,
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

// Compressed register slot.
struct CReg(u8);

impl From<CReg> for u8 {
    fn from(x: CReg) -> Self {
        x.0 + 8
    }
}

struct RvCDecoder {
    xlen: usize,
}

impl RvCDecoder {
    fn new(xlen: usize) -> Self {
        Self { xlen }
    }

    fn disas(&self, insn: u16) -> RvCInsn {
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
                        imm: nzuimm54_96_2_3,
                    }
                } else {
                    RvCInsn::Invalid(insn)
                }
            }
            (32 | 64, 0b001) => RvCInsn::Fld {
                rd,
                rs1,
                imm: uimm53_76,
            },
            (128, 0b001) => unimplemented!(),
            (_, 0b010) => RvCInsn::Lw {
                rd,
                rs1,
                imm: uimm53_2_6,
            },
            (32, 0b011) => RvCInsn::Flw {
                rd,
                rs1,
                imm: uimm53_2_6,
            },
            (64 | 128, 0b011) => RvCInsn::Ld {
                rd,
                rs1,
                imm: uimm53_76,
            },
            (32 | 64, 0b101) => RvCInsn::Fsd {
                rd,
                rs1,
                imm: uimm53_76,
            },
            (128, 0b101) => unimplemented!(),
            (_, 0b110) => RvCInsn::Sw {
                rd,
                rs1,
                imm: uimm53_2_6,
            },
            (32, 0b111) => RvCInsn::Fsw {
                rd,
                rs1,
                imm: uimm53_2_6,
            },
            (64 | 128, 0b111) => RvCInsn::Sd {
                rd,
                rs1,
                imm: uimm53_76,
            },

            _ => RvCInsn::Invalid(insn),
        }
    }

    fn disas_01(&self, _: u16) -> RvCInsn {
        todo!();
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
                rd: 10,
                rs1: 15,
                imm: 8
            }
        )
    }
}
