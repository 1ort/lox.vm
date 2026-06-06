mod lexer;
mod token;
use crate::{chunk::Chunk, compiler::token::TokenType, opcode::OpCode};
use core::panic;
use lexer::Lexer;
use std::{iter::Peekable, mem::discriminant};
use token::Token;

pub fn compile(source: &str) -> Chunk {
    let lexer = Lexer::new(source);
    let mut chunk = Chunk::new();
    let mut parser = Parser {
        tokens: lexer.peekable(),
        chunk: &mut chunk,
    };
    parser.compile();
    chunk
}

struct Parser<'a> {
    tokens: Peekable<Lexer<'a>>,
    chunk: &'a mut Chunk,
}

impl<'a> Parser<'a> {
    fn compile(&mut self) {
        self.expression();
        let eof = self.tokens.next().expect("Expect EOF");
        self.chunk.add_code(OpCode::Return, eof.span);
    }

    fn expression(&mut self) {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let lhs = self.tokens.next().expect("Expect token.");
        match &lhs.token_type {
            TokenType::Number(lexeme) => {
                let value: f64 = lexeme.parse().expect("Expect number literal");
                self.chunk.add_constant(value, lhs.span.clone());
            }
            TokenType::LeftParen => {
                self.expr_bp(0);
                self.expect_token(&TokenType::RightParen, "Expect ')' after expression.")
                    .expect("expect )");
            }
            token_type @ (TokenType::Minus | TokenType::Bang) => {
                let (_, r_bp) = prefix_binding_power(token_type).unwrap_or_else(|| {
                    panic!("expected binding power for {token_type:?}");
                });
                let opcode = match token_type {
                    TokenType::Minus => OpCode::Negate,
                    _ => {
                        panic!("expected opcode for {lhs:?}")
                    }
                };
                self.expr_bp(r_bp);
                self.chunk.add_code(opcode, lhs.span);
            }
            _ => panic!("Bad token: {lhs:?}"),
        }
        loop {
            let op = self.tokens.peek().expect("Expect token");
            if matches!(op.token_type, TokenType::Eof) {
                break;
            } else if let Some((l_bp, r_bp)) = infix_binding_power(&op.token_type) {
                if l_bp < min_bp {
                    break;
                }
                let op = self.tokens.next().expect("Expect token");
                self.expr_bp(r_bp);
                let opcode = match op.token_type {
                    TokenType::Minus => OpCode::Subtract,
                    TokenType::Plus => OpCode::Add,
                    TokenType::Slash => OpCode::Divide,
                    TokenType::Star => OpCode::Multiply,
                    _ => {
                        panic!("expected opcode for {op:?}")
                    }
                };
                self.chunk.add_code(opcode, op.span);
            } else {
                break;
            }
        }
    }

    fn expect_token(
        &mut self,
        expected_token_type: &TokenType,
        message: &str,
    ) -> Result<(), String> {
        match self.tokens.peek() {
            Some(Token { token_type, .. })
                if discriminant(expected_token_type) == discriminant(token_type) =>
            {
                self.tokens.next();
                Ok(())
            }
            _ => Err(message.to_owned()),
        }
    }
}

fn prefix_binding_power(token_type: &TokenType) -> Option<((), u8)> {
    match token_type {
        TokenType::Minus => Some(((), 9)),
        _ => None,
    }
}

fn infix_binding_power(token_type: &TokenType) -> Option<(u8, u8)> {
    let bp = match token_type {
        TokenType::Plus | TokenType::Minus => (5, 6),
        TokenType::Star | TokenType::Slash => (7, 8),
        _ => {
            return None;
        }
    };
    Some(bp)
}
