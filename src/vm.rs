use std::iter::Iterator;

use crate::{
    chunk::{self, Chunk, debug::format_instruction},
    opcode::OpCode,
    value::Value,
};

const STACK_MAX: usize = 256;

#[derive(Debug)]
pub enum ErrorKind {
    Runtime(String),
}

pub struct VM<'a> {
    chunk: &'a Chunk,
    stack: Vec<Value>, // TODO: store &Value directly to top of stack
}

impl<'a> VM<'a> {
    pub fn new(chunk: &'a Chunk) -> Self {
        VM {
            chunk,
            stack: Vec::with_capacity(STACK_MAX),
        }
    }

    pub fn set_chunk(&mut self, chunk: &'a Chunk) {
        self.chunk = chunk
    }

    pub fn reset(&mut self) {
        self.stack = Vec::with_capacity(STACK_MAX)
    }

    pub fn run(&mut self) -> Result<Value, ErrorKind> {
        self.reset();
        let mut bytes = self.chunk.iter_code().enumerate();

        loop {
            let opcode: OpCode = if let Some((offset, byte)) = bytes.next() {
                if false {
                    let stack = &self.stack;
                    println!("{stack:?}");
                    let mut s = String::default();
                    format_instruction(self.chunk, offset, &mut s);
                    println!("{s}");
                }
                *byte
            } else {
                break;
            }
            .into();

            match opcode {
                OpCode::Pass => {}
                OpCode::Constant => {
                    let index = self.next_byte(&mut bytes) as u16;
                    let val = self.read_const(index).clone();
                    self.push(val);
                }
                OpCode::ConstLong => {
                    let index = u16::from_ne_bytes([
                        self.next_byte(&mut bytes),
                        self.next_byte(&mut bytes),
                    ]);
                    let val = self.read_const(index).clone();
                    self.push(val);
                }
                OpCode::Return => {
                    let val = self.pop();
                    return Ok(val);
                }
                OpCode::Negate => {
                    let val = (-self.pop()).map_err(ErrorKind::Runtime)?;
                    self.push(val);
                }
                OpCode::Not => {
                    let val = (!self.pop()).map_err(ErrorKind::Runtime)?;
                    self.push(val);
                }
                OpCode::Add | OpCode::Subtract | OpCode::Multiply | OpCode::Divide => {
                    let a = self.pop();
                    let b = self.pop();
                    let res = match opcode {
                        OpCode::Add => b + a,
                        OpCode::Subtract => b - a,
                        OpCode::Multiply => b * a,
                        OpCode::Divide => b / a,
                        _ => unreachable!(),
                    }
                    .map_err(ErrorKind::Runtime)?;
                    self.push(res);
                }
                OpCode::True => self.push(true),
                OpCode::False => self.push(false),
                OpCode::Nil => self.push(Value::Nil),
                OpCode::Equal | OpCode::Greater | OpCode::Less => {
                    let a = self.pop();
                    let b = self.pop();
                    let res = match opcode {
                        OpCode::Equal => b == a,
                        OpCode::Greater => b > a,
                        OpCode::Less => b < a,
                        _ => unreachable!(),
                    };
                    self.push(res);
                }
            }
        }
        Ok(Value::Number(0.))
    }

    fn push(&mut self, val: impl Into<Value>) {
        self.stack.push(val.into());
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Attempt to pop empty stack")
    }

    fn next_byte(&mut self, bytes: &mut impl Iterator<Item = (usize, &'a u8)>) -> u8 {
        let (_, byte) = bytes.next().expect("Can't read constant");
        *byte
    }

    fn read_const(&self, index: u16) -> &Value {
        &self.chunk.constants[index as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Chunk;
    use crate::value::Value;

    fn chunk_with_constant(val: impl Into<Value>) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.add_constant(val, 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        chunk
    }

    fn chunk_with_binary_op(a: impl Into<Value>, b: impl Into<Value>, op: OpCode) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.add_constant(a, 0..0);
        chunk.add_constant(b, 0..0);
        chunk.add_code(op as u8, 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        chunk
    }

    #[test]
    fn test_empty_chunk() {
        let chunk = Chunk::new();
        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_ok_and(|x| x == Value::Number(0.)));
    }

    #[test]
    fn test_constant() {
        let chunk = chunk_with_constant(42.);
        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_ok_and(|x| x == Value::Number(42.)));
    }

    #[test]
    fn test_constant_long() {
        let mut chunk = Chunk::new();
        chunk.add_const_long(20., 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_ok_and(|x| x == Value::Number(20.)));
    }

    #[test]
    fn test_addition() {
        let chunk = chunk_with_binary_op(5.0, 3.0, OpCode::Add);
        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_ok_and(|x| x == Value::Number(8.)));
    }

    #[test]
    fn test_negate_operator() {
        let mut chunk = Chunk::new();
        chunk.add_constant(10., 0..0);
        chunk.add_code(OpCode::Negate, 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_ok_and(|x| x == Value::Number(-10.)));
    }

    #[test]
    fn test_multiplication() {
        let chunk = chunk_with_binary_op(2., 4.0, OpCode::Multiply);
        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_ok_and(|x| x == Value::Number(8.)));
    }

    #[test]
    fn test_division() {
        let chunk = chunk_with_binary_op(16., 4., OpCode::Divide);
        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_ok_and(|x| x == Value::Number(4.)));
    }

    #[test]
    fn test_division_by_zero() {
        let chunk = chunk_with_binary_op(16., 0., OpCode::Divide);
        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_err_and(
            |err| matches!(err, ErrorKind::Runtime(err) if err.eq("Division by zero.")
            )
        ))
    }

    #[test]
    fn test_subtraction() {
        let chunk = chunk_with_binary_op(16., 4., OpCode::Subtract);
        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_ok_and(|x| x == Value::Number(12.)));
    }

    #[test]
    fn test_multiple_operations() {
        let mut chunk = Chunk::new();
        let span = 0..1;
        chunk.add_constant(5., span.clone());

        chunk.add_constant(10., span.clone());
        chunk.add_constant(9., span.clone());
        chunk.add_code(OpCode::Subtract, span.clone());
        // 10 - 9 = 1
        chunk.add_constant(3., span.clone());
        chunk.add_constant(4., span.clone());
        chunk.add_code(OpCode::Add, span.clone());
        // 4 + 3 = 7
        chunk.add_constant(20., span.clone());
        chunk.add_code(OpCode::Multiply, span.clone());
        // 20 * 7 = 140
        chunk.add_code(OpCode::Divide, span.clone());
        // 1/140
        chunk.add_code(OpCode::Divide, span.clone());
        // 5 / (1/140) == 700
        chunk.add_code(OpCode::Return, span.clone());

        let mut vm = VM::new(&chunk);
        assert!(vm.run().is_ok_and(|x| x == Value::Number(700.)));
    }
}
