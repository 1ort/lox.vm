use crate::vm::VM;

mod chunk;
mod opcode;
mod value;
mod vm;

fn main() {
    use chunk::Chunk;
    use opcode::OpCode;

    let mut chunk = Chunk::new();

    chunk.add_constant(1.5, 99);
    chunk.add_code(OpCode::Negate, 99);
    chunk.add_code(OpCode::Return, 100);
    println!("{chunk}");

    let mut vm = VM::new(&chunk);
    let res = vm.run();
    println!("{res:?}")
}
