use gc::{Gc, GcHeap};

use crate::{
    builtins::Builtin,
    chunk::{FunctionObject, debug::format_instruction},
    interner::Interner,
    opcode::OpCode,
    value::{self, ClosureObject, Upvalue, Value},
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

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

#[derive(Debug)]
struct CallFrame {
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
    heap: GcHeap,
    open_upvalues: BTreeMap<usize, Gc<RefCell<Upvalue>>>,
}

impl<'a> VM {
    pub fn new(interner: Interner, heap: GcHeap) -> Self {
        Self {
            stack: Vec::with_capacity(STACK_MAX),
            frames: Vec::with_capacity(FRAMES_MAX),
            globals: HashMap::new(),
            interner,
            heap,
            open_upvalues: BTreeMap::new(),
        }
    }

    pub fn add_builtins(&mut self, builtins: Vec<Builtin>) {
        for builtin in builtins {
            self.globals
                .insert(self.interner.intern(builtin.name), builtin.value);
        }
    }

    pub(super) fn borrow_interner_and_heap(&mut self) -> (&mut Interner, &mut GcHeap) {
        (&mut self.interner, &mut self.heap)
    }

    fn reset(&mut self) {
        self.stack.truncate(0);
        self.frames.truncate(0);
        self.open_upvalues.clear();
        self.enter_frame(0);
    }

    #[inline(always)]
    fn ip(&self) -> usize {
        self.frame().ip
    }

    #[inline(always)]
    fn frame(&self) -> &CallFrame {
        self.frames
            .last()
            .expect("at least one stackframe should present")
    }

    #[inline(always)]
    fn frame_mut(&mut self) -> &mut CallFrame {
        self.frames
            .last_mut()
            .expect("at least one stackframe should present")
    }

    #[inline(always)]
    fn enter_frame(&mut self, offset: usize) {
        self.frames.push(CallFrame {
            ip: 0,
            fp: self.stack.len() - offset,
        });
    }

    #[inline(always)]
    fn current_closure(&self) -> &ClosureObject {
        let Value::Closure(closure) = &self.stack[self.frame().fp] else {
            panic!("Expect ClosureObject at the bottom of stack")
        };
        closure
    }

    #[inline(always)]
    fn current_bytes(&self) -> &[u8] {
        &self.current_closure().function.chunk.code
    }

    #[inline(always)]
    fn next_byte(&mut self) -> u8 {
        let bytes = self.current_bytes();
        let byte = bytes[self.frame().ip];
        self.frame_mut().ip += 1;
        byte
    }

    #[inline(always)]
    fn next_word(&mut self) -> u16 {
        u16::from_ne_bytes([self.next_byte(), self.next_byte()])
    }

    #[inline(always)]
    fn push_stack(&mut self, val: impl Into<Value>) {
        self.stack.push(val.into());
    }

    #[inline(always)]
    fn peek_stack(&self, offset: u16) -> &Value {
        let index = self.stack.len() - 1 - offset as usize;
        &self.stack[index]
    }

    #[inline(always)]
    fn pop_stack(&mut self) -> Value {
        self.stack.pop().expect("Attempt to pop empty stack")
    }

    #[inline(always)]
    fn read_stack(&self, index: u16) -> &Value {
        &self.stack[self.frame().fp + index as usize]
    }

    #[inline(always)]
    fn set_stack(&mut self, index: u16, value: Value) {
        let index = self.frame().fp + index as usize;
        self.stack[index] = value;
    }

    #[inline(always)]
    fn read_const(&'a self, index: u16) -> &'a Value {
        let chunk = &self.current_closure().function.chunk;
        &chunk.constants[index as usize]
    }

    #[inline(always)]
    fn close_upvalues(&mut self, first_local_index: usize) {
        let upvalues_to_close = self.open_upvalues.split_off(&first_local_index);
        for upvalue in upvalues_to_close.into_values() {
            let value = match *upvalue.borrow() {
                Upvalue::Opened(index) => self.stack[index].clone(),
                Upvalue::Closed(_) => unreachable!(),
            };
            upvalue.replace(Upvalue::Closed(value));
        }
    }

    #[inline(always)]
    fn debug_chunk(&self) {
        if cfg!(feature = "debug_vm") {
            let closure = self.current_closure();
            let chunk = &closure.function.chunk;
            println!("==={}===", closure.function.name);
            print!("{}", chunk);
            println!("===========");
        }
    }

    pub fn interpret(&mut self, func: FunctionObject) -> Result<Value, RuntimeError> {
        self.reset();
        let gc_func = self.heap.alloc(func);
        let closure = self.heap.alloc(ClosureObject::new(gc_func));
        self.push_stack(closure);
        self.debug_chunk();
        self.run()
    }

    fn run(&mut self) -> Result<Value, RuntimeError> {
        loop {
            let bytes = self.current_bytes();

            if self.ip() >= bytes.len() {
                if cfg!(feature = "debug_vm") {
                    print!("[");
                    for val in &self.stack[self.frame().fp..] {
                        print!("{}, ", val);
                    }
                    println!("]");
                }
                break;
            }

            if cfg!(feature = "debug_vm") {
                let chunk = &self.current_closure().function.chunk;
                let mut buff = String::new();
                format_instruction(chunk, self.ip(), &mut buff);
                print!("{buff:<60} | ");
                print!("[");
                for val in &self.stack[self.frame().fp..] {
                    print!("{}, ", val);
                }
                println!("]");
            }

            let opcode: OpCode = self.next_byte().into();
            match opcode {
                OpCode::Pass => {}
                OpCode::Constant => {
                    let index = self.next_word();
                    let val = self.read_const(index).clone();
                    self.push_stack(val);
                }
                OpCode::Return => {
                    let value = self.pop_stack();
                    let frame = self
                        .frames
                        .pop()
                        .expect("At least one stack frame should present");

                    if self.frames.is_empty() {
                        return Ok(value);
                    }
                    self.close_upvalues(frame.fp);
                    self.stack.truncate(frame.fp);
                    self.push_stack(value);
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
                    let index = self.next_word();
                    let name = {
                        let name = self.read_const(index);
                        let Value::Str(identifier) = name else {
                            panic!("Expect identifier to be Str")
                        };
                        Rc::clone(identifier)
                    };
                    let value = self.pop_stack();
                    self.globals.insert(name, value);
                }
                OpCode::GetGlobal => {
                    let index = self.next_word();
                    let name = self.read_const(index);
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
                    let index = self.next_word();
                    let name = self.read_const(index);
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
                    let index = self.next_word();
                    let value = self.read_stack(index);
                    self.push_stack(value.clone());
                }
                OpCode::SetLocal => {
                    let index = self.next_word();
                    let value = self.peek_stack(0);
                    self.set_stack(index, value.clone());
                }
                OpCode::JumpIfFalse => {
                    let offset = self.next_word();
                    let cond: bool = self.peek_stack(0).into();
                    if !cond {
                        self.frame_mut().ip += offset as usize;
                    }
                }
                OpCode::Jump => {
                    let offset = self.next_word();
                    self.frame_mut().ip += offset as usize;
                }
                OpCode::Loop => {
                    let offset = self.next_word();
                    self.frame_mut().ip -= offset as usize;
                }
                OpCode::Call => {
                    let arg_count = self.next_word();
                    match self.peek_stack(arg_count) {
                        Value::Closure(closure) => {
                            let code_object = &closure.function;
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
                            self.debug_chunk();
                        }
                        Value::NativeFunction(callable) => {
                            let offset = self.stack.len() - arg_count as usize;
                            let args = &self.stack[(offset)..];
                            let result = callable(args).map_err(RuntimeError)?;
                            self.stack.truncate(offset - 1); // also remove callable from stack
                            self.push_stack(result);
                        }
                        _ => {
                            return Err(RuntimeError(
                                "Can only call functions and classes.".into(),
                            ));
                        }
                    }
                }
                OpCode::GetUpvalue => {
                    let upvalue_index = self.next_word();
                    let upvalue_rc = &self.current_closure().upvalues[upvalue_index as usize];
                    let value = match *upvalue_rc.borrow() {
                        Upvalue::Opened(abs_stack_index) => self.stack[abs_stack_index].clone(),
                        Upvalue::Closed(ref value) => value.clone(),
                    };
                    self.push_stack(value);
                }
                OpCode::SetUpvalue => {
                    let upvalue_index = self.next_word();
                    let new_value = self.peek_stack(0).clone();
                    let upvalue_rc = &self.current_closure().upvalues[upvalue_index as usize];
                    let upvalue_rc = Gc::clone(upvalue_rc);

                    let t = match *upvalue_rc.borrow() {
                        Upvalue::Opened(index) => {
                            self.stack[index] = new_value;
                            Upvalue::Opened(index)
                        }
                        Upvalue::Closed(_) => Upvalue::Closed(new_value),
                    };
                    upvalue_rc.replace(t);
                }
                OpCode::Closure => {
                    let fun_index = self.next_word();
                    let value = self.read_const(fun_index);
                    let Value::Function(fun) = value else {
                        panic!("Can't build closure: invalid function object index");
                    };
                    let mut closure = ClosureObject::new(Gc::clone(fun));
                    for _ in 0..fun.upvalue_count {
                        let is_local = self.next_byte();
                        let index = self.next_word();
                        let upvalue = if is_local == 1 {
                            // all upvalues pointing to same stack index should be allocated on heap
                            let abs_stack_index = self.frame().fp + index as usize;
                            self.open_upvalues
                                .entry(abs_stack_index)
                                .or_insert_with(|| {
                                    self.heap
                                        .alloc(RefCell::new(Upvalue::Opened(abs_stack_index)))
                                })
                        } else {
                            &self.current_closure().upvalues[index as usize]
                        };
                        closure.upvalues.push(Gc::clone(upvalue));
                    }
                    let closure = self.heap.alloc(closure);
                    self.push_stack(closure);
                }
                OpCode::CloseUpvalue => {
                    let abs_stack_index = self.stack.len() - 1;
                    self.close_upvalues(abs_stack_index);
                    self.pop_stack();
                }
            }
        }
        Ok(Value::Nil)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{Chunk, FunctionObject};
    use crate::value::Value;

    fn run(chunk: Chunk) -> Result<Value, RuntimeError> {
        let interner = Interner::new();
        let heap = GcHeap::new();
        let mut vm = VM::new(interner, heap);
        let mut code_object = FunctionObject::new(&Rc::from(""));
        code_object.chunk = chunk;
        vm.interpret(code_object)
    }

    fn chunk_with_constant(val: impl Into<Value>) -> Chunk {
        let mut chunk = Chunk::new();
        {
            let index = chunk.push_constant(val);
            chunk.add_byte(OpCode::Constant, 0..0);
            chunk.add_word(index, 0..0);
        }
        chunk.add_byte(OpCode::Return, 0..0);
        chunk
    }

    fn chunk_with_binary_op(a: impl Into<Value>, b: impl Into<Value>, op: OpCode) -> Chunk {
        let mut chunk = Chunk::new();
        {
            let index = chunk.push_constant(a);
            chunk.add_byte(OpCode::Constant, 0..0);
            chunk.add_word(index, 0..0);
        }
        {
            let index = chunk.push_constant(b);
            chunk.add_byte(OpCode::Constant, 0..0);
            chunk.add_word(index, 0..0);
        }
        chunk.add_byte(op, 0..0);
        chunk.add_byte(OpCode::Return, 0..0);
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
    fn test_addition() {
        let chunk = chunk_with_binary_op(5.0, 3.0, OpCode::Add);
        assert!(run(chunk).is_ok_and(|x| x == Value::Number(8.)));
    }

    #[test]
    fn test_negate_operator() {
        let mut chunk = Chunk::new();

        {
            let index = chunk.push_constant(10.0);
            chunk.add_byte(OpCode::Constant, 0..0);
            chunk.add_word(index, 0..0);
        }

        chunk.add_byte(OpCode::Negate, 0..0);
        chunk.add_byte(OpCode::Return, 0..0);
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
}
