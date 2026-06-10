use super::Parser;
use super::SyntaxError;
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
        let lhs = self.next();
        match lhs.token_type {
            TokenType::Number => {
                let lexeme = self.lexeme(&lhs.span);
                let value: f64 = lexeme.parse().map_err(|err| SyntaxError {
                    message: format!("Can not parse number: {err}."),
                    span: lhs.span.clone(),
                })?;
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
                self.expr_bp(0)?;
                self.expect_token(TokenType::RightParen, "Expect ')' after expression.")?;
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
                self.expr_bp(r_bp)?;
                self.chunk.add_code(opcode, lhs.span);
            }
            _ => {
                return Err(SyntaxError {
                    message: "Unexpected token".to_owned(),
                    span: lhs.span,
                });
            }
        }
        loop {
            let op = self.peek();
            if let Some((l_bp, r_bp)) = infix_binding_power(&op.token_type) {
                if l_bp < min_bp {
                    break;
                }
                let op = self.next();
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
