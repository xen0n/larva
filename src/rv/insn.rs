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
    Fence,

    // Zifencei
    FenceI,

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
    todo!();
}

fn disas_riscv_insn_misc_mem(insn: u32) -> RvInsn {
    todo!();
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

        0b001 | 0b101 => {
            match (s.rv64_shift_funct(), s.i_funct3()) {
                (0b000000, 0b001) => RvInsn::Slli(s.into()),
                (0b000000, 0b101) => RvInsn::Srli(s.into()),
                (0b010000, 0b101) => RvInsn::Srai(s.into()),
                _ => RvInsn::Invalid(insn),
            }
        }

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
    todo!();
}

fn disas_riscv_insn_amo(insn: u32) -> RvInsn {
    todo!();
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
    todo!();
}

fn disas_riscv_insn_msub(insn: u32) -> RvInsn {
    todo!();
}

fn disas_riscv_insn_nmsub(insn: u32) -> RvInsn {
    todo!();
}

fn disas_riscv_insn_nmadd(insn: u32) -> RvInsn {
    todo!();
}

fn disas_riscv_insn_op_fp(insn: u32) -> RvInsn {
    todo!();
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
