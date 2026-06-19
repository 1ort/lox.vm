use std::rc::Rc;

use super::Identifier;
use super::Parser;
use super::SyntaxError;
use super::token::TokenType;

use super::FunctionKind;
use super::FunctionObject;
use super::compiler::Compiler;
use crate::opcode::OpCode;

impl<'a> Parser<'a> {
    pub(super) fn declaration(&mut self) -> Result<(), SyntaxError> {
        match self.peek().token_type {
            TokenType::Fun => self.fun_declaration(),
            TokenType::Var => self.var_declaration(),
            _ => self.statement(),
        }
    }

    pub(super) fn block(&mut self) -> Result<(), SyntaxError> {
        self.compiler.begin_scope();
        self.next()?;
        while !matches!(
            self.peek().token_type,
            TokenType::RightBrace | TokenType::Eof
        ) {
            match self.declaration() {
                Ok(_) => continue,
                Err(err) => {
                    self.errors.push(err);
                    self.synchronize();
                }
            }
        }
        let closing_brace = self.expect_token(TokenType::RightBrace, "Expect '}' after block.")?;
        self.compiler.end_scope(&closing_brace.span);
        Ok(())
    }

    fn fun_declaration(&mut self) -> Result<(), SyntaxError> {
        let fun_tok = self.next()?;
        let identifier = self.variable()?;
        if !self.compiler.is_global_scope() {
            let index = self.compiler.add_local(&identifier)?;
            self.compiler.initialize_local(index);
        }
        let inner_context = Compiler::new(
            FunctionObject::new(&identifier.name),
            FunctionKind::Function,
        );
        self.enter_context(inner_context);
        self.compiler.begin_scope();
        self.compiler.reserve_first_stack_slot();

        let result = self.function_statement();

        let inner_context = self.exit_context();
        let function_object = inner_context.function_object;
        result?;
        self.compiler
            .emit_constant(Rc::new(function_object), fun_tok.span);
        if self.compiler.is_global_scope() {
            self.compiler
                .emit_define_global(&identifier.name, identifier.span);
        }
        Ok(())
    }

    fn function_statement(&mut self) -> Result<(), SyntaxError> {
        self.expect_token(TokenType::LeftParen, "Expect '(' after function name.")?;
        if !matches!(self.peek().token_type, TokenType::RightParen) {
            loop {
                let param = self.parameter()?;

                self.compiler.function_object.arity += 1;

                if self.compiler.function_object.arity == 255 {
                    return Err(SyntaxError {
                        message: "Can't have more than 255 parameters.".to_owned(),
                        span: param.span,
                    });
                }
                let local_index = self.compiler.add_local(&param)?;
                self.compiler.initialize_local(local_index);

                if matches!(self.peek().token_type, TokenType::Comma) {
                    self.next()?;
                } else {
                    break;
                }
            }
        }
        self.expect_token(TokenType::RightParen, "Expect ')' after parameters.")?;
        if !matches!(self.peek().token_type, TokenType::LeftBrace) {
            return Err(SyntaxError {
                message: "Expect '{' before function body.".to_owned(),
                span: self.peek().span.clone(),
            });
        }
        self.block()?;
        let block_end = self.peek().span.start;
        self.compiler.emit_return(block_end..block_end);
        Ok(())
    }

    fn var_declaration(&mut self) -> Result<(), SyntaxError> {
        let var = self.next()?;
        let identifier = self.variable()?;

        let local_index = if !self.compiler.is_global_scope() {
            Some(self.compiler.add_local(&identifier)?)
        } else {
            None
        };

        if matches!(self.peek().token_type, TokenType::Equal) {
            let _ = self.next();
            self.expression()?;
        } else {
            self.compiler.emit_byte(OpCode::Nil, var.span.clone());
        }
        self.expect_token(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )?;

        match local_index {
            Some(index) => self.compiler.initialize_local(index),
            None => self
                .compiler
                .emit_define_global(&identifier.name, identifier.span),
        }

        Ok(())
    }

    fn expression_statement(&mut self) -> Result<(), SyntaxError> {
        self.expression()?;
        let span = self
            .expect_token(TokenType::Semicolon, "Expect ';' after value.")?
            .span;
        self.compiler.emit_byte(OpCode::Pop, span);
        Ok(())
    }

    pub(super) fn identifier(&mut self, error_message: &str) -> Result<Identifier, SyntaxError> {
        let token = self.expect_token(TokenType::Identifier, error_message)?;
        let lexeme = &self.source[token.span.clone()];
        let name = self.interner.intern(lexeme);
        Ok(Identifier {
            name,
            span: token.span,
        })
    }

    fn variable(&mut self) -> Result<Identifier, SyntaxError> {
        self.identifier("Expect variable name.")
    }

    fn parameter(&mut self) -> Result<Identifier, SyntaxError> {
        self.identifier("Expect parameter name.")
    }

    fn statement(&mut self) -> Result<(), SyntaxError> {
        match self.peek().token_type {
            TokenType::Print => self.print_statement(),
            TokenType::LeftBrace => self.block(),
            TokenType::If => self.if_statement(),
            TokenType::While => self.while_statement(),
            TokenType::For => self.for_statement(),
            TokenType::Break => self.break_statement(),
            TokenType::Continue => self.continue_statement(),
            TokenType::Return => self.return_statement(),
            _ => self.expression_statement(),
        }
    }

    fn print_statement(&mut self) -> Result<(), SyntaxError> {
        let next = self.next()?;
        self.expression()?;
        self.expect_token(TokenType::Semicolon, "Expect ';' after value.")?;
        self.compiler.emit_byte(OpCode::Print, next.span);
        Ok(())
    }

