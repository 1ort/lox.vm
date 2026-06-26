use std::{
    cell::RefCell,
    ops::{Add, Div, Mul, Neg, Not, Sub},
    ptr,
    rc::Rc,
};

use crate::chunk::FunctionObject;
use gc::{Gc, Trace};

use Value::*;

#[derive(Debug, Trace)]
pub enum Upvalue {
    Opened(usize), // Absolute stack index
    Closed(Value),
}

#[derive(Clone, Debug, Trace)]
pub struct ClosureObject {
    pub function: Gc<FunctionObject>,
    pub upvalues: Vec<Gc<RefCell<Upvalue>>>,
}

impl ClosureObject {
    pub fn new(func: Gc<FunctionObject>) -> Self {
        ClosureObject {
            upvalues: Vec::with_capacity(func.upvalue_count),
            function: func,
        }
    }
}

#[derive(Clone, Debug, Trace)]
#[non_exhaustive]
pub enum Value {
    Number(f64),
    Str(Rc<str>),
    Bool(bool),
    Nil,
    Function(Gc<FunctionObject>),
    NativeFunction(#[trace(skip)] fn(&[Value]) -> Result<Value, String>),
    Closure(Gc<ClosureObject>),
}

impl Value {
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    pub fn code_object(&self) -> Result<Gc<FunctionObject>, String> {
        match self {
            Function(code_object) => Ok(Gc::clone(code_object)),
            _ => Err("Can only call functions and classes.".into()),
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number(val) => write!(f, "{val}"),
            Str(val) => write!(f, "{}", val.as_ref()),
            Bool(val) => write!(f, "{val}"),
            Nil => write!(f, "nil"),
            Function(func) => {
                let name = func.name.as_ref();
                write!(f, "fun {}", name)
            }
            Closure(closure) => {
                let name = closure.function.name.as_ref();
                write!(f, "fun {}", name)
            }
            NativeFunction(_) => write!(f, "native function"),
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<Rc<str>> for Value {
    fn from(value: Rc<str>) -> Self {
        Self::Str(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Gc<FunctionObject>> for Value {
    fn from(value: Gc<FunctionObject>) -> Self {
        Self::Function(value)
    }
}

impl From<Gc<ClosureObject>> for Value {
    fn from(value: Gc<ClosureObject>) -> Self {
        Self::Closure(value)
    }
}

impl From<Value> for bool {
    fn from(value: Value) -> Self {
        match value {
            Bool(x) => x,
            Nil => false,
            _ => true,
        }
    }
}

impl From<&Value> for bool {
    fn from(value: &Value) -> Self {
        match value {
            Bool(x) => *x,
            Nil => false,
            _ => true,
        }
    }
}

impl Neg for Value {
    type Output = Result<Self, String>;

    fn neg(self) -> Self::Output {
        match self {
            Number(num) => Ok(Number(-num)),
            _ => Err("Only number can be negated".into()),
        }
    }
}

impl Add for Value {
    type Output = Result<Self, String>;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Number(a), Number(b)) => Ok((a + b).into()),
            _ => Err("Only numbers can be added.".into()),
        }
    }
}

impl Sub for Value {
    type Output = Result<Self, String>;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Number(a), Number(b)) => Ok(Number(a - b)),
            _ => Err("Only two numbers can be subtracted.".into()),
        }
    }
}

impl Mul for Value {
    type Output = Result<Self, String>;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Number(a), Number(b)) => Ok(Number(a * b)),
            _ => Err("Only two numbers can be multiplied.".into()),
        }
    }
}

impl Div for Value {
    type Output = Result<Self, String>;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Number(a), Number(b)) => {
                if b == 0.0 {
                    Err("Division by zero.".into())
                } else {
                    Ok(Number(a / b))
                }
            }
            _ => Err("Only two numbers can be divided.".into()),
        }
    }
}

impl Not for Value {
    type Output = Result<Self, String>;

    fn not(self) -> Self::Output {
        let x: bool = self.into();
        Ok(Bool(!x))
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(l0), Self::Number(r0)) => l0 == r0,
            (Self::Str(l0), Self::Str(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            (Self::Function(this), Self::Function(other)) => Gc::ptr_eq(this, other),
            (Self::Closure(this), Self::Closure(other)) => Gc::ptr_eq(this, other),
            (Self::NativeFunction(this), Self::NativeFunction(other)) => {
                ptr::fn_addr_eq(*this, *other)
            }
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Number(a), Number(b)) => a.partial_cmp(b),
            (Str(a), Str(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}
