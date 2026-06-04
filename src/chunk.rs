use crate::opcode::OpCode;
use crate::value::Value;
use debug::format_chunk;
use std::fmt::Display;
use std::slice::Iter;

pub mod debug;

#[derive(Debug, Default)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: Vec<Value>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk::default()
    }

    pub fn add_code(&mut self, byte: impl Into<u8>, line: usize) {
        self.code.push(byte.into());
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: impl Into<Value>, line: usize) {
        let const_index = self.push_constant(value);
        if const_index < 256 {
            self.add_code(OpCode::Constant, line);
            self.add_code(const_index as u8, line);
        } else if const_index < 2usize.pow(16) {
            let const_index: [u8; 2] = (const_index as u16).to_le_bytes();
            self.add_code(OpCode::ConstLong, line);
            self.add_code(const_index[0], line);
            self.add_code(const_index[1], line);
        } else {
            panic!("Can't store more constants")
        }
    }

    pub fn iter_code(&self) -> Iter<u8> {
        self.code.iter()
    }

    /// Adds a constant to the chunk. Returns it's index
    fn push_constant(&mut self, value: impl Into<Value>) -> usize {
        self.constants.push(value.into());
        self.constants.len() - 1
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
        assert!(chunk.lines.is_empty());
        assert!(chunk.constants.is_empty());
    }

    #[test]
    fn add_code_appends_byte_and_line() {
        let mut chunk = Chunk::new();
        chunk.add_code(42u8, 10);
        chunk.add_code(100u8, 20);

        assert_eq!(chunk.code, vec![42, 100]);
        assert_eq!(chunk.lines, vec![10, 20]);
    }

    #[test]
    fn add_constant_small_index() {
        let mut chunk = Chunk::new();

        for _ in 0..256 {
            chunk.add_constant(999.0, 7);
        }
        let expected_code = vec![1, 255];

        assert_eq!(&chunk.code[chunk.code.len() - 2..], &expected_code);
        assert_eq!(chunk.lines.last().unwrap(), &7);
        assert_eq!(chunk.constants.last().unwrap(), &Value::Number(999.0));
    }

    #[test]
    fn add_constant_long_index() {
        let mut chunk = Chunk::new();
        for i in 0..300 {
            chunk.push_constant(i as f64);
        }
        chunk.add_constant(12345.0, 42);

        let index: u16 = 300;
        let [low, high] = index.to_le_bytes();
        let last_bytes = &chunk.code[chunk.code.len() - 3..];

        assert_eq!(last_bytes, &[2, low, high]);
        assert_eq!(chunk.lines[chunk.lines.len() - 3..], [42, 42, 42]);
        assert_eq!(chunk.constants.last().unwrap(), &Value::Number(12345.0));
    }

    #[test]
    fn add_constant_max_long_index() {
        let mut chunk = Chunk::new();
        for _ in 0..2usize.pow(16) {
            chunk.add_constant(3.15, 100);
        }
    }

    #[test]
    #[should_panic(expected = "Can't store more constants")]
    fn add_constant_too_many_panics() {
        let mut chunk = Chunk::new();
        for _ in 0..2usize.pow(16) {
            chunk.add_constant(3.15, 100);
        }
        chunk.add_constant(3.15, 100);
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
