use crate::{
    chunk::{Chunk, FunctionObject, debug::format_instruction},
    interner::Interner,
    opcode::OpCode,
    value::Value,
};
use std::{collections::HashMap, rc::Rc};

const STACK_MAX: usize = u16::MAX as usize;
const FRAMES_MAX: usize = u16::MAX as usize;

#[derive(Debug)]
#[expect(unused)]
pub struct RuntimeError(String); // TODO:: add stacktrace

impl From<String> for RuntimeError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

struct CallFrame {
    //function: &'a FunctionObject,
    // instruction pointer
    ip: usize,
    // frame pointer
    fp: usize,
}

pub struct VM {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    globals: HashMap<Rc<str>, Value>,
    interner: Interner,
}

impl<'a> VM {
    pub fn new(interner: Interner) -> Self {
        let mut this = Self {
            stack: Vec::with_capacity(STACK_MAX),
            frames: Vec::with_capacity(FRAMES_MAX),
            globals: HashMap::new(),
            interner,
        };
        this.enter_frame(0);
        this
    }

    pub(super) fn borrow_interner(&mut self) -> &mut Interner {
        &mut self.interner
    }

    fn frame(&self) -> &CallFrame {
        self.frames
            .last()
            .expect("at least one stackframe should present")
    }

    fn frame_mut(&mut self) -> &mut CallFrame {
        self.frames
            .last_mut()
            .expect("at least one stackframe should present")
    }

    fn enter_frame(&mut self, offset: usize) {
        self.frames.push(CallFrame {
            ip: 0,
            fp: self.stack.len() - offset,
        });
    }

    fn exit_frame(&mut self) {
        let frame = self
            .frames
            .pop()
            .expect("At least one stack frame should exist");
        debug_assert_eq!(self.stack.len(), frame.fp);
        //self.stack.truncate(frame.fp);
    }

    fn next_byte(&mut self, bytes: &[u8]) -> u8 {
        let byte = bytes[self.frame().ip];
        self.frame_mut().ip += 1;
        byte
    }

    fn next_word(&mut self, bytes: &[u8]) -> u16 {
        u16::from_ne_bytes([self.next_byte(bytes), self.next_byte(bytes)])
    }

    fn push_stack(&mut self, val: impl Into<Value>) {
        self.stack.push(val.into());
    }

    fn peek_stack(&self, offset: u16) -> &Value {
        let index = self.stack.len() - offset as usize;
        &self.stack[index]
    }

    fn pop_stack(&mut self) -> Value {
        self.stack.pop().expect("Attempt to pop empty stack")
    }

    fn read_stack(&self, index: u16) -> &Value {
        &self.stack[self.frame().fp + index as usize]
    }

    fn set_stack(&mut self, index: u16, value: Value) {
        let index = self.frame().fp + index as usize;
        self.stack[index] = value;
    }

    fn read_const(&self, chunk: &'a Chunk, index: u16) -> &'a Value {
        &chunk.constants[index as usize]
    }

    fn call(
        &mut self,
        code_object: Rc<FunctionObject>,
        arg_count: u16,
    ) -> Result<(), RuntimeError> {
        if arg_count != code_object.arity as u16 {
            return Err(format!(
                "Expected {} arguments but got {}.",
                code_object.arity, arg_count
            )
            .into());
        }
        if self.frames.len() >= FRAMES_MAX {
            return Err("Stack overflow.".to_string().into());
        }

        self.enter_frame(arg_count as usize + 1);
        let value = self.run(code_object.as_ref())?;
        self.push_stack(value);

        self.exit_frame();
        Ok(())
    }

    pub fn run(&mut self, function_object: &FunctionObject) -> Result<Value, RuntimeError> {
        let chunk = &function_object.chunk;
        let bytes: &[u8] = &chunk.code;
        if cfg!(feature = "debug_vm") {
            println!("===chunk===");
            print!("{}", chunk);
            println!("===========");
        }

        loop {
            if self.frame().ip >= bytes.len() {
                if cfg!(feature = "debug_vm") {
                    println!("{:?}", &self.stack);
                }
                break;
            }

            if cfg!(feature = "debug_vm") {
                let mut buff = String::new();
                format_instruction(chunk, self.frame().ip, &mut buff);
                let stack = &self.stack;
                println!("{buff:<60} | {:?}", &stack);
            }

            let opcode: OpCode = self.next_byte(bytes).into();
            match opcode {
                OpCode::Pass => {}
                OpCode::Constant => {
                    let index = self.next_word(bytes);
                    let val = self.read_const(chunk, index).clone();
                    self.push_stack(val);
                }
                OpCode::Return => {
                    let val = self.pop_stack();
                    return Ok(val);
                }
                OpCode::Negate => {
                    let val = (-self.pop_stack()).map_err(RuntimeError)?;
                    self.push_stack(val);
                }
                OpCode::Not => {
                    let val = (!self.pop_stack()).map_err(RuntimeError)?;
                    self.push_stack(val);
                }
                OpCode::Add | OpCode::Subtract | OpCode::Multiply | OpCode::Divide => {
                    let a = self.pop_stack();
                    let b = self.pop_stack();
                    let res = match opcode {
                        OpCode::Add => match (&a, &b) {
                            (Value::Str(a), Value::Str(b)) => {
                                let concatenated = format!("{b}{a}");
                                Ok(self.interner.intern(&concatenated).into())
                            }
                            _ => a + b,
                        },
                        OpCode::Subtract => b - a,
                        OpCode::Multiply => b * a,
                        OpCode::Divide => b / a,
                        _ => unreachable!(),
                    }
                    .map_err(RuntimeError)?;
                    self.push_stack(res);
                }
                OpCode::True => self.push_stack(true),
                OpCode::False => self.push_stack(false),
                OpCode::Nil => self.push_stack(Value::Nil),
                OpCode::Equal | OpCode::Greater | OpCode::Less => {
                    let a = self.pop_stack();
                    let b = self.pop_stack();
                    let res = match opcode {
                        OpCode::Equal => b == a,
                        OpCode::Greater => b > a,
                        OpCode::Less => b < a,
                        _ => unreachable!(),
                    };
                    self.push_stack(res);
                }
                OpCode::Print => {
                    let value = self.pop_stack();
                    println!("{value}");
                }
                OpCode::Pop => {
                    self.pop_stack();
                }
                OpCode::DefineGlobal => {
                    let index = self.next_word(bytes);
                    let name = self.read_const(chunk, index);
                    let Value::Str(identifier) = name else {
                        panic!("Expect identifier to be Str")
                    };
                    let value = self.pop_stack();
                    self.globals.insert(Rc::clone(identifier), value);
                }
                OpCode::GetGlobal => {
                    let index = self.next_word(bytes);
                    let name = self.read_const(chunk, index);
                    let Value::Str(identifier) = name else {
                        panic!("Expect identifier to be Str")
                    };
                    let global = self.globals.get(identifier);
                    let Some(value) = global else {
                        return Err(RuntimeError(format!("Undefined variable {identifier}")));
                    };
                    self.push_stack(value.clone());
                }
                OpCode::SetGlobal => {
                    let index = self.next_word(bytes);
                    let name = self.read_const(chunk, index);
                    let Value::Str(identifier) = name else {
                        panic!("Expect identifier to be Str")
                    };
                    let value = self.peek_stack(0);
                    if !self.globals.contains_key(identifier) {
                        return Err(RuntimeError(format!("Undefined variable {identifier}")));
                    }
                    self.globals.insert(Rc::clone(identifier), value.clone());
                }
                OpCode::GetLocal => {
                    let index = self.next_word(bytes);
                    let value = self.read_stack(index);
                    self.push_stack(value.clone());
                }
                OpCode::SetLocal => {
                    let index = self.next_word(bytes);
                    let value = self.peek_stack(0);
                    self.set_stack(index, value.clone());
                }
                OpCode::JumpIfFalse => {
                    let offset = self.next_word(bytes);
                    let cond: bool = self.peek_stack(0).into();
                    if !cond {
                        self.frame_mut().ip += offset as usize;
                    }
                }
                OpCode::Jump => {
                    let offset = self.next_word(bytes);
                    self.frame_mut().ip += offset as usize;
                }
                OpCode::Loop => {
                    let offset = self.next_word(bytes);
                    self.frame_mut().ip -= offset as usize;
                }
                OpCode::Call => {
                    let arg_count = self.next_word(bytes);
                    let callable = self.peek_stack(arg_count);
                    let code_object = callable.code_object().map_err(RuntimeError)?;
                    self.call(code_object, arg_count)?;
                }
            }
        }
        Ok(Value::Nil)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::Chunk;
    use crate::value::Value;

    fn run(chunk: Chunk) -> Result<Value, RuntimeError> {
        let interner = Interner::new();
        let mut vm = VM::new(interner);
        let code_object = FunctionObject {
            chunk,
            arity: 0,
            name: Rc::from(""),
        };
        vm.run(&code_object)
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
        assert!(run(chunk).is_ok_and(|x| x == Value::Nil));
    }

    #[test]
    fn test_constant() {
        let chunk = chunk_with_constant(42.);
        assert!(run(chunk).is_ok_and(|x| x == Value::Number(42.)));
    }

    #[test]
    fn test_constant_long() {
        let mut chunk = Chunk::new();
        chunk.add_const_code(OpCode::Constant, 20., 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        assert!(run(chunk).is_ok_and(|x| x == Value::Number(20.)));
    }

    #[test]
    fn test_addition() {
        let chunk = chunk_with_binary_op(5.0, 3.0, OpCode::Add);
        assert!(run(chunk).is_ok_and(|x| x == Value::Number(8.)));
    }

    #[test]
    fn test_negate_operator() {
        let mut chunk = Chunk::new();
        chunk.add_const_code(OpCode::Constant, 10., 0..0);
        chunk.add_code(OpCode::Negate, 0..0);
        chunk.add_code(OpCode::Return, 0..0);
        assert!(run(chunk).is_ok_and(|x| x == Value::Number(-10.)));
    }

    #[test]
    fn test_multiplication() {
        let chunk = chunk_with_binary_op(2., 4.0, OpCode::Multiply);
        assert!(run(chunk).is_ok_and(|x| x == Value::Number(8.)));
    }

    #[test]
    fn test_division() {
        let chunk = chunk_with_binary_op(16., 4., OpCode::Divide);
        assert!(run(chunk).is_ok_and(|x| x == Value::Number(4.)));
    }

    #[test]
    fn test_division_by_zero() {
        let chunk = chunk_with_binary_op(16., 0., OpCode::Divide);
        assert!(run(chunk).is_err_and(
            |err| matches!(err, RuntimeError(err) if err.eq("Division by zero.")
            )
        ))
    }

    #[test]
    fn test_subtraction() {
        let chunk = chunk_with_binary_op(16., 4., OpCode::Subtract);
        assert!(run(chunk).is_ok_and(|x| x == Value::Number(12.)));
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

        assert!(run(chunk).is_ok_and(|x| x == Value::Number(700.)));
    }
}
