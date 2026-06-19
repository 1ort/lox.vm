use crate::chunk::Chunk;
use crate::chunk::OpCode;
use crate::value::Value;
use std::fmt::Write;
use std::panic;

pub(super) fn format_chunk(chunk: &Chunk, f: &mut impl Write) {
    let mut offset = 0;

    while offset < chunk.code.len() {
        offset = format_instruction(chunk, offset, f);
        writeln!(f).unwrap();
    }
}

pub fn format_instruction(chunk: &Chunk, offset: usize, f: &mut impl Write) -> usize {
    use OpCode::*;

    let span = &chunk.spans[offset];
    let span = &format!("{span:?}");
    write!(f, "{offset:<04} {span:>8} ").unwrap();

    let instruction: OpCode = chunk.code[offset].into();
    match instruction {
        Constant | DefineGlobal | GetGlobal | SetGlobal => {
            constant_instruction(instruction, chunk, offset, f)
        }
        GetLocal | SetLocal | JumpIfFalse | Jump | Loop | Call => {
            byte_instruction(instruction, chunk, offset, f)
        }
        Closure => {
            let fun_index = get_word(chunk, offset + 1);
            let fun = &chunk.constants[fun_index as usize];
            let Value::Function(fun) = fun else {
                panic!("Can not debug closure: expected function constant. Got: {fun:?}");
            };
            let mut offset = offset + 3;

            let instruction = format!("{instruction:?}");
            writeln!(f, "{instruction:>16} {fun_index:>4} {fun:?}").unwrap();

            for _ in 0..fun.upvalue_count {
                let is_local = chunk.code[offset];
                let index = get_word(chunk, offset + 1);
                write!(f, "{offset:<04} {:>8} ", "|").unwrap();
                offset += 3;
                let upvalue = format!(
                    "{} {index}",
                    if is_local == 1 {
                        "local"
                    } else if is_local == 0 {
                        "upvalue"
                    } else {
                        panic!("unknown upvalue kind {is_local}")
                    }
                );
                writeln!(f, "{:>16} {:>4} {upvalue:?}", "|", "|",).unwrap();
            }

            offset + 3
        }
        _ => simple_instruction(instruction, offset, f),
    }
}

fn simple_instruction(instruction: OpCode, offset: usize, f: &mut impl Write) -> usize {
    let instruction = format!("{instruction:?}");
    write!(f, "{instruction:>16}").unwrap();
    offset + 1
}

fn constant_instruction(
    instruction: OpCode,
    chunk: &Chunk,
    offset: usize,
    f: &mut impl Write,
) -> usize {
    let const_index = get_word(chunk, offset + 1);
    let value = &chunk.constants[const_index as usize];
    let instruction = format!("{instruction:?}");
    write!(f, "{instruction:>16} {const_index:>4} {value}").unwrap();
    offset + 3
}

fn byte_instruction(
    instruction: OpCode,
    chunk: &Chunk,
    offset: usize,
    f: &mut impl Write,
) -> usize {
    let stack_index = get_word(chunk, offset + 1);
    let instruction = format!("{instruction:?}");
    write!(f, "{instruction:>16} {stack_index:>4}").unwrap();
    offset + 3
}

fn get_word(chunk: &Chunk, offset: usize) -> u16 {
    let stack_index_bytes: [u8; 2] = (&chunk.code[(offset)..(offset + 2)]).try_into().unwrap();
    u16::from_le_bytes(stack_index_bytes)
}
