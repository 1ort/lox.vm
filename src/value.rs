use std::ops::{Add, Div, Mul, Neg, Not, Sub};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Str(String),
    Bool(bool),
    Nil,
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Str(value)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<Value> for bool {
    fn from(value: Value) -> Self {
        match value {
            Number(_) => true,
            Str(_) => true,
            Bool(x) => x,
            Nil => false,
        }
    }
}

use Value::*;

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
            (Number(a), Number(b)) => Ok(Number(a + b)),
            (Str(a), Str(b)) => Ok(format!("{a}{b}").into()),
            _ => Err("Only numbers or strings can be added.".into()),
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
