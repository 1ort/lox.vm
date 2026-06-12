use super::Identifier;
use super::Parser;
use super::SyntaxError;
use crate::compiler::token::TokenType;
use crate::opcode::OpCode;

impl<'a> Parser<'a> {
    pub(crate) fn declaration(&mut self) -> Result<(), SyntaxError> {
        match self.peek().token_type {
            TokenType::Var => self.var_declaration(),
            _ => self.statement(),
        }
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
            self.chunk.add_code(OpCode::Nil, var.span.clone());
        }
        self.expect_token(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )?;

        match local_index {
            Some(index) => self.locals[index].initialized = true,
            None => {
                self.chunk
                    .add_const_code(OpCode::DefineGlobal, identifier.name, identifier.span)
            }
        }

        Ok(())
    }

    fn block(&mut self) -> Result<(), SyntaxError> {
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

    fn expression_statement(&mut self) -> Result<(), SyntaxError> {
        self.expression()?;
        let span = self
            .expect_token(TokenType::Semicolon, "Expect ';' after value.")?
            .span;
        self.chunk.add_code(OpCode::Pop, span);
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
            _ => self.expression_statement(),
        }
    }

    fn print_statement(&mut self) -> Result<(), SyntaxError> {
        let next = self.next()?;
        self.expression()?;
        self.expect_token(TokenType::Semicolon, "Expect ';' after value.")?;
        self.chunk.add_code(OpCode::Print, next.span);
        Ok(())
    }

    fn if_statement(&mut self) -> Result<(), SyntaxError> {
        let if_tok = self.next()?;

        self.expect_token(TokenType::LeftParen, "Expect '(' after 'if'.")?;
        self.expression()?;
        self.expect_token(TokenType::RightParen, "Expect ')' after condition.")?;

        let jump = self.emit_jump(OpCode::JumpIfFalse, if_tok.span.clone());
        self.chunk.add_code(OpCode::Pop, if_tok.span.clone());

        self.statement()?;

        let else_jump = self.emit_jump(OpCode::Jump, if_tok.span.clone());
        self.chunk.add_code(OpCode::Pop, if_tok.span.clone());

        self.patch_jump(jump);
        if matches!(self.peek().token_type, TokenType::Else) {
            self.next()?;
            self.statement()?;
            self.patch_jump(else_jump);
        }
        Ok(())
    }
}
