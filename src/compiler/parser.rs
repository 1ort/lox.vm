use super::lexer::Lexer;
use super::token::Token;
use crate::{
    chunk::Chunk, compiler::token::TokenType, interner::Interner, opcode::OpCode, value::Value,
};
use std::{iter::Peekable, mem::discriminant, ops::Range};

mod expression;

#[derive(Debug)]
pub struct SyntaxError {
    #[expect(unused)]
    message: String,
    #[expect(unused)]
    span: Range<usize>,
}

pub(super) struct Parser<'a> {
    source: &'a str,
    tokens: Peekable<Lexer<'a>>,
    chunk: &'a mut Chunk,
    interner: &'a mut Interner,
}

impl<'a> Parser<'a> {
    pub(super) fn new(
        source: &'a str,
        tokens: Peekable<Lexer<'a>>,
        chunk: &'a mut Chunk,
        interner: &'a mut Interner,
    ) -> Self {
        Self {
            source,
            tokens,
            chunk,
            interner,
        }
    }

    pub(super) fn compile(mut self) -> Result<(), Vec<SyntaxError>> {
        let mut errors: Vec<SyntaxError> = Vec::new();

        loop {
            let next = self.peek();
            if matches!(next.token_type, TokenType::Eof) {
                break;
            }
            if let Err(err) = self.declaration() {
                errors.push(err);
                self.synchronize();
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn lexeme(&self, span: &Range<usize>) -> &'a str {
        &self.source[span.clone()]
    }

    fn next(&mut self) -> Result<Token, SyntaxError> {
        match self
            .tokens
            .next()
            .expect("iterator should not be exhausted")
        {
            Token {
                token_type: TokenType::UnterminatedString,
                span,
            } => Err(SyntaxError {
                message: "Unterminated string".to_owned(),
                span,
            }),
            Token {
                token_type: TokenType::Unknown,
                span,
            } => Err(SyntaxError {
                message: "Unknown token".to_owned(),
                span,
            }),
            Token {
                token_type: TokenType::Eof,
                span,
            } => Err(SyntaxError {
                message: "Unexpected EOF".to_owned(),
                span,
            }),
            tok => Ok(tok),
        }
    }

    fn peek(&mut self) -> &Token {
        self.tokens
            .peek()
            .expect("iterator should not be exhausted")
    }

    fn expect_token(
        &mut self,
        expected_token_type: TokenType,
        message: &str,
    ) -> Result<Token, SyntaxError> {
        if discriminant(&expected_token_type) == discriminant(&self.peek().token_type) {
            self.next()
        } else {
            Err(SyntaxError {
                message: message.to_owned(),
                span: self.peek().span.clone(),
            })
        }
    }

    fn synchronize(&mut self) {
        if self.tokens.peek().is_none() {
            return;
        }

        loop {
            match self.peek().token_type {
                TokenType::Semicolon => {
                    let _ = self.next();
                    break;
                }
                TokenType::Eof
                | TokenType::Class
                | TokenType::Var
                | TokenType::Fun
                | TokenType::Print
                | TokenType::Return
                | TokenType::For
                | TokenType::While
                | TokenType::If => break,
                _ => {
                    let _ = self.next();
                }
            }
        }
    }

    fn declaration(&mut self) -> Result<(), SyntaxError> {
        match self.peek().token_type {
            TokenType::Var => self.var_declaration(),
            _ => self.statement(),
        }
    }

    fn var_declaration(&mut self) -> Result<(), SyntaxError> {
        let var = self.next()?;
        let global = self.identifier()?;

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
        self.chunk
            .add_const_code(OpCode::DefineGlobal, global, var.span);
        Ok(())
    }

    fn statement(&mut self) -> Result<(), SyntaxError> {
        match self.peek().token_type {
            TokenType::Print => self.print_statement(),
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

    fn expression_statement(&mut self) -> Result<(), SyntaxError> {
        self.expression()?;
        let span = self
            .expect_token(TokenType::Semicolon, "Expect ';' after value.")?
            .span;
        self.chunk.add_code(OpCode::Pop, span);
        Ok(())
    }

    fn identifier(&mut self) -> Result<Value, SyntaxError> {
        let token = self.expect_token(TokenType::Identifier, "Expect variable name.")?;
        let lexeme = &self.source[token.span.clone()];
        let identifier = self.interner.intern(lexeme);
        Ok(Value::Str(identifier))
    }
}
