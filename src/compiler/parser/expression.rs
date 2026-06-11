use super::Parser;
use super::SyntaxError;
use crate::compiler::token;
use crate::compiler::token::TokenType;
use crate::opcode::OpCode;
use crate::value::Value;

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

impl<'a> Parser<'a> {
    pub(super) fn expression(&mut self) -> Result<(), SyntaxError> {
        self.expr_bp(0)
    }

    pub(super) fn expr_bp(&mut self, min_bp: u8) -> Result<(), SyntaxError> {
        match self.peek().token_type.clone() {
            TokenType::Number => {
                let span = self.next()?.span;
                let lexeme = self.lexeme(&span);
                let value: f64 = lexeme.parse().map_err(|err| SyntaxError {
                    message: format!("Can not parse number: {err}."),
                    span: span.clone(),
                })?;
                self.chunk.add_constant(value, span);
            }
            TokenType::String => {
                let span = self.next()?.span;
                let lexeme = self.lexeme(&span);
                let value: Value = self.interner.intern(lexeme).into();
                self.chunk.add_constant(value, span);
            }
            TokenType::UnterminatedString => {
                let span = self.peek().span.clone();
                return Err(SyntaxError {
                    message: "Unterminated string".to_owned(),
                    span,
                });
            }
            TokenType::True => {
                let span = self.next()?.span;
                self.chunk.add_code(OpCode::True, span);
            }
            TokenType::False => {
                let span = self.next()?.span;
                self.chunk.add_code(OpCode::False, span);
            }
            TokenType::Nil => {
                let span = self.next()?.span;
                self.chunk.add_code(OpCode::Nil, span);
            }
            TokenType::LeftParen => {
                let _ = self.next();
                self.expr_bp(0)?;
                self.expect_token(TokenType::RightParen, "Expect ')' after expression.")?;
            }
            TokenType::Minus | TokenType::Bang => {
                let token = self.next()?;
                let (_, r_bp) = prefix_binding_power(&token.token_type)
                    .expect("should be binding power for prefix op");
                let opcode = match token.token_type {
                    TokenType::Minus => OpCode::Negate,
                    TokenType::Bang => OpCode::Not,
                    _ => {
                        panic!("expected opcode for {token:?}")
                    }
                };
                self.expr_bp(r_bp)?;
                self.chunk.add_code(opcode, token.span);
            }
            TokenType::Identifier => {
                let span = self.next()?.span;
                let lexeme = self.lexeme(&span);
                let value: Value = self.interner.intern(lexeme).into();
                let arg = self.chunk.add_constant(value, span.clone());
                self.chunk.get_global(arg, span.clone());
            }
            token => {
                eprintln!("{token:?}");
                let span = self.peek().span.clone();
                return Err(SyntaxError {
                    message: "Expected expression".to_owned(),
                    span,
                });
            }
        }
        loop {
            let op = self.peek();
            if let Some((l_bp, r_bp)) = infix_binding_power(&op.token_type) {
                if l_bp < min_bp {
                    break;
                }
                let op = self.next()?;
                self.expr_bp(r_bp)?;
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

        Ok(())
    }
}
