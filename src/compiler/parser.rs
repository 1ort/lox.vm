use super::lexer::Lexer;
use super::token::Token;
use crate::{
    chunk::Chunk, compiler::token::TokenType, interner::Interner, opcode::OpCode, value::Value,
};
use std::{iter::Peekable, mem::discriminant, ops::Range};

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

    pub(super) fn compile(&mut self) {
        self.expression();
        let eof = self.tokens.next().expect("Expect EOF");
        match eof.token_type {
            TokenType::Eof => {}
            _ => panic!("Expect EOF, got {eof:?}"),
        }
        self.chunk.add_code(OpCode::Return, eof.span);
    }

    fn expression(&mut self) {
        self.expr_bp(0)
    }

    fn lexeme(&self, span: &Range<usize>) -> &'a str {
        &self.source[span.clone()]
    }

    fn expr_bp(&mut self, min_bp: u8) {
        let lhs = self.tokens.next().expect("Expect token.");
        match lhs.token_type {
            TokenType::Number => {
                let lexeme = self.lexeme(&lhs.span);
                let value: f64 = lexeme.parse().expect("Expect number literal");
                self.chunk.add_constant(value, lhs.span.clone());
            }
            TokenType::String => {
                let lexeme = self.lexeme(&lhs.span);
                let value: Value = self.interner.intern(lexeme).into();
                self.chunk.add_constant(value, lhs.span.clone());
            }
            TokenType::True => {
                self.chunk.add_code(OpCode::True, lhs.span.clone());
            }
            TokenType::False => {
                self.chunk.add_code(OpCode::False, lhs.span.clone());
            }
            TokenType::Nil => {
                self.chunk.add_code(OpCode::Nil, lhs.span.clone());
            }
            TokenType::LeftParen => {
                self.expr_bp(0);
                self.expect_token(&TokenType::RightParen, "Expect ')' after expression.")
                    .expect("expect )");
            }
            ref token_type @ (TokenType::Minus | TokenType::Bang) => {
                let (_, r_bp) = prefix_binding_power(token_type).unwrap_or_else(|| {
                    panic!("expected binding power for {token_type:?}");
                });
                let opcode = match token_type {
                    TokenType::Minus => OpCode::Negate,
                    TokenType::Bang => OpCode::Not,
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
            if let Some((l_bp, r_bp)) = infix_binding_power(&op.token_type) {
                if l_bp < min_bp {
                    break;
                }
                let op = self.tokens.next().expect("Expect token");
                self.expr_bp(r_bp);
                let opcodes: &[OpCode] = match op.token_type {
                    TokenType::Minus => &[OpCode::Subtract],
                    TokenType::Plus => &[OpCode::Add],
                    TokenType::Slash => &[OpCode::Divide],
                    TokenType::Star => &[OpCode::Multiply],
                    TokenType::Less => &[OpCode::Less],
                    TokenType::Greater => &[OpCode::Greater],
                    TokenType::EqualEqual => &[OpCode::Equal],
                    TokenType::GreaterEqual => &[OpCode::Less, OpCode::Not],
                    TokenType::LessEqual => &[OpCode::Less, OpCode::Not],
                    TokenType::BangEqual => &[OpCode::Equal, OpCode::Not],
                    _ => {
                        panic!("expected opcode for {op:?}")
                    }
                };
                for code in opcodes.iter().cloned() {
                    self.chunk.add_code(code, op.span.clone());
                }
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
        TokenType::Minus | TokenType::Bang => Some(((), 40)),
        _ => None,
    }
}

fn infix_binding_power(token_type: &TokenType) -> Option<(u8, u8)> {
    let bp = match token_type {
        TokenType::Star | TokenType::Slash => (29, 30),
        TokenType::Plus | TokenType::Minus => (19, 20),
        TokenType::EqualEqual
        | TokenType::Less
        | TokenType::LessEqual
        | TokenType::Greater
        | TokenType::GreaterEqual
        | TokenType::BangEqual => (9, 10),
        _ => {
            return None;
        }
    };
    Some(bp)
}
