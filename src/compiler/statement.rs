use super::Compiler;
use super::Identifier;
use super::LoopContext;
use super::SyntaxError;
use super::token::TokenType;
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
        let identifier = self.identifier()?;

        self.expect_token(TokenType::LeftParen, "Expect '(' after function name.")?;
        self.expect_token(TokenType::RightParen, "Expect ')' after parameters.")?;

        match self.peek().token_type {
            TokenType::LeftBrace => {}
            _ => {
                return Err(SyntaxError {
                    message: "Expect '{' before function body.".to_owned(),
                    span: self.peek().span.clone(),
                });
            }
        }
        Ok(())
    }

    fn var_declaration(&mut self) -> Result<(), SyntaxError> {
        let var = self.next()?;
        let identifier = self.identifier()?;

        let local_index = if self.scope_depth > 0 {
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
            Some(index) => self.locals[index].initialized = true,
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

    fn identifier(&mut self) -> Result<Identifier, SyntaxError> {
        let token = self.expect_token(TokenType::Identifier, "Expect variable name.")?;
        let lexeme = &self.source[token.span.clone()];
        let name = self.interner.intern(lexeme);
        Ok(Identifier {
            name,
            span: token.span,
        })
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
        let enclosing_loop = self.loop_context.replace(LoopContext {
            stack_depth_at_start: self.locals.len(),
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
        self.loop_context = enclosing_loop;
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
        let enclosing_loop = self.loop_context.replace(LoopContext {
            stack_depth_at_start: self.locals.len(),
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
                self.loop_context
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
        self.loop_context = enclosing_loop;
        compile_result
    }

    fn patch_breaks(&mut self) {
        for offset in {
            if let Some(ref ctx) = self.loop_context {
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
        if self.loop_context.is_none() {
            return Err(SyntaxError {
                message: "'break' outside loop.".to_owned(),
                span: break_tok.span,
            });
        }
        let mut ctx = self.loop_context.take().expect("None should be checked");
        let delta_depth = self.locals.len() - ctx.stack_depth_at_start;
        for _ in 0..delta_depth {
            self.current_chunk()
                .add_code(OpCode::Pop, break_tok.span.clone());
        }
        let jump = self.emit_jump(OpCode::Jump, break_tok.span.clone());
        ctx.break_patches.push(jump);
        self.loop_context.replace(ctx);
        self.expect_token(TokenType::Semicolon, "Expect ';' after break.")?;
        Ok(())
    }

    fn continue_statement(&mut self) -> Result<(), SyntaxError> {
        let continue_tok = self.next()?;
        if self.loop_context.is_none() {
            return Err(SyntaxError {
                message: "'continue' outside loop.".to_owned(),
                span: continue_tok.span,
            });
        }
        let ctx = self.loop_context.take().expect("None should be checked");

        let delta_depth = self.locals.len() - ctx.stack_depth_at_start;
        for _ in 0..delta_depth {
            self.current_chunk()
                .add_code(OpCode::Pop, continue_tok.span.clone());
        }
        self.emit_loop(ctx.loop_start, continue_tok.span.clone());
        self.loop_context.replace(ctx);
        self.expect_token(TokenType::Semicolon, "Expect ';' after break.")?;
        Ok(())
    }
}
