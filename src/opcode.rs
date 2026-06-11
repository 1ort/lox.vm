#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[non_exhaustive]
pub enum OpCode {
    Pass,
    Constant,
    ConstLong,
    Return,
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    True,
    False,
    Nil,
    Equal,
    Greater,
    Less,
    Print,
    Pop,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
}

impl From<OpCode> for u8 {
    fn from(value: OpCode) -> Self {
        value as u8
    }
}

impl From<u8> for OpCode {
    fn from(value: u8) -> Self {
        use OpCode::*;
        match value {
            1 => Constant,
            2 => ConstLong,
            3 => Return,
            4 => Negate,
            5 => Add,
            6 => Subtract,
            7 => Multiply,
            8 => Divide,
            9 => Not,
            10 => True,
            11 => False,
            12 => Nil,
            13 => Equal,
            14 => Greater,
            15 => Less,
            16 => Print,
            17 => Pop,
            18 => DefineGlobal,
            19 => GetGlobal,
            20 => SetGlobal,
            _ => Pass,
        }
    }
}
