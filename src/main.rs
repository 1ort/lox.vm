use crate::compile::compile;
use crate::interner::Interner;
use crate::vm::VM;

mod builtins;
mod chunk;
mod compile;
mod interner;
mod opcode;
mod value;
mod vm;

use std::env;
use std::process::ExitCode;

use gc::GcHeap;
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
    let interner = Interner::new();
    let heap = GcHeap::new();
    let mut vm = VM::new(interner, heap);
    vm.add_builtins(builtins::get_builtins());
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())
                    .expect("Can not add line to history");
                let (interner, heap) = vm.borrow_interner_and_heap();

                let function_object = compile("REPL", &line, interner, heap);
                if let Err(errors) = function_object {
                    for error in errors {
                        // TODO: add error formatter
                        eprintln!("{error:?}");
                    }
                    continue;
                }
                let function_object = function_object.expect("Chunk should be checked");
                let result = vm.interpret(function_object);
                match result {
                    Ok(value) if !value.is_nil() => println!("{value}"),
                    Err(error) => eprintln!("{error:?}"),
                    _ => {}
                }
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

fn run_file(_path: &std::ffi::OsString) -> ExitCode {
    todo!()
}
