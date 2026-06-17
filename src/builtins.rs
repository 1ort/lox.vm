use std::time::{SystemTime, UNIX_EPOCH};

use crate::value::Value;

pub struct Builtin {
    pub name: &'static str,
    pub value: Value,
}

pub fn get_builtins() -> Vec<Builtin> {
    vec![Builtin {
        name: "clock",
        value: Value::NativeFunction(clock),
    }]
}

fn clock(args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!(
            "Function 'clock' takes 0 arguments, but {} provided",
            args.len()
        ));
    }
    let start = SystemTime::now();
    let millis = start
        .duration_since(UNIX_EPOCH)
        .expect("time should go forward")
        .as_millis() as f64;
    Ok(millis.into())
}
