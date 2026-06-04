use std::ops::Neg;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl Neg for Value {
    type Output = Result<Self, String>;

    fn neg(self) -> Self::Output {
        match self {
            Value::Number(num) => Ok(Value::Number(num.neg())),
            _ => Err("Invalid negative target".into()),
        }
    }
}
