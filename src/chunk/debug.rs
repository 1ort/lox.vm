use crate::chunk::Chunk;
use crate::chunk::OpCode;
use std::fmt::Formatter;

pub(super) fn format_chunk(chunk: &Chunk, f: &mut Formatter<'_>) {
    for offset in 0..chunk.code.len() {
        format_instruction(chunk, offset, f);
    }
}

pub(super) fn format_instruction(chunk: &Chunk, offset: usize, f: &mut Formatter<'_>) {
    use OpCode::*;

    let line = if offset == 0 || chunk.lines[offset] > chunk.lines[offset - 1] {
        let l = chunk.lines[offset];
        &format!("{l}")
    } else {
        "|"
    };
    write!(f, "{offset:<04} {line:>4} ").unwrap();

    let instruction: OpCode = chunk.code[offset].into();
    match instruction {
        Pass | Return => simple_instruction(instruction, offset, f),
        Constant => constant_instruction(instruction, chunk, offset, f),
    };
}

fn simple_instruction(instruction: OpCode, offset: usize, f: &mut Formatter<'_>) -> usize {
    writeln!(f, "{instruction:?}").unwrap();
    offset + 1
}

fn constant_instruction(
    instruction: OpCode,
    chunk: &Chunk,
    offset: usize,
    f: &mut Formatter<'_>,
) -> usize {
    let const_index = chunk.code[offset + 1];
    let value = &chunk.constants[const_index as usize];

    let instruction = format!("{instruction:?}");

    writeln!(f, "{instruction:<8} {const_index:>4} {value:?}").unwrap();
    offset + 2
}
