#[derive(Debug)]
pub struct RTypeArgs {
    pub rd: u8,
    pub rs1: u8,
    pub rs2: u8,
}

#[derive(Debug)]
pub struct ITypeArgs {
    pub rd: u8,
    pub rs1: u8,
    pub imm: i32,
}

#[derive(Debug)]
pub struct SBTypeArgs {
    pub rs1: u8,
    pub rs2: u8,
    pub imm: i32,
}

#[derive(Debug)]
pub struct UJTypeArgs {
    pub rd: u8,
    pub imm: i32,
}

// variant of RTypeArgs
#[derive(Debug)]
pub struct ShiftArgs {
    pub rd: u8,
    pub rs1: u8,
    pub shamt: u8,
}

// variant of RTypeArgs
#[derive(Debug)]
pub struct AmoArgs {
    pub aq: bool,
    pub rl: bool,
    pub rd: u8,
    pub rs1: u8,
    pub rs2: u8,
}

// variant of RTypeArgs
#[derive(Debug)]
pub struct AmoLrArgs {
    pub aq: bool,
    pub rl: bool,
    pub rd: u8,
    pub rs1: u8,
}
