use crate::vm::VM;

mod chunk;
mod opcode;
mod value;
mod vm;

fn main() {
    use chunk::Chunk;
    use opcode::OpCode;

    let mut chunk = Chunk::new();

    chunk.add_constant(1.2, 123);
    chunk.add_constant(3.4, 123);

    chunk.add_code(OpCode::Add, 123);

    chunk.add_constant(5.6, 123);

    chunk.add_code(OpCode::Divide, 123);
    chunk.add_code(OpCode::Negate, 123);
    chunk.add_code(OpCode::Return, 123);
    println!("{chunk}");

    let mut vm = VM::new(&chunk);
    let res = vm.run();
    println!("{res:?}")
}
