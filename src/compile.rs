use crate::{chunk::FunctionObject, interner::Interner};
use compiler::Compiler;
use lexer::Lexer;
use std::{
    iter::Peekable,
    mem::{self, discriminant},
    ops::Range,
    rc::Rc,
};
use token::{Token, TokenType};
mod codegen;
mod compiler;
mod expression;
mod lexer;
mod statement;
mod token;

pub fn compile(
    name: &str,
    source: &str,
    interner: &mut Interner,
) -> Result<FunctionObject, Vec<SyntaxError>> {
    let lexer = Lexer::new(source);
    let function_object = FunctionObject::new(&interner.intern(name));

    let mut tokens = lexer.peekable();
    let compiler = Parser::new(source, &mut tokens, function_object, interner);
    compiler.compile()
}

#[derive(Debug)]
pub struct SyntaxError {
    #[expect(unused)]
    message: String,
    #[expect(unused)]
    span: Range<usize>,
}

#[derive(Clone)]
struct Identifier {
    name: Rc<str>,
    span: Range<usize>,
}

impl Identifier {
    fn empty() -> Identifier {
        Identifier {
            name: Rc::from(""),
            span: 0..0,
        }
    }
}

enum FunctionKind {
    Function,
    Script,
}

struct Parser<'a> {
    source: &'a str,
    tokens: &'a mut Peekable<Lexer<'a>>,
    interner: &'a mut Interner,

    errors: Vec<SyntaxError>,
    compiler: Compiler,
}

impl<'a> Parser<'a> {
    fn new(
        source: &'a str,
        tokens: &'a mut Peekable<Lexer<'a>>,
        function_object: FunctionObject,
        interner: &'a mut Interner,
    ) -> Self {
        Self {
            source,
            tokens,
            interner,
            errors: Vec::new(),
            compiler: Compiler::new(function_object, FunctionKind::Script),
        }
    }

    fn compile(mut self) -> Result<FunctionObject, Vec<SyntaxError>> {
        self.compiler.reserve_first_stack_slot();
        loop {
            let next = self.peek();
            if matches!(next.token_type, TokenType::Eof) {
                let span = next.span.clone();
                self.compiler.emit_return(span);
                break;
            }
            if let Err(err) = self.declaration() {
                self.errors.push(err);
                self.synchronize();
            }
        }
        if self.errors.is_empty() {
            Ok(self.compiler.function_object)
        } else {
            Err(self.errors)
        }
    }

    fn enter_context(&mut self, new_context: Compiler) {
        let ctx = mem::replace(&mut self.compiler, new_context);
        self.compiler.enclosing = Some(Box::new(ctx));
    }

    fn exit_context(&mut self) -> Compiler {
        let enclosing = *self
            .compiler
            .enclosing
            .take()
            .expect("can not exit outer context");
        mem::replace(&mut self.compiler, enclosing)
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
            tok => {
                //println!("{tok:?}");
                Ok(tok)
            }
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
}

#[cfg(test)]
mod test {
    use super::compile;
    use crate::interner::Interner;
    #[test]
    fn test_parse_operators() {
        let pairs = [
            ("a + b + c;", "(a + b) + c;"),
            ("a - b - c;", "(a - b) - c;"),
            ("a * b * c;", "(a * b) * c;"),
            ("a / b / c;", "(a / b) / c;"),
            ("a + b * c;", "a + (b * c);"),
            ("a * b + c;", "(a * b) + c;"),
            ("a - b / c;", "a - (b / c);"),
            ("a * b > c;", "(a * b) > c;"),
            ("a + b == c;", "(a + b) == c;"),
            ("!a * b;", "(!a) * b;"),
            ("!a + b;", "(!a) + b;"),
            ("!a > b;", "(!a) > b;"),
            ("!!a;", "!(!a);"),
            ("a - b + c;", "(a - b) + c;"),
            ("a * b / c;", "(a * b) / c;"),
            ("a > b + c;", "a > (b + c);"),
            ("a != b * c;", "a != (b * c);"),
            ("a * !b;", "a * (!b);"),
            ("a + b * c - d / e;", "a + (b * c) - (d / e);"),
            ("!a + b * c;", "(!a) + (b * c);"),
            ("a - b - c - d;", "((a - b) - c) - d;"),
            ("a + b > c + d;", "(a + b) > (c + d);"),
            ("a >= b + c;", "a >= (b + c);"),
            ("a + b <= c;", "(a + b) <= c;"),
            ("a < b > c;", "(a < b) > c;"),
            ("a and b and c;", "(a and b) and c;"),
            ("a and b or c;", "(a and b) or c;"),
            ("a or b and c;", "a or (b and c);"),
        ];
        for (index, &(left, right)) in pairs.iter().enumerate() {
            let init = "var a;var b;var c;var d;var e;";
            let left = &format!("{{{init}{left}}}");
            let right = &format!("{{{init}{right}}}");
            let chunk_left = compile("test", left, &mut Interner::default()).unwrap();
            let chunk_right = compile("test", right, &mut Interner::default()).unwrap();
            assert_eq!(
                chunk_left.chunk.code, chunk_right.chunk.code,
                "case # {index}"
            )
        }
    }
}
