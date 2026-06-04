use chunk::Chunk;
use opcode::OpCode;

mod chunk;
mod opcode;
mod value;

fn main() {
    let mut chunk = Chunk::new();
    for i in 0..350 {
        chunk.add_constant(f64::from((i * 3) as u32), i);
    }
    chunk.add_code(OpCode::Return, 351);

    println!("{chunk}");
}
