#[derive(Debug)]
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
            _ => Pass,
        }
    }
}
