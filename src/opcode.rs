#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[non_exhaustive]
pub enum OpCode {
    Pass,
    Constant,
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
            2 => Return,
            3 => Negate,
            4 => Add,
            5 => Subtract,
            6 => Multiply,
            7 => Divide,
            8 => Not,
            9 => True,
            10 => False,
            11 => Nil,
            12 => Equal,
            13 => Greater,
            14 => Less,
            15 => Print,
            16 => Pop,
            17 => DefineGlobal,
            18 => GetGlobal,
            19 => SetGlobal,
            _ => Pass,
        }
    }
}
