use crate::vm::VM;

mod chunk;
mod compiler;
mod opcode;
mod value;
mod vm;

fn main() {
    use compiler::compile;

    let chunk = compile("(10/7)+3");
    println!("{chunk}");
    let mut vm = VM::new(&chunk);
    let result = vm.run();
    println!("{result:?}")
}
