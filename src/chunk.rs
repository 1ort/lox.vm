use crate::opcode::OpCode;
use crate::value::Value;
use debug::format_chunk;
use std::fmt::Display;
use std::ops::Range;
use std::rc::Rc;

pub mod debug;

#[derive(Debug)]
pub struct FunctionObject {
    pub chunk: Chunk,
    pub arity: u8,
    pub name: Rc<str>,
}

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

    pub fn add_byte(&mut self, byte: impl Into<u8>, span: impl Into<Range<usize>>) {
        self.code.push(byte.into());
        self.spans.push(span.into());
    }
    pub fn add_word(&mut self, word: impl Into<u16>, span: impl Into<Range<usize>>) {
        let bytes: [u8; 2] = (word.into()).to_le_bytes();
        let span = span.into();
        self.add_byte(bytes[0], span.clone());
        self.add_byte(bytes[1], span);
    }

    pub fn patch_word(&mut self, offset: usize, word: impl Into<u16>) {
        let bytes: [u8; 2] = (word.into()).to_le_bytes();
        self.code[offset] = bytes[0];
        self.code[offset + 1] = bytes[1];
    }

    pub fn push_constant(&mut self, value: impl Into<Value>) -> u16 {
        let const_size = self.constants.len();
        if const_size < 2usize.pow(16) {
            self.constants.push(value.into());
            (self.constants.len() - 1) as u16
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
        chunk.add_byte(42u8, 0..1);
        chunk.add_byte(100u8, 1..2);

        assert_eq!(chunk.code, vec![42, 100]);
        assert_eq!(chunk.spans, vec![0..1, 1..2]);
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
