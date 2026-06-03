use crate::chunk::OpCode;

mod chunk;

fn main() {
    use chunk::Chunk;
    let mut chunk = Chunk::new();
    let const_index = chunk.add_constant(1.5) as u8;

    chunk.add_code(OpCode::Constant, 123);
    chunk.add_code(const_index, 123);
    chunk.add_code(OpCode::Return, 124);

    println!("{chunk}");
}
