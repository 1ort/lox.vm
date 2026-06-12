use crate::{chunk::Chunk, interner::Interner, opcode::OpCode, value::Value};
use std::{collections::HashMap, rc::Rc};

const STACK_MAX: usize = 256;

#[derive(Debug)]
#[expect(unused)]
pub struct RuntimeError(String);

pub struct VM {
    stack: Vec<Value>, // TODO: store &Value directly to top of stack
    ip: usize,
    globals: HashMap<Rc<str>, Value>,
}

impl Default for VM {
    fn default() -> Self {
        Self {
            stack: Vec::with_capacity(STACK_MAX),
            globals: HashMap::new(),
            ip: 0,
        }
    }
}

impl<'a> VM {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.stack = Vec::with_capacity(STACK_MAX);
        self.ip = 0;
    }

    fn next(&mut self, bytes: &[u8]) -> u8 {
        let byte = bytes[self.ip];
        self.ip += 1;
        byte
    }

    fn next_double(&mut self, bytes: &[u8]) -> u16 {
        u16::from_ne_bytes([self.next(bytes), self.next(bytes)])
    }

    pub fn run(&mut self, chunk: &Chunk, interner: &mut Interner) -> Result<Value, RuntimeError> {
        self.reset();
        let bytes: &[u8] = &chunk.code;

        if cfg!(feature = "debug_vm") {
            println!("===chunk===");
            print!("{}", chunk);
            println!("===========");
        }

        loop {
            if self.ip >= bytes.len() {
                break;
            }
            let opcode: OpCode = self.next(bytes).into();
            match opcode {
                OpCode::Pass => {}
                OpCode::Constant => {
                    let index = self.next_double(bytes);
                    let val = self.read_const(chunk, index).clone();
                    self.push(val);
                }
                OpCode::Return => {
                    let val = self.pop();
                    return Ok(val);
                }
                OpCode::Negate => {
                    let val = (-self.pop()).map_err(RuntimeError)?;
                    self.push(val);
                }
                OpCode::Not => {
                    let val = (!self.pop()).map_err(RuntimeError)?;
                    self.push(val);
                }
                OpCode::Add | OpCode::Subtract | OpCode::Multiply | OpCode::Divide => {
                    let a = self.pop();
                    let b = self.pop();
                    let res = match opcode {
                        OpCode::Add => match (&a, &b) {
                            (Value::Str(a), Value::Str(b)) => {
                                let concatenated = format!("{b}{a}");
                                Ok(interner.intern(&concatenated).into())
                            }
                            _ => a + b,
                        },
                        OpCode::Subtract => b - a,
                        OpCode::Multiply => b * a,
                        OpCode::Divide => b / a,
                        _ => unreachable!(),
                    }
                    .map_err(RuntimeError)?;
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
                OpCode::Print => {
                    let value = self.pop();
                    println!("{value}");
                }
                OpCode::Pop => {
                    self.pop();
                }
                OpCode::DefineGlobal => {
                    let index = self.next_double(bytes);
                    let name = self.read_const(chunk, index);
                    let Value::Str(identifier) = name else {
                        panic!("Expect identifier to be Str")
                    };
                    let value = self.pop();
                    self.globals.insert(Rc::clone(identifier), value);
                }
                OpCode::GetGlobal => {
                    let index = self.next_double(bytes);
                    let name = self.read_const(chunk, index);
                    let Value::Str(identifier) = name else {
                        panic!("Expect identifier to be Str")
                    };
                    let global = self.globals.get(identifier);
                    let Some(value) = global else {
                        return Err(RuntimeError(format!("Undefined variable {identifier}")));
                    };
                    self.push(value.clone());
                }
                OpCode::SetGlobal => {
                    let index = self.next_double(bytes);
                    let name = self.read_const(chunk, index);
                    let Value::Str(identifier) = name else {
                        panic!("Expect identifier to be Str")
                    };
                    let value = self.peek();
                    if !self.globals.contains_key(identifier) {
                        return Err(RuntimeError(format!("Undefined variable {identifier}")));
                    }
                    self.globals.insert(Rc::clone(identifier), value.clone());
                }
                OpCode::GetLocal => {
                    let index = self.next_double(bytes);
                    let value = self.value_at_index(index);
                    self.push(value.clone());
                }
                OpCode::SetLocal => {
                    let index = self.next_double(bytes);
                    let value = self.peek();
                    self.stack[index as usize] = value.clone();
                }
            }

            if cfg!(feature = "debug_vm") {
                let stack = &self.stack;
                println!("{:?}", &stack);
            }
        }
        Ok(Value::Nil)
    }

    fn push(&mut self, val: impl Into<Value>) {
        self.stack.push(val.into());
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Attempt to pop empty stack")
    }

    fn peek(&self) -> &Value {
        self.stack.last().expect("Attempt to peek empty stack")
    }

    fn read_const(&self, chunk: &'a Chunk, index: u16) -> &'a Value {
        &chunk.constants[index as usize]
    }

    fn value_at_index(&self, index: u16) -> &Value {
        &self.stack[index as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Chunk;
    use crate::value::Value;

    fn run(chunk: &Chunk) -> Result<Value, RuntimeError> {
        let mut vm = VM::new();
        vm.run(chunk, &mut Interner::new())
    }

    fn chunk_with_constant(val: impl Into<Value>) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.add_const_code(OpCode::Constant, val, 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        chunk
    }

    fn chunk_with_binary_op(a: impl Into<Value>, b: impl Into<Value>, op: OpCode) -> Chunk {
        let mut chunk = Chunk::new();
        chunk.add_const_code(OpCode::Constant, a, 0..0);
        chunk.add_const_code(OpCode::Constant, b, 0..0);
        chunk.add_code(op as u8, 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        chunk
    }

    #[test]
    fn test_empty_chunk() {
        let chunk = Chunk::new();
        assert!(run(&chunk).is_ok_and(|x| x == Value::Nil));
    }

    #[test]
    fn test_constant() {
        let chunk = chunk_with_constant(42.);
        assert!(run(&chunk).is_ok_and(|x| x == Value::Number(42.)));
    }

    #[test]
    fn test_constant_long() {
        let mut chunk = Chunk::new();
        chunk.add_const_code(OpCode::Constant, 20., 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        assert!(run(&chunk).is_ok_and(|x| x == Value::Number(20.)));
    }

    #[test]
    fn test_addition() {
        let chunk = chunk_with_binary_op(5.0, 3.0, OpCode::Add);
        assert!(run(&chunk).is_ok_and(|x| x == Value::Number(8.)));
    }

    #[test]
    fn test_negate_operator() {
        let mut chunk = Chunk::new();
        chunk.add_const_code(OpCode::Constant, 10., 0..0);
        chunk.add_code(OpCode::Negate, 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        assert!(run(&chunk).is_ok_and(|x| x == Value::Number(-10.)));
    }

    #[test]
    fn test_multiplication() {
        let chunk = chunk_with_binary_op(2., 4.0, OpCode::Multiply);
        assert!(run(&chunk).is_ok_and(|x| x == Value::Number(8.)));
    }

    #[test]
    fn test_division() {
        let chunk = chunk_with_binary_op(16., 4., OpCode::Divide);
        assert!(run(&chunk).is_ok_and(|x| x == Value::Number(4.)));
    }

    #[test]
    fn test_division_by_zero() {
        let chunk = chunk_with_binary_op(16., 0., OpCode::Divide);
        assert!(run(&chunk).is_err_and(
            |err| matches!(err, RuntimeError(err) if err.eq("Division by zero.")
            )
        ))
    }

    #[test]
    fn test_subtraction() {
        let chunk = chunk_with_binary_op(16., 4., OpCode::Subtract);
        assert!(run(&chunk).is_ok_and(|x| x == Value::Number(12.)));
    }

    #[test]
    fn test_multiple_operations() {
        let mut chunk = Chunk::new();
        let span = 0..1;
        chunk.add_const_code(OpCode::Constant, 5., span.clone());

        chunk.add_const_code(OpCode::Constant, 10., span.clone());
        chunk.add_const_code(OpCode::Constant, 9., span.clone());
        chunk.add_code(OpCode::Subtract, span.clone());
        // 10 - 9 = 1
        chunk.add_const_code(OpCode::Constant, 3., span.clone());
        chunk.add_const_code(OpCode::Constant, 4., span.clone());
        chunk.add_code(OpCode::Add, span.clone());
        // 4 + 3 = 7
        chunk.add_const_code(OpCode::Constant, 20., span.clone());
        chunk.add_code(OpCode::Multiply, span.clone());
        // 20 * 7 = 140
        chunk.add_code(OpCode::Divide, span.clone());
        // 1/140
        chunk.add_code(OpCode::Divide, span.clone());
        // 5 / (1/140) == 700
        chunk.add_code(OpCode::Return, span.clone());

        assert!(run(&chunk).is_ok_and(|x| x == Value::Number(700.)));
    }
}