    fn if_statement(&mut self) -> Result<(), SyntaxError> {
        let if_tok = self.next()?;

        self.expect_token(TokenType::LeftParen, "Expect '(' after 'if'.")?;
        self.expression()?;
        self.expect_token(TokenType::RightParen, "Expect ')' after condition.")?;

        let then_jump = self
            .compiler
            .emit_jump(OpCode::JumpIfFalse, if_tok.span.clone());
        self.compiler.emit_byte(OpCode::Pop, if_tok.span.clone());

        self.statement()?;

        let else_jump = self.compiler.emit_jump(OpCode::Jump, if_tok.span.clone());

        self.compiler.patch_jump(then_jump);
        self.compiler.emit_byte(OpCode::Pop, if_tok.span.clone());

        if matches!(self.peek().token_type, TokenType::Else) {
            self.next()?;
            self.statement()?;
        }
        self.compiler.patch_jump(else_jump);
        Ok(())
    }

    fn while_statement(&mut self) -> Result<(), SyntaxError> {
        let loop_start = self.compiler.next_ip();
        self.compiler.enter_loop_context(loop_start);
        let result = {
            let while_tok = self.next()?;
            self.expect_token(TokenType::LeftParen, "Expect '(' after 'while'.")?;
            let loop_start = self.compiler.next_ip();
            self.expression()?;
            self.expect_token(TokenType::RightParen, "Expect ')' after condition.")?;

            let exit_jump = self
                .compiler
                .emit_jump(OpCode::JumpIfFalse, while_tok.span.clone());
            self.compiler.emit_byte(OpCode::Pop, while_tok.span.clone());
            self.statement()?;

            self.compiler.emit_loop(loop_start, while_tok.span.clone());
            self.compiler.patch_jump(exit_jump);
            self.compiler.emit_byte(OpCode::Pop, while_tok.span.clone());
            Ok(())
        };
        self.compiler.exit_loop_context();
        result
    }

    fn for_statement(&mut self) -> Result<(), SyntaxError> {
        let for_tok = self.next()?;
        self.compiler.begin_scope();
        self.expect_token(TokenType::LeftParen, "Expect '(' after 'for'.")?;

        // initializer
        match self.peek().token_type {
            TokenType::Semicolon => {
                self.next()?;
            }
            TokenType::Var => self.var_declaration()?,
            _ => self.expression_statement()?,
        }

        let loop_start = self.compiler.next_ip();
        self.compiler.enter_loop_context(loop_start);
        let compile_result = {
            let mut loop_start = self.compiler.next_ip();

            // condition
            let exit_jump = if matches!(self.peek().token_type, TokenType::Semicolon) {
                None
            } else {
                self.expression()?;
                let exit_jump = self
                    .compiler
                    .emit_jump(OpCode::JumpIfFalse, for_tok.span.clone());
                self.compiler.emit_byte(OpCode::Pop, for_tok.span.clone());
                Some(exit_jump)
            };
            self.expect_token(TokenType::Semicolon, "Expect ';' after condition.")?;

            // increment
            if !matches!(self.peek().token_type, TokenType::RightParen) {
                let body_jump = self.compiler.emit_jump(OpCode::Jump, for_tok.span.clone());
                let increment_start = self.compiler.next_ip();

                self.expression()?;
                self.compiler.emit_byte(OpCode::Pop, for_tok.span.clone());

                self.compiler.emit_loop(loop_start, for_tok.span.clone());
                loop_start = increment_start;
                self.compiler.rebase_loop_start(increment_start);
                self.compiler.patch_jump(body_jump);
            }
            self.expect_token(TokenType::RightParen, "Expect ')' after for clauses.")?;

            // body:
            self.statement()?;
            self.compiler.emit_loop(loop_start, for_tok.span.clone());

            if let Some(exit_jump) = exit_jump {
                self.compiler.patch_jump(exit_jump);
                self.compiler.emit_byte(OpCode::Pop, for_tok.span.clone());
            }

            Ok(())
        };
        self.compiler.end_scope(&for_tok.span);
        self.compiler.exit_loop_context();
        compile_result
    }

    fn break_statement(&mut self) -> Result<(), SyntaxError> {
        let break_tok = self.next()?;
        if !self.compiler.is_inside_loop() {
            return Err(SyntaxError {
                message: "'break' outside loop.".to_owned(),
                span: break_tok.span,
            });
        }
        self.compiler.compile_break(break_tok.span);
        self.expect_token(TokenType::Semicolon, "Expect ';' after break.")?;
        Ok(())
    }

    fn continue_statement(&mut self) -> Result<(), SyntaxError> {
        let continue_tok = self.next()?;
        if !self.compiler.is_inside_loop() {
            return Err(SyntaxError {
                message: "'continue' outside loop.".to_owned(),
                span: continue_tok.span,
            });
        }
        self.compiler.compile_continue(continue_tok.span);
        self.expect_token(TokenType::Semicolon, "Expect ';' after break.")?;
        Ok(())
    }

    fn return_statement(&mut self) -> Result<(), SyntaxError> {
        let return_tok = self.next()?;

        if self.compiler.is_top_level_code() {
            return Err(SyntaxError {
                message: "Can't return from top-level code.".to_owned(),
                span: return_tok.span,
            });
        }

        if matches!(self.peek().token_type, TokenType::Semicolon) {
            self.compiler.emit_return(return_tok.span);
        } else {
            self.expression()?;
            self.compiler
                .emit_byte(OpCode::Return, return_tok.span.clone());
        }
        self.expect_token(TokenType::Semicolon, "Expect ';' after return value.")?;
        Ok(())
    }
}
