use crate::chunk::Chunk;
use crate::chunk::OpCode;
use std::fmt::Formatter;

pub(super) fn format_chunk(chunk: &Chunk, f: &mut Formatter<'_>) {
    let mut offset = 0;

    while offset < chunk.code.len() {
        offset = format_instruction(chunk, offset, f);
    }
}

pub(super) fn format_instruction(chunk: &Chunk, offset: usize, f: &mut Formatter<'_>) -> usize {
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
        ConstLong => constlong_instruction(instruction, chunk, offset, f),
    }
}

fn simple_instruction(instruction: OpCode, offset: usize, f: &mut Formatter<'_>) -> usize {
    let instruction = format!("{instruction:?}");
    writeln!(f, "{instruction:>16}").unwrap();
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
    writeln!(f, "{instruction:>16} {const_index:>4} {value:?}").unwrap();
    offset + 2
}

fn constlong_instruction(
    instruction: OpCode,
    chunk: &Chunk,
    offset: usize,
    f: &mut Formatter<'_>,
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
