use crate::compiler::compile;
use crate::interner::Interner;
use crate::vm::VM;

mod chunk;
mod compiler;
mod interner;
mod opcode;
mod value;
mod vm;

use std::env;
use std::process::ExitCode;

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match &args[..] {
        [] => repl(),
        [path] => run_file(path),
        _ => incorrect_usage(),
    }
}

fn repl() -> ExitCode {
    let mut rl = DefaultEditor::new().expect("Can not start repl");
    let mut interner = Interner::new();
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())
                    .expect("Can not add line to history");
                let chunk = compile(&line, &mut interner);
                let mut vm = VM::new(&chunk, &mut interner);
                let result = vm.run();
                println!("{result:?}");
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                return ExitCode::from(0);
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                return ExitCode::from(0);
            }
            Err(err) => {
                println!("Error: {:?}", err);
                return ExitCode::from(1);
            }
        }
    }
}

fn incorrect_usage() -> ExitCode {
    println!("Usage: lox [script]");
    ExitCode::from(64)
}

fn run_file(path: &std::ffi::OsString) -> ExitCode {
    todo!()
}
