use super::args::*;
use super::disas_helper::*;

#[derive(Debug)]
pub enum RvInsn {
    // Invalid encoding
    Invalid(u32),

    // Privileged
    Ecall,
    Ebreak,

    // RV32I
    Lui(UJTypeArgs),
    Auipc(UJTypeArgs),
    Jal(UJTypeArgs),
    Jalr(ITypeArgs),
    Beq(SBTypeArgs),
    Bne(SBTypeArgs),
    Blt(SBTypeArgs),
    Bge(SBTypeArgs),
    Bltu(SBTypeArgs),
    Bgeu(SBTypeArgs),
    Lb(ITypeArgs),
    Lh(ITypeArgs),
    Lw(ITypeArgs),
    Lbu(ITypeArgs),
    Lhu(ITypeArgs),
    Sb(SBTypeArgs),
    Sh(SBTypeArgs),
    Sw(SBTypeArgs),
    Addi(ITypeArgs),
    Slti(ITypeArgs),
    Sltiu(ITypeArgs),
    Xori(ITypeArgs),
    Ori(ITypeArgs),
    Andi(ITypeArgs),
    Slli(ShiftArgs),
    Srli(ShiftArgs),
    Srai(ShiftArgs),
    Add(RTypeArgs),
    Sub(RTypeArgs),
    Sll(RTypeArgs),
    Slt(RTypeArgs),
    Sltu(RTypeArgs),
    Xor(RTypeArgs),
    Srl(RTypeArgs),
    Sra(RTypeArgs),
    Or(RTypeArgs),
    And(RTypeArgs),
    Fence(FenceArgs),

    // Zifencei
    FenceI(ITypeArgs),

    // RV64I
    Lwu(ITypeArgs),
    Ld(ITypeArgs),
    Sd(SBTypeArgs),
    Addiw(ITypeArgs),
    Slliw(ShiftArgs),
    Srliw(ShiftArgs),
    Sraiw(ShiftArgs),
    Addw(RTypeArgs),
    Subw(RTypeArgs),
    Sllw(RTypeArgs),
    Srlw(RTypeArgs),
    Sraw(RTypeArgs),

    // RV32M
    Mul(RTypeArgs),
    Mulh(RTypeArgs),
    Mulhsu(RTypeArgs),
    Mulhu(RTypeArgs),
    Div(RTypeArgs),
    Divu(RTypeArgs),
    Rem(RTypeArgs),
    Remu(RTypeArgs),

    // RV64M
    Mulw(RTypeArgs),
    Divw(RTypeArgs),
    Divuw(RTypeArgs),
    Remw(RTypeArgs),
    Remuw(RTypeArgs),

    // RV32A
    LrW(AmoLrArgs),
    ScW(AmoArgs),
    AmoSwapW(AmoArgs),
    AmoAddW(AmoArgs),
    AmoXorW(AmoArgs),
    AmoAndW(AmoArgs),
    AmoOrW(AmoArgs),
    AmoMinW(AmoArgs),
    AmoMaxW(AmoArgs),
    AmoMinuW(AmoArgs),
    AmoMaxuW(AmoArgs),

    // RV64A
    LrD(AmoLrArgs),
    ScD(AmoArgs),
    AmoSwapD(AmoArgs),
    AmoAddD(AmoArgs),
    AmoXorD(AmoArgs),
    AmoAndD(AmoArgs),
    AmoOrD(AmoArgs),
    AmoMinD(AmoArgs),
    AmoMaxD(AmoArgs),
    AmoMinuD(AmoArgs),
    AmoMaxuD(AmoArgs),

    // RV32F
    Flw(ITypeArgs),
    Fsw(SBTypeArgs),
    FmaddS(R4TypeArgs),
    FmsubS(R4TypeArgs),
    FnmsubS(R4TypeArgs),
    FnmaddS(R4TypeArgs),
    FaddS(RFTypeArgs),
    FsubS(RFTypeArgs),
    FmulS(RFTypeArgs),
    FdivS(RFTypeArgs),
    FsqrtS(R2FTypeArgs),
    FsgnjS(RTypeArgs),
    FsgnjnS(RTypeArgs),
    FsgnjxS(RTypeArgs),
    FminS(RTypeArgs),
    FmaxS(RTypeArgs),
    FcvtWS(R2FTypeArgs),
    FcvtWuS(R2FTypeArgs),
    FmvXW(R2TypeArgs),
    FeqS(RTypeArgs),
    FltS(RTypeArgs),
    FleS(RTypeArgs),
    FclassS(R2TypeArgs),
    FcvtSW(R2FTypeArgs),
    FcvtSWu(R2FTypeArgs),
    FmvWX(R2TypeArgs),

