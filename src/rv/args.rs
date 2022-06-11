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

#[derive(Debug)]
pub struct FenceSet {
    pub i: bool,
    pub o: bool,
    pub r: bool,
    pub w: bool,
}

impl From<u8> for FenceSet {
    fn from(x: u8) -> Self {
        Self {
            i: x & 0b1000 != 0,
            o: x & 0b0100 != 0,
            r: x & 0b0010 != 0,
            w: x & 0b0001 != 0,
        }
    }
}

// variant of ITypeArgs
#[derive(Debug)]
pub struct FenceArgs {
    pub fm: u8,
    pub pred: FenceSet,
    pub succ: FenceSet,
}
