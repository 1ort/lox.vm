use std::iter::Iterator;

use crate::{
    chunk::{Chunk, debug::format_instruction},
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
    stack: Vec<Value>, // todo: store &Value directly to top of stack
}

impl<'a> VM<'a> {
    pub fn new(chunk: &'a Chunk) -> Self {
        VM {
            chunk,
            stack: Vec::with_capacity(STACK_MAX),
        }
    }

    pub fn reset(&mut self) {
        self.stack = Vec::with_capacity(STACK_MAX)
    }

    pub fn run(&mut self) -> Result<(), ErrorKind> {
        self.reset();
        let mut bytes = self.chunk.iter_code().enumerate();

        loop {
            let opcode: OpCode = if let Some((offset, byte)) = bytes.next() {
                {
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
                    println!("{val:?}");
                    return Ok(());
                }
                OpCode::Negate => {
                    let val = (-self.pop()).map_err(ErrorKind::Runtime)?;
                    self.push(val);
                }
                OpCode::Add | OpCode::Subtract | OpCode::Multiply | OpCode::Divide => {
                    let a = self.pop();
                    let b = self.pop();
                    let res = match opcode {
                        OpCode::Add => a + b,
                        OpCode::Subtract => a - b,
                        OpCode::Multiply => a * b,
                        OpCode::Divide => a / b,
                        _ => unreachable!(),
                    }
                    .map_err(ErrorKind::Runtime)?;
                    self.push(res);
                }
            }
        }
        Ok(())
    }

    fn push(&mut self, val: Value) {
        self.stack.push(val);
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
