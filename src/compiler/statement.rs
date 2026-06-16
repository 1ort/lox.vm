use std::mem;
use std::rc::Rc;

use super::Compiler;
use super::Identifier;
use super::LoopContext;
use super::SyntaxError;
use super::token::TokenType;
use crate::chunk::Chunk;
use crate::compiler::CompilerContext;
use crate::compiler::FunctionKind;
use crate::compiler::FunctionObject;
use crate::opcode::OpCode;

impl<'a> Compiler<'a> {
    pub(super) fn declaration(&mut self) -> Result<(), SyntaxError> {
        match self.peek().token_type {
            TokenType::Fun => self.fun_declaration(),
            TokenType::Var => self.var_declaration(),
            _ => self.statement(),
        }
    }

    pub(super) fn block(&mut self) -> Result<(), SyntaxError> {
        self.begin_scope();
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
        self.end_scope(&closing_brace.span);
        Ok(())
    }

    fn fun_declaration(&mut self) -> Result<(), SyntaxError> {
        let fun_tok = self.next()?;
        let identifier = self.variable()?;
        if self.context.scope_depth > 0 {
            let index = self.add_local(&identifier)?;
            self.context.locals[index].initialized = true;
        }
        let enclosing_function_object = mem::replace(
            &mut self.function_object,
            FunctionObject {
                arity: 0,
                chunk: Chunk::new(),
                name: Rc::clone(&identifier.name),
            },
        );
        let enclosing_context = mem::replace(
            &mut self.context,
            CompilerContext::new(FunctionKind::Function),
        );
        self.begin_scope();
        self.reserve_first_stack_slot();
        let result = self.function_statement();
        self.end_scope(&fun_tok.span.clone());
        self.context = enclosing_context;
        let function_object = mem::replace(&mut self.function_object, enclosing_function_object);
        result?;
        self.current_chunk().add_const_code(
            OpCode::Constant,
            Rc::new(function_object),
            fun_tok.span,
        );
        if self.context.scope_depth == 0 {
            self.current_chunk().add_const_code(
                OpCode::DefineGlobal,
                Rc::clone(&identifier.name),
                identifier.span,
            );
        }
        Ok(())
    }

