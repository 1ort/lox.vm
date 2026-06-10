use super::lexer::Lexer;
use super::token::Token;
use crate::{chunk::Chunk, compiler::token::TokenType, interner::Interner, opcode::OpCode};
use std::{iter::Peekable, mem::discriminant, ops::Range};

mod expression;

#[derive(Debug)]
pub struct SyntaxError {
    message: String,
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

    pub(super) fn compile(&mut self) -> Result<(), SyntaxError> {
        loop {
            let next = self.peek();
            if matches!(next.token_type, TokenType::Eof) {
                //let eof = self.next();
                //self.chunk.add_code(OpCode::Return, eof.span);
                break;
            }

            self.declaration()?;
        }
        Ok(())
    }

    fn lexeme(&self, span: &Range<usize>) -> &'a str {
        &self.source[span.clone()]
    }

    fn next(&mut self) -> Token {
        self.tokens
            .next()
            .expect("iterator should not be exhausted")
    }

    fn peek(&mut self) -> &Token {
        self.tokens
            .peek()
            .expect("iterator should not be exhausted")
    }

    fn match_token(&mut self, token_type: TokenType) -> Option<Token> {
        if discriminant(&token_type) == discriminant(&self.peek().token_type) {
            Some(self.next())
        } else {
            None
        }
    }

    fn expect_token(
        &mut self,
        expected_token_type: TokenType,
        message: &str,
    ) -> Result<Token, SyntaxError> {
        self.match_token(expected_token_type).ok_or(SyntaxError {
            message: message.to_owned(),
            span: self.peek().span.clone(),
        })
    }

    fn declaration(&mut self) -> Result<(), SyntaxError> {
        self.statement()
    }

    fn statement(&mut self) -> Result<(), SyntaxError> {
        match self.peek().token_type {
            TokenType::Print => self.print_statement(),
            _ => self.expression_statement(),
        }
    }

    fn print_statement(&mut self) -> Result<(), SyntaxError> {
        let span = self.next().span;
        self.expression()?;
        self.expect_token(TokenType::Semicolon, "Expect ';' after value.")?;
        self.chunk.add_code(OpCode::Print, span);
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
}
