use crate::chunk::Chunk;
use crate::chunk::OpCode;
use std::fmt::Write;

pub(super) fn format_chunk(chunk: &Chunk, f: &mut impl Write) {
    let mut offset = 0;

    while offset < chunk.code.len() {
        offset = format_instruction(chunk, offset, f);
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
        GetLocal | SetLocal => byte_instruction(instruction, chunk, offset, f),
        _ => simple_instruction(instruction, offset, f),
    }
}

fn simple_instruction(instruction: OpCode, offset: usize, f: &mut impl Write) -> usize {
    let instruction = format!("{instruction:?}");
    writeln!(f, "{instruction:>16}").unwrap();
    offset + 1
}

fn constant_instruction(
    instruction: OpCode,
    chunk: &Chunk,
    offset: usize,
    f: &mut impl Write,
) -> usize {
    let const_index_bytes: [u8; 2] = (&chunk.code[(offset + 1)..(offset + 3)])
        .try_into()
        .unwrap();

    let const_index = u16::from_le_bytes(const_index_bytes);
    let value = &chunk.constants[const_index as usize];
    let instruction = format!("{instruction:?}");
    writeln!(f, "{instruction:>16} {const_index:>4} {value:?}").unwrap();
    offset + 3
}

fn byte_instruction(
    instruction: OpCode,
    chunk: &Chunk,
    offset: usize,
    f: &mut impl Write,
) -> usize {
    let stack_index_bytes: [u8; 2] = (&chunk.code[(offset + 1)..(offset + 3)])
        .try_into()
        .unwrap();
    let stack_index = u16::from_le_bytes(stack_index_bytes);
    let instruction = format!("{instruction:?}");
    writeln!(f, "{instruction:>16} {stack_index:>4}").unwrap();
    offset + 3
}
