use crate::chunk::debug::format_chunk;
use std::fmt::Display;

mod debug;

#[derive(Debug)]
pub enum OpCode {
    Pass,
    Constant,
    Return,
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
            _ => Pass,
        }
    }
}

#[derive(Debug)]
pub enum Value {
    Number(f64),
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

#[derive(Debug, Default)]
pub struct Chunk {
    code: Vec<u8>,
    lines: Vec<usize>,
    constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk::default()
    }

    /// Adds a constant to the chunk. Returns it's index
    pub fn add_constant(&mut self, value: impl Into<Value>) -> usize {
        self.constants.push(value.into());
        self.constants.len() - 1
    }

    pub fn add_code(&mut self, byte: impl Into<u8>, line: usize) {
        self.code.push(byte.into());
        self.lines.push(line);
    }
}

impl Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format_chunk(self, f);
        Ok(())
    }
}