    // RV64F
    FcvtLS(R2FTypeArgs),
    FcvtLuS(R2FTypeArgs),
    FcvtSL(R2FTypeArgs),
    FcvtSLu(R2FTypeArgs),

    // RV32D
    Fld(ITypeArgs),
    Fsd(SBTypeArgs),
    FmaddD(R4TypeArgs),
    FmsubD(R4TypeArgs),
    FnmsubD(R4TypeArgs),
    FnmaddD(R4TypeArgs),
    FaddD(RFTypeArgs),
    FsubD(RFTypeArgs),
    FmulD(RFTypeArgs),
    FdivD(RFTypeArgs),
    FsqrtD(R2FTypeArgs),
    FsgnjD(RTypeArgs),
    FsgnjnD(RTypeArgs),
    FsgnjxD(RTypeArgs),
    FminD(RTypeArgs),
    FmaxD(RTypeArgs),
    FcvtSD(R2FTypeArgs),
    FcvtDS(R2FTypeArgs),
    FeqD(RTypeArgs),
    FltD(RTypeArgs),
    FleD(RTypeArgs),
    FclassD(R2TypeArgs),
    FcvtWD(R2FTypeArgs),
    FcvtWuD(R2FTypeArgs),
    FcvtDW(R2FTypeArgs),
    FcvtDWu(R2FTypeArgs),

    // RV64D
    FcvtLD(R2FTypeArgs),
    FcvtLuD(R2FTypeArgs),
    FmvXD(R2TypeArgs),
    FcvtDL(R2FTypeArgs),
    FcvtDLu(R2FTypeArgs),
    FmvDX(R2TypeArgs),
}

