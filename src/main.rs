use crate::vm::VM;

mod chunk;
mod compiler;
mod opcode;
mod value;
mod vm;

fn main() {
    use chunk::Chunk;
    use opcode::OpCode;

    let mut chunk = Chunk::new();

    chunk.add_constant(1.2, 0..0);
    chunk.add_constant(3.4, 0..0);

    chunk.add_code(OpCode::Add, 0..0);

    chunk.add_constant(5.6, 0..0);

    chunk.add_code(OpCode::Divide, 0..0);
    chunk.add_code(OpCode::Negate, 0..0);
    chunk.add_code(OpCode::Return, 0..0);
    println!("{chunk}");

    let mut vm = VM::new(&chunk);
    let res = vm.run();
    println!("{res:?}")
}