    fn function_statement(&mut self) -> Result<(), SyntaxError> {
        self.expect_token(TokenType::LeftParen, "Expect '(' after function name.")?;
        if !matches!(self.peek().token_type, TokenType::RightParen) {
            loop {
                let param = self.parameter()?;

                self.function_object.arity += 1;

                if self.function_object.arity == 255 {
                    return Err(SyntaxError {
                        message: "Can't have more than 255 parameters.".to_owned(),
                        span: param.span,
                    });
                }
                let local_index = self.add_local(&param)?;
                self.context.locals[local_index].initialized = true;

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
        self.block()
    }

    fn var_declaration(&mut self) -> Result<(), SyntaxError> {
        let var = self.next()?;
        let identifier = self.variable()?;

        let local_index = if self.context.scope_depth > 0 {
            Some(self.add_local(&identifier)?)
        } else {
            None
        };

        if matches!(self.peek().token_type, TokenType::Equal) {
            let _ = self.next();
            self.expression()?;
        } else {
            self.current_chunk().add_code(OpCode::Nil, var.span.clone());
        }
        self.expect_token(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )?;

        match local_index {
            Some(index) => self.context.locals[index].initialized = true,
            None => self.current_chunk().add_const_code(
                OpCode::DefineGlobal,
                identifier.name,
                identifier.span,
            ),
        }

        Ok(())
    }

    fn expression_statement(&mut self) -> Result<(), SyntaxError> {
        self.expression()?;
        let span = self
            .expect_token(TokenType::Semicolon, "Expect ';' after value.")?
            .span;
        self.current_chunk().add_code(OpCode::Pop, span);
        Ok(())
    }

    fn identifier(&mut self, error_message: &str) -> Result<Identifier, SyntaxError> {
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
        self.current_chunk().add_code(OpCode::Print, next.span);
        Ok(())
    }

    fn if_statement(&mut self) -> Result<(), SyntaxError> {
        let if_tok = self.next()?;

        self.expect_token(TokenType::LeftParen, "Expect '(' after 'if'.")?;
        self.expression()?;
        self.expect_token(TokenType::RightParen, "Expect ')' after condition.")?;

        let then_jump = self.emit_jump(OpCode::JumpIfFalse, if_tok.span.clone());
        self.current_chunk()
            .add_code(OpCode::Pop, if_tok.span.clone());

        self.statement()?;

        let else_jump = self.emit_jump(OpCode::Jump, if_tok.span.clone());
        //self.current_chunk().add_code(OpCode::Pop, if_tok.span.clone());

        self.patch_jump(then_jump);
        self.current_chunk()
            .add_code(OpCode::Pop, if_tok.span.clone());

        if matches!(self.peek().token_type, TokenType::Else) {
            self.next()?;
            self.statement()?;
        }
        self.patch_jump(else_jump);
        Ok(())
    }

    fn while_statement(&mut self) -> Result<(), SyntaxError> {
        let loop_start = self.current_chunk().code.len();
        let enclosing_loop = self.context.loop_context.replace(LoopContext {
            stack_depth_at_start: self.context.locals.len(),
            break_patches: Vec::new(),
            loop_start,
        });
        let result = {
            let while_tok = self.next()?;
            self.expect_token(TokenType::LeftParen, "Expect '(' after 'while'.")?;
            let loop_start = self.current_chunk().code.len();
            self.expression()?;
            self.expect_token(TokenType::RightParen, "Expect ')' after condition.")?;

            let exit_jump = self.emit_jump(OpCode::JumpIfFalse, while_tok.span.clone());
            self.current_chunk()
                .add_code(OpCode::Pop, while_tok.span.clone());
            self.statement()?;

            self.emit_loop(loop_start, while_tok.span.clone());
            self.patch_jump(exit_jump);
            self.current_chunk()
                .add_code(OpCode::Pop, while_tok.span.clone());

            self.patch_breaks();

            Ok(())
        };
        self.context.loop_context = enclosing_loop;
        result
    }

    fn for_statement(&mut self) -> Result<(), SyntaxError> {
        let for_tok = self.next()?;
        self.begin_scope();
        self.expect_token(TokenType::LeftParen, "Expect '(' after 'for'.")?;

        // initializer
        match self.peek().token_type {
            TokenType::Semicolon => {
                self.next()?;
            }
            TokenType::Var => self.var_declaration()?,
            _ => self.expression_statement()?,
        }

        let loop_start = self.current_chunk().code.len();
        let enclosing_loop = self.context.loop_context.replace(LoopContext {
            stack_depth_at_start: self.context.locals.len(),
            break_patches: Vec::new(),
            loop_start,
        });
        let compile_result = {
            let mut loop_start = self.current_chunk().code.len();

            // condition
            let exit_jump = if matches!(self.peek().token_type, TokenType::Semicolon) {
                None
            } else {
                self.expression()?;
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse, for_tok.span.clone());
                self.current_chunk()
                    .add_code(OpCode::Pop, for_tok.span.clone());
                Some(exit_jump)
            };
            self.expect_token(TokenType::Semicolon, "Expect ';' after condition.")?;

            // increment
            if !matches!(self.peek().token_type, TokenType::RightParen) {
                let body_jump = self.emit_jump(OpCode::Jump, for_tok.span.clone());
                let increment_start = self.current_chunk().code.len();

                self.expression()?;
                self.current_chunk()
                    .add_code(OpCode::Pop, for_tok.span.clone());

                self.emit_loop(loop_start, for_tok.span.clone());
                loop_start = increment_start;
                self.context
                    .loop_context
                    .as_mut()
                    .expect("loop context should exist inside for loop")
                    .loop_start = increment_start;
                self.patch_jump(body_jump);
            }
            self.expect_token(TokenType::RightParen, "Expect ')' after for clauses.")?;

            // body:
            self.statement()?;
            self.emit_loop(loop_start, for_tok.span.clone());

            if let Some(exit_jump) = exit_jump {
                self.patch_jump(exit_jump);
                self.current_chunk()
                    .add_code(OpCode::Pop, for_tok.span.clone());
            }

            self.patch_breaks();
            Ok(())
        };
        self.end_scope(&for_tok.span);
        self.context.loop_context = enclosing_loop;
        compile_result
    }

    fn patch_breaks(&mut self) {
        for offset in {
            if let Some(ref ctx) = self.context.loop_context {
                ctx.break_patches.clone()
            } else {
                Vec::new()
            }
        } {
            self.patch_jump(offset);
        }
    }

    fn break_statement(&mut self) -> Result<(), SyntaxError> {
        let break_tok = self.next()?;
        if self.context.loop_context.is_none() {
            return Err(SyntaxError {
                message: "'break' outside loop.".to_owned(),
                span: break_tok.span,
            });
        }
        let mut ctx = self
            .context
            .loop_context
            .take()
            .expect("None should be checked");
        let delta_depth = self.context.locals.len() - ctx.stack_depth_at_start;
        for _ in 0..delta_depth {
            self.current_chunk()
                .add_code(OpCode::Pop, break_tok.span.clone());
        }
        let jump = self.emit_jump(OpCode::Jump, break_tok.span.clone());
        ctx.break_patches.push(jump);
        self.context.loop_context.replace(ctx);
        self.expect_token(TokenType::Semicolon, "Expect ';' after break.")?;
        Ok(())
    }

    fn continue_statement(&mut self) -> Result<(), SyntaxError> {
        let continue_tok = self.next()?;
        if self.context.loop_context.is_none() {
            return Err(SyntaxError {
                message: "'continue' outside loop.".to_owned(),
                span: continue_tok.span,
            });
        }
        let ctx = self
            .context
            .loop_context
            .take()
            .expect("None should be checked");

        let delta_depth = self.context.locals.len() - ctx.stack_depth_at_start;
        for _ in 0..delta_depth {
            self.current_chunk()
                .add_code(OpCode::Pop, continue_tok.span.clone());
        }
        self.emit_loop(ctx.loop_start, continue_tok.span.clone());
        self.context.loop_context.replace(ctx);
        self.expect_token(TokenType::Semicolon, "Expect ';' after break.")?;
        Ok(())
    }

    fn return_statement(&mut self) -> Result<(), SyntaxError> {
        Ok(())
    }
}