pub fn disas_riscv_insn_32bit(insn: u32) -> RvInsn {
    let opcode = (insn >> 2) & 0b11111;
    match opcode {
        0b00_000 => disas_riscv_insn_load(insn),
        0b00_001 => disas_riscv_insn_load_fp(insn),
        0b00_011 => disas_riscv_insn_misc_mem(insn),
        0b00_100 => disas_riscv_insn_op_imm(insn),
        0b00_101 => RvInsn::Auipc(disas_u(insn).into()),
        0b00_110 => disas_riscv_insn_op_imm_32(insn),
        0b01_000 => disas_riscv_insn_store(insn),
        0b01_001 => disas_riscv_insn_store_fp(insn),
        0b01_011 => disas_riscv_insn_amo(insn),
        0b01_100 => disas_riscv_insn_op(insn),
        0b01_101 => RvInsn::Lui(disas_u(insn).into()),
        0b01_110 => disas_riscv_insn_op_32(insn),
        0b10_000 => disas_riscv_insn_madd(insn),
        0b10_001 => disas_riscv_insn_msub(insn),
        0b10_010 => disas_riscv_insn_nmsub(insn),
        0b10_011 => disas_riscv_insn_nmadd(insn),
        0b10_100 => disas_riscv_insn_op_fp(insn),
        0b11_000 => disas_riscv_insn_branch(insn),
        0b11_001 => disas_riscv_insn_jalr(insn),
        0b11_011 => RvInsn::Jal(disas_j(insn).into()),
        0b11_100 => disas_riscv_insn_system(insn),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_load(insn: u32) -> RvInsn {
    let s = disas_i(insn);
    match s.i_funct3() {
        0b000 => RvInsn::Lb(s.into()),
        0b001 => RvInsn::Lh(s.into()),
        0b010 => RvInsn::Lw(s.into()),
        0b100 => RvInsn::Lbu(s.into()),
        0b101 => RvInsn::Lhu(s.into()),

        0b110 => RvInsn::Lwu(s.into()),
        0b011 => RvInsn::Ld(s.into()),

        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_load_fp(insn: u32) -> RvInsn {
    let s = disas_i(insn);
    match s.i_funct3() {
        0b010 => RvInsn::Flw(s.into()),
        0b011 => RvInsn::Fld(s.into()),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_misc_mem(insn: u32) -> RvInsn {
    let s = disas_i(insn);
    match s.i_funct3() {
        0b000 => RvInsn::Fence(s.into()),
        0b001 => RvInsn::FenceI(s.into()),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_op_imm(insn: u32) -> RvInsn {
    let s = disas_i(insn);
    match s.i_funct3() {
        0b000 => RvInsn::Addi(s.into()),
        0b010 => RvInsn::Slti(s.into()),
        0b011 => RvInsn::Sltiu(s.into()),
        0b100 => RvInsn::Xori(s.into()),
        0b110 => RvInsn::Ori(s.into()),
        0b111 => RvInsn::Andi(s.into()),

        0b001 | 0b101 => match (s.rv64_shift_funct(), s.i_funct3()) {
            (0b000000, 0b001) => RvInsn::Slli(s.into()),
            (0b000000, 0b101) => RvInsn::Srli(s.into()),
            (0b010000, 0b101) => RvInsn::Srai(s.into()),
            _ => RvInsn::Invalid(insn),
        },

        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_op_imm_32(insn: u32) -> RvInsn {
    let s = disas_i(insn);
    match (s.rv32_shift_funct(), s.i_funct3()) {
        (_, 0b000) => RvInsn::Addiw(s.into()),
        (0b0000000, 0b001) => RvInsn::Slliw(s.into()),
        (0b0000000, 0b101) => RvInsn::Srliw(s.into()),
        (0b0100000, 0b101) => RvInsn::Sraiw(s.into()),

        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_store(insn: u32) -> RvInsn {
    let s = disas_s(insn);
    match s.sb_funct3() {
        0b000 => RvInsn::Sb(s.into()),
        0b001 => RvInsn::Sh(s.into()),
        0b010 => RvInsn::Sw(s.into()),

        0b011 => RvInsn::Sd(s.into()),

        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_store_fp(insn: u32) -> RvInsn {
    let s = disas_s(insn);
    match s.sb_funct3() {
        0b010 => RvInsn::Fsw(s.into()),
        0b011 => RvInsn::Fsd(s.into()),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_amo(insn: u32) -> RvInsn {
    let s = disas_r(insn);
    match (s.funct3(), s.amo_funct()) {
        (0b010, 0b00010) => {
            if s.rs2() == 0 {
                RvInsn::LrW(s.into())
            } else {
                RvInsn::Invalid(insn)
            }
        }
        (0b010, 0b00011) => RvInsn::ScW(s.into()),
        (0b010, 0b00001) => RvInsn::AmoSwapW(s.into()),
        (0b010, 0b00000) => RvInsn::AmoAddW(s.into()),
        (0b010, 0b00100) => RvInsn::AmoXorW(s.into()),
        (0b010, 0b01100) => RvInsn::AmoAndW(s.into()),
        (0b010, 0b01000) => RvInsn::AmoOrW(s.into()),
        (0b010, 0b10000) => RvInsn::AmoMinW(s.into()),
        (0b010, 0b10100) => RvInsn::AmoMaxW(s.into()),
        (0b010, 0b11000) => RvInsn::AmoMinuW(s.into()),
        (0b010, 0b11100) => RvInsn::AmoMaxuW(s.into()),

        (0b011, 0b00010) => {
            if s.rs2() == 0 {
                RvInsn::LrD(s.into())
            } else {
                RvInsn::Invalid(insn)
            }
        }
        (0b011, 0b00011) => RvInsn::ScD(s.into()),
        (0b011, 0b00001) => RvInsn::AmoSwapD(s.into()),
        (0b011, 0b00000) => RvInsn::AmoAddD(s.into()),
        (0b011, 0b00100) => RvInsn::AmoXorD(s.into()),
        (0b011, 0b01100) => RvInsn::AmoAndD(s.into()),
        (0b011, 0b01000) => RvInsn::AmoOrD(s.into()),
        (0b011, 0b10000) => RvInsn::AmoMinD(s.into()),
        (0b011, 0b10100) => RvInsn::AmoMaxD(s.into()),
        (0b011, 0b11000) => RvInsn::AmoMinuD(s.into()),
        (0b011, 0b11100) => RvInsn::AmoMaxuD(s.into()),

        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_op(insn: u32) -> RvInsn {
    let s = disas_r(insn);
    match (s.funct7(), s.funct3()) {
        (0b0000000, 0b000) => RvInsn::Add(s.into()),
        (0b0100000, 0b000) => RvInsn::Sub(s.into()),
        (0b0000000, 0b001) => RvInsn::Sll(s.into()),
        (0b0000000, 0b010) => RvInsn::Slt(s.into()),
        (0b0000000, 0b011) => RvInsn::Sltu(s.into()),
        (0b0000000, 0b100) => RvInsn::Xor(s.into()),
        (0b0000000, 0b101) => RvInsn::Srl(s.into()),
        (0b0100000, 0b101) => RvInsn::Sra(s.into()),
        (0b0000000, 0b110) => RvInsn::Or(s.into()),
        (0b0000000, 0b111) => RvInsn::And(s.into()),

        (0b0000001, 0b000) => RvInsn::Mul(s.into()),
        (0b0000001, 0b001) => RvInsn::Mulh(s.into()),
        (0b0000001, 0b010) => RvInsn::Mulhsu(s.into()),
        (0b0000001, 0b011) => RvInsn::Mulhu(s.into()),
        (0b0000001, 0b100) => RvInsn::Div(s.into()),
        (0b0000001, 0b101) => RvInsn::Divu(s.into()),
        (0b0000001, 0b110) => RvInsn::Rem(s.into()),
        (0b0000001, 0b111) => RvInsn::Remu(s.into()),

        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_op_32(insn: u32) -> RvInsn {
    let s = disas_r(insn);
    match (s.funct7(), s.funct3()) {
        (0b0000000, 0b000) => RvInsn::Addw(s.into()),
        (0b0100000, 0b000) => RvInsn::Subw(s.into()),
        (0b0000000, 0b001) => RvInsn::Sllw(s.into()),
        (0b0000000, 0b101) => RvInsn::Srlw(s.into()),
        (0b0100000, 0b101) => RvInsn::Sraw(s.into()),

        (0b0000001, 0b000) => RvInsn::Mulw(s.into()),
        (0b0000001, 0b100) => RvInsn::Divw(s.into()),
        (0b0000001, 0b101) => RvInsn::Divuw(s.into()),
        (0b0000001, 0b110) => RvInsn::Remw(s.into()),
        (0b0000001, 0b111) => RvInsn::Remuw(s.into()),

        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_madd(insn: u32) -> RvInsn {
    let s = disas_r4(insn);
    match s.funct2() {
        0b00 => RvInsn::FmaddS(s.into()),
        0b01 => RvInsn::FmaddD(s.into()),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_msub(insn: u32) -> RvInsn {
    let s = disas_r4(insn);
    match s.funct2() {
        0b00 => RvInsn::FmsubS(s.into()),
        0b01 => RvInsn::FmsubD(s.into()),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_nmsub(insn: u32) -> RvInsn {
    let s = disas_r4(insn);
    match s.funct2() {
        0b00 => RvInsn::FnmsubS(s.into()),
        0b01 => RvInsn::FnmsubD(s.into()),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_nmadd(insn: u32) -> RvInsn {
    let s = disas_r4(insn);
    match s.funct2() {
        0b00 => RvInsn::FnmaddS(s.into()),
        0b01 => RvInsn::FnmaddD(s.into()),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_op_fp(insn: u32) -> RvInsn {
    let s = disas_r(insn);
    match (s.funct7(), s.rs2(), s.funct3()) {
        // RV32F
        (0b0000000, _, _) => RvInsn::FaddS(s.into()),
        (0b0000100, _, _) => RvInsn::FsubS(s.into()),
        (0b0001000, _, _) => RvInsn::FmulS(s.into()),
        (0b0001100, _, _) => RvInsn::FdivS(s.into()),
        (0b0101100, 0b00000, _) => RvInsn::FsqrtS(s.into()),
        (0b0010000, _, 0b000) => RvInsn::FsgnjS(s.into()),
        (0b0010000, _, 0b001) => RvInsn::FsgnjnS(s.into()),
        (0b0010000, _, 0b010) => RvInsn::FsgnjxS(s.into()),
        (0b0010100, _, 0b000) => RvInsn::FminS(s.into()),
        (0b0010100, _, 0b001) => RvInsn::FmaxS(s.into()),
        (0b1100000, 0b00000, _) => RvInsn::FcvtWS(s.into()),
        (0b1100000, 0b00001, _) => RvInsn::FcvtWuS(s.into()),
        (0b1110000, 0b00000, 0b000) => RvInsn::FmvXW(s.into()),
        (0b1010000, _, 0b010) => RvInsn::FeqS(s.into()),
        (0b1010000, _, 0b001) => RvInsn::FltS(s.into()),
        (0b1010000, _, 0b000) => RvInsn::FleS(s.into()),
        (0b1110000, 0b00000, 0b001) => RvInsn::FclassS(s.into()),
        (0b1101000, 0b00000, _) => RvInsn::FcvtSW(s.into()),
        (0b1101000, 0b00001, _) => RvInsn::FcvtSWu(s.into()),
        (0b1111000, 0b00000, 0b000) => RvInsn::FmvWX(s.into()),

        // RV64F
        (0b1100000, 0b00010, _) => RvInsn::FcvtLS(s.into()),
        (0b1100000, 0b00011, _) => RvInsn::FcvtLuS(s.into()),
        (0b1101000, 0b00010, _) => RvInsn::FcvtSL(s.into()),
        (0b1101000, 0b00011, _) => RvInsn::FcvtSLu(s.into()),

        // RV32D
        (0b0000001, _, _) => RvInsn::FaddD(s.into()),
        (0b0000101, _, _) => RvInsn::FsubD(s.into()),
        (0b0001001, _, _) => RvInsn::FmulD(s.into()),
        (0b0001101, _, _) => RvInsn::FdivD(s.into()),
        (0b0101101, 0b00000, _) => RvInsn::FsqrtD(s.into()),
        (0b0010001, _, 0b000) => RvInsn::FsgnjD(s.into()),
        (0b0010001, _, 0b001) => RvInsn::FsgnjnD(s.into()),
        (0b0010001, _, 0b010) => RvInsn::FsgnjxD(s.into()),
        (0b0010101, _, 0b000) => RvInsn::FminD(s.into()),
        (0b0010101, _, 0b001) => RvInsn::FmaxD(s.into()),
        (0b0100000, 0b00001, _) => RvInsn::FcvtSD(s.into()),
        (0b0100001, 0b00000, _) => RvInsn::FcvtDS(s.into()),
        (0b1010001, _, 0b010) => RvInsn::FeqD(s.into()),
        (0b1010001, _, 0b001) => RvInsn::FltD(s.into()),
        (0b1010001, _, 0b000) => RvInsn::FleD(s.into()),
        (0b1110001, 0b00000, 0b001) => RvInsn::FclassD(s.into()),
        (0b1100001, 0b00000, _) => RvInsn::FcvtWD(s.into()),
        (0b1100001, 0b00001, _) => RvInsn::FcvtWuD(s.into()),
        (0b1101001, 0b00000, _) => RvInsn::FcvtDW(s.into()),
        (0b1101001, 0b00001, _) => RvInsn::FcvtDWu(s.into()),

        // RV64D
        (0b1100001, 0b00010, _) => RvInsn::FcvtLD(s.into()),
        (0b1100001, 0b00011, _) => RvInsn::FcvtLuD(s.into()),
        (0b1110001, 0b00000, 0b000) => RvInsn::FmvXD(s.into()),
        (0b1101001, 0b00010, _) => RvInsn::FcvtDL(s.into()),
        (0b1101001, 0b00011, _) => RvInsn::FcvtDLu(s.into()),
        (0b1111001, 0b00000, 0b000) => RvInsn::FmvDX(s.into()),

        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_branch(insn: u32) -> RvInsn {
    let s = disas_b(insn);
    match s.sb_funct3() {
        0b000 => RvInsn::Beq(s.into()),
        0b001 => RvInsn::Bne(s.into()),
        0b100 => RvInsn::Blt(s.into()),
        0b101 => RvInsn::Bge(s.into()),
        0b110 => RvInsn::Bltu(s.into()),
        0b111 => RvInsn::Bgeu(s.into()),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_jalr(insn: u32) -> RvInsn {
    let s = disas_i(insn);
    match s.i_funct3() {
        0b000 => RvInsn::Jalr(s.into()),
        _ => RvInsn::Invalid(insn),
    }
}

fn disas_riscv_insn_system(insn: u32) -> RvInsn {
    match insn {
        0x00000073 => RvInsn::Ecall,
        0x00100073 => RvInsn::Ebreak,
        _ => RvInsn::Invalid(insn),
    }
}
