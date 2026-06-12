use crate::opcode::OpCode;
use crate::value::Value;
use debug::format_chunk;
use std::fmt::Display;
use std::ops::Range;

pub mod debug;

#[derive(Debug, Default)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub spans: Vec<Range<usize>>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk::default()
    }

    pub fn add_code(&mut self, byte: impl Into<u8>, span: impl Into<Range<usize>>) {
        self.code.push(byte.into());
        self.spans.push(span.into());
    }

    pub fn add_index_code(&mut self, opcode: OpCode, index: u16, span: Range<usize>) {
        let index_bytes: [u8; 2] = (index).to_le_bytes();
        self.add_code(opcode, span.clone());
        self.add_code(index_bytes[0], span.clone());
        self.add_code(index_bytes[1], span);
    }

    pub fn add_const_code(&mut self, opcode: OpCode, value: impl Into<Value>, span: Range<usize>) {
        let index = self.push_constant(value);
        self.add_index_code(opcode, index as u16, span);
    }

    pub fn push_constant(&mut self, value: impl Into<Value>) -> usize {
        let const_size = self.constants.len();
        if const_size < 2usize.pow(16) {
            self.constants.push(value.into());
            self.constants.len() - 1
        } else {
            panic!("Can't store more constants")
        }
    }
}

impl Display for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format_chunk(self, f);
        Ok(())
    }
}

#[cfg(test)]
mod test_chunk {
    use super::*;

    #[test]
    fn new_chunk_is_empty() {
        let chunk = Chunk::new();
        assert!(chunk.code.is_empty());
        assert!(chunk.spans.is_empty());
        assert!(chunk.constants.is_empty());
    }

    #[test]
    fn add_code_appends_byte_and_span() {
        let mut chunk = Chunk::new();
        chunk.add_code(42u8, 0..1);
        chunk.add_code(100u8, 1..2);

        assert_eq!(chunk.code, vec![42, 100]);
        assert_eq!(chunk.spans, vec![0..1, 1..2]);
    }

    #[test]
    fn add_constant() {
        let mut chunk = Chunk::new();
        for i in 0..300 {
            chunk.push_constant(i as f64);
        }
        chunk.add_const_code(OpCode::Constant, 12345.0, 0..1);

        let index: u16 = 300;
        let [low, high] = index.to_le_bytes();
        let last_bytes = &chunk.code[chunk.code.len() - 3..];

        assert_eq!(last_bytes, &[1, low, high]);
        assert_eq!(chunk.spans[chunk.spans.len() - 3..], [0..1, 0..1, 0..1]);
        assert_eq!(chunk.constants.last().unwrap(), &Value::Number(12345.0));
    }

    #[test]
    #[should_panic(expected = "Can't store more constants")]
    fn add_constant_too_many_panics() {
        let mut chunk = Chunk::new();
        for _ in 0..2usize.pow(16) {
            chunk.add_const_code(OpCode::Constant, 3.15, 0..1);
        }

        chunk.add_const_code(OpCode::Constant, 3.15, 0..1);
    }

    #[test]
    fn push_constant_returns_correct_index() {
        let mut chunk = Chunk::new();
        let idx1 = chunk.push_constant(10.0);
        let idx2 = chunk.push_constant(20.0);
        let idx3 = chunk.push_constant(30.0);

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 2);
        assert_eq!(chunk.constants.len(), 3);
    }
}
