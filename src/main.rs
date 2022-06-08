fn main() {
    macro_rules! d {
        ($x: expr) => {
            println!("{:?}", disas_riscv_insn_32bit($x));
        }
    }

    d!(0x000fc397);
    d!(0x41c30333);
    d!(0x4c03be03);
    d!(0x00135313);
    d!(0x000e0067);
}

#[derive(Debug)]
struct RTypeArgs {
    rd: u8,
    rs1: u8,
    rs2: u8,
}

#[derive(Debug)]
struct ITypeArgs {
    rd: u8,
    rs1: u8,
    imm: i32,
}

#[derive(Debug)]
struct SBTypeArgs {
    rs1: u8,
    rs2: u8,
    imm: i32,
}

#[derive(Debug)]
struct UJTypeArgs {
    rd: u8,
    imm: i32,
}

// variant of RTypeArgs
#[derive(Debug)]
struct ShiftArgs {
    rd: u8,
    rs1: u8,
    shamt: u8,
}

#[derive(Debug)]
enum RvInsn {
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

fn disas_riscv_insn_32bit(insn: u32) -> RvInsn {
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

// funct7, rs2, rs1, funct3, rd
struct RTypeSlots(u8, u8, u8, u8, u8);

impl RTypeSlots {
    fn funct7(&self) -> u8 {
        self.0
    }

    fn funct3(&self) -> u8 {
        self.3
    }
}

impl From<RTypeSlots> for RTypeArgs {
    fn from(x: RTypeSlots) -> Self {
        Self { rd: x.4, rs1: x.2, rs2: x.1 }
    }
}

// I-type: imm, rs1, funct3, rd
// S-type & B-type: imm, rs2, rs1, funct3
struct ISBTypeSlots(i32, u8, u8, u8);

impl ISBTypeSlots {
    fn i_funct3(&self) -> u8 {
        self.2
    }

    fn sb_funct3(&self) -> u8 {
        self.3
    }

    fn rv32_shift_funct(&self) -> u8 {
        (self.0 >> 25) as u8
    }

    fn rv64_shift_funct(&self) -> u8 {
        (self.0 >> 26) as u8
    }

}

impl From<ISBTypeSlots> for ITypeArgs {
    fn from(x: ISBTypeSlots) -> Self {
        Self { rd: x.3, rs1: x.1, imm: x.0 }
    }
}

impl From<ISBTypeSlots> for SBTypeArgs {
    fn from(x: ISBTypeSlots) -> Self {
        Self { rs1: x.2, rs2: x.1, imm: x.0 }
    }
}

impl From<ISBTypeSlots> for ShiftArgs {
    fn from(x: ISBTypeSlots) -> Self {
        Self { rd: x.3, rs1: x.1, shamt: (x.0 & 0xff) as u8 }
    }
}

// U-type & J-type: imm, rd
struct UJTypeSlots(i32, u8);

impl From<UJTypeSlots> for UJTypeArgs {
    fn from(x: UJTypeSlots) -> Self {
        Self { rd: x.1, imm: x.0 }
    }
}

fn disas_r(insn: u32) -> RTypeSlots {
    let funct7 = (insn >> 25) as u8;
    let rs2 = ((insn >> 20) & 0b11111) as u8;
    let rs1 = ((insn >> 15) & 0b11111) as u8;
    let funct3 = ((insn >> 12) & 0b111) as u8;
    let rd = ((insn >> 7) & 0b11111) as u8;
    RTypeSlots(funct7, rs2, rs1, funct3, rd)
}

fn disas_i(insn: u32) -> ISBTypeSlots {
    let imm = simm_from_uimm(insn >> 20, 12);
    let rs1 = ((insn >> 15) & 0b11111) as u8;
    let funct3 = ((insn >> 12) & 0b111) as u8;
    let rd = ((insn >> 7) & 0b11111) as u8;
    ISBTypeSlots(imm, rs1, funct3, rd)
}

fn disas_s(insn: u32) -> ISBTypeSlots {
    let imm = (insn >> (25 - 5)) | ((insn >> 7) & 0b11111);
    let imm = simm_from_uimm(imm, 12);
    let rs2 = ((insn >> 20) & 0b11111) as u8;
    let rs1 = ((insn >> 15) & 0b11111) as u8;
    let funct3 = ((insn >> 12) & 0b111) as u8;
    ISBTypeSlots(imm, rs2, rs1, funct3)
}

fn disas_b(insn: u32) -> ISBTypeSlots {
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

fn disas_u(insn: u32) -> UJTypeSlots {
    let imm = simm_from_uimm(insn >> 12, 20) << 12;
    let rd = ((insn >> 7) & 0b11111) as u8;
    UJTypeSlots(imm, rd)
}

fn disas_j(insn: u32) -> UJTypeSlots {
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
