use super::lexer::Lexer;
use super::token::Token;
use crate::{chunk::Chunk, compiler::token::TokenType, interner::Interner, opcode::OpCode};
use std::{iter::Peekable, mem::discriminant, ops::Range, rc::Rc};

mod expression;
mod statement;

#[derive(Clone)]
struct Identifier {
    name: Rc<str>,
    span: Range<usize>,
}

struct Local {
    identifier: Identifier,
    depth: usize,
    initialized: bool,
}

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
    locals: Vec<Local>,
    scope_depth: usize,
    errors: Vec<SyntaxError>,
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
            locals: Vec::new(),
            scope_depth: 0,
            errors: Vec::new(),
        }
    }

    pub(super) fn compile(mut self) -> Result<(), Vec<SyntaxError>> {
        loop {
            let next = self.peek();
            if matches!(next.token_type, TokenType::Eof) {
                break;
            }
            if let Err(err) = self.declaration() {
                self.errors.push(err);
                self.synchronize();
            }
        }
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors)
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

    fn add_local(&mut self, identifier: &Identifier) -> Result<usize, SyntaxError> {
        for local in self.locals.iter().rev() {
            if local.depth < self.scope_depth {
                break;
            }
            if local.identifier.name.eq(&identifier.name) {
                return Err(SyntaxError {
                    message: "Already a variable with this name in this scope.".to_owned(),
                    span: identifier.span.clone(),
                });
            }
        }

        if self.locals.len() < 2usize.pow(16) {
            let local = Local {
                identifier: identifier.clone(),
                depth: self.scope_depth,
                initialized: false,
            };
            self.locals.push(local);
            Ok(self.locals.len() - 1)
        } else {
            panic!("Too many local variables in function.")
        }
    }

    fn resolve_local(
        &mut self,
        name: &Rc<str>,
        span: Range<usize>,
    ) -> Result<Option<u16>, SyntaxError> {
        for (stack_index, local) in self.locals.iter().enumerate().rev() {
            if local.identifier.name.eq(name) {
                if !local.initialized {
                    return Err(SyntaxError {
                        message: "Can't read local variable in its own initializer.".to_owned(),
                        span,
                    });
                } else {
                    return Ok(Some(stack_index as u16));
                }
            }
        }
        Ok(None)
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self, span: &Range<usize>) {
        while self
            .locals
            .last()
            .is_some_and(|loc| loc.depth == self.scope_depth)
        {
            self.locals.pop().expect("locals.last() should be Some()");
            self.chunk.add_code(OpCode::Pop, span.clone());
        }

        self.scope_depth -= 1;
    }
}
