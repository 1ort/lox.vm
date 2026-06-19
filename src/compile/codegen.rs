use super::Identifier;
use crate::compile::Compiler;
use crate::opcode::OpCode;
use crate::value::Value;
use std::ops::Range;
use std::rc::Rc;

impl Compiler {
    pub(crate) fn emit_byte(&mut self, byte: impl Into<u8>, span: Range<usize>) {
        self.current_chunk().add_byte(byte, span);
    }

    pub(crate) fn emit_word(&mut self, word: impl Into<u16>, span: Range<usize>) {
        self.current_chunk().add_word(word, span);
    }

    pub(crate) fn emit_jump(&mut self, opcode: impl Into<u8>, span: Range<usize>) -> usize {
        self.emit_byte(opcode, span.clone());
        self.emit_word(u16::MAX, span.clone());
        self.current_chunk().code.len() - 2
    }

    pub(crate) fn emit_loop(&mut self, loop_start: usize, span: Range<usize>) {
        self.emit_byte(OpCode::Loop, span.clone());
        let jump = self.current_chunk().code.len() - loop_start + 2;
        if jump >= 2usize.pow(16) {
            panic!("Too much code to jump over.")
        }
        self.emit_word(jump as u16, span);
    }

    pub(crate) fn patch_jump(&mut self, offset: usize) {
        let jump = self.current_chunk().code.len() - offset - 2;
        if jump >= 2usize.pow(16) {
            panic!("Too much code to jump over.")
        }
        self.current_chunk().patch_word(offset, jump as u16);
    }

    pub(crate) fn emit_return(&mut self, span: Range<usize>) {
        self.emit_byte(OpCode::Nil, span.clone());
        self.emit_byte(OpCode::Return, span.clone());
    }

    pub(crate) fn emit_define_global(&mut self, name: &Rc<str>, span: Range<usize>) {
        let index = self.current_chunk().push_constant(Rc::clone(name));
        self.emit_byte(OpCode::DefineGlobal, span.clone());
        self.emit_word(index, span);
    }

    pub(crate) fn emit_get_global(&mut self, identifier: Identifier) {
        let span = identifier.span;
        let index = self.current_chunk().push_constant(identifier.name);
        self.emit_byte(OpCode::GetGlobal, span.clone());
        self.emit_word(index, span);
    }

    pub(crate) fn emit_set_global(&mut self, identifier: Identifier) {
        let span = identifier.span;
        let index = self.current_chunk().push_constant(identifier.name);
        self.emit_byte(OpCode::SetGlobal, span.clone());
        self.emit_word(index, span);
    }

    pub(crate) fn emit_constant(&mut self, value: impl Into<Value>, span: Range<usize>) {
        let index = self.current_chunk().push_constant(value);
        self.emit_byte(OpCode::Constant, span.clone());
        self.emit_word(index, span);
    }

    pub(crate) fn emit_get_local(&mut self, local_index: u16, span: Range<usize>) {
        self.emit_byte(OpCode::GetLocal, span.clone());
        self.emit_word(local_index, span);
    }

    pub(crate) fn emit_set_local(&mut self, local_index: u16, span: Range<usize>) {
        self.emit_byte(OpCode::SetLocal, span.clone());
        self.emit_word(local_index, span);
    }

    pub(crate) fn emit_get_upvalue(&mut self, local_index: u16, span: Range<usize>) {
        self.emit_byte(OpCode::GetUpvalue, span.clone());
        self.emit_word(local_index, span);
    }

    pub(crate) fn emit_set_upvalue(&mut self, local_index: u16, span: Range<usize>) {
        self.emit_byte(OpCode::SetUpvalue, span.clone());
        self.emit_word(local_index, span);
    }

    pub(crate) fn emit_call(&mut self, arg_count: u16, span: Range<usize>) {
        self.emit_byte(OpCode::Call, span.clone());
        self.emit_word(arg_count, span);
    }

    pub(crate) fn next_ip(&mut self) -> usize {
        self.current_chunk().code.len()
    }
}
