use crate::{
    chunk::{Chunk, FunctionObject},
    interner::Interner,
    opcode::OpCode,
};
use lexer::Lexer;
use std::{iter::Peekable, mem::discriminant, ops::Range, rc::Rc};
use token::{Token, TokenType};

mod expression;
mod lexer;
mod statement;
mod token;

#[derive(Debug)]
pub struct SyntaxError {
    #[expect(unused)]
    message: String,
    #[expect(unused)]
    span: Range<usize>,
}

pub fn compile(
    name: &str,
    source: &str,
    interner: &mut Interner,
) -> Result<FunctionObject, Vec<SyntaxError>> {
    let lexer = Lexer::new(source);
    let function_object = FunctionObject {
        arity: 0,
        chunk: Chunk::new(),
        name: interner.intern(name),
    };

    let mut tokens = lexer.peekable();
    let compiler = Compiler::new(source, &mut tokens, function_object, interner);
    compiler.compile()
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

struct Local {
    identifier: Identifier,
    depth: usize,
    initialized: bool,
}

struct LoopContext {
    stack_depth_at_start: usize,
    break_patches: Vec<usize>,
    loop_start: usize,
}

enum FunctionKind {
    Function,
    Script,
}

struct CompilerContext {
    #[expect(unused)]
    function_kind: FunctionKind,
    locals: Vec<Local>,
    scope_depth: usize,
    loop_context: Option<LoopContext>,
}

impl CompilerContext {
    fn new(function_kind: FunctionKind) -> Self {
        CompilerContext {
            function_kind,
            locals: Vec::new(),
            scope_depth: 0,
            loop_context: None,
        }
    }
}

struct Compiler<'a> {
    source: &'a str,
    function_object: FunctionObject,
    tokens: &'a mut Peekable<Lexer<'a>>,
    interner: &'a mut Interner,

    errors: Vec<SyntaxError>,
    context: CompilerContext,
}

impl<'a> Compiler<'a> {
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
            function_object,
            context: CompilerContext::new(FunctionKind::Script),
        }
    }

    fn compile(mut self) -> Result<FunctionObject, Vec<SyntaxError>> {
        self.reserve_first_stack_slot();
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
            Ok(self.function_object)
        } else {
            Err(self.errors)
        }
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.function_object.chunk
    }

    fn reserve_first_stack_slot(&mut self) {
        self.add_local(&Identifier::empty())
            .expect("this should be first local variable in context");
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

    fn add_local(&mut self, identifier: &Identifier) -> Result<usize, SyntaxError> {
        for local in self.context.locals.iter().rev() {
            if local.depth < self.context.scope_depth {
                break;
            }
            if local.identifier.name.eq(&identifier.name) {
                return Err(SyntaxError {
                    message: "Already a variable with this name in this scope.".to_owned(),
                    span: identifier.span.clone(),
                });
            }
        }

        if self.context.locals.len() < 2usize.pow(16) {
            let local = Local {
                identifier: identifier.clone(),
                depth: self.context.scope_depth,
                initialized: false,
            };
            self.context.locals.push(local);
            Ok(self.context.locals.len() - 1)
        } else {
            panic!("Too many local variables in function.")
        }
    }

    fn resolve_local(
        &mut self,
        name: &Rc<str>,
        span: Range<usize>,
    ) -> Result<Option<u16>, SyntaxError> {
        for (stack_index, local) in self.context.locals.iter().enumerate().rev() {
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
        self.context.scope_depth += 1;
    }

    fn end_scope(&mut self, span: &Range<usize>) {
        while self
            .context
            .locals
            .last()
            .is_some_and(|loc| loc.depth == self.context.scope_depth)
        {
            self.context
                .locals
                .pop()
                .expect("locals.last() should be Some()");
            self.current_chunk().add_code(OpCode::Pop, span.clone());
        }

        self.context.scope_depth -= 1;
    }

    fn emit_jump(&mut self, opcode: impl Into<u8>, span: Range<usize>) -> usize {
        self.current_chunk().add_code(opcode, span.clone());
        self.current_chunk().add_code(0xff, span.clone());
        self.current_chunk().add_code(0xff, span);
        self.current_chunk().code.len() - 2
    }

    fn emit_loop(&mut self, loop_start: usize, span: Range<usize>) {
        self.current_chunk().add_code(OpCode::Loop, span.clone());
        let jump = self.current_chunk().code.len() - loop_start + 2;
        if jump >= 2usize.pow(16) {
            panic!("Too much code to jump over.")
        }
        let jump_bytes: [u8; 2] = (jump as u16).to_le_bytes();
        self.current_chunk().add_code(jump_bytes[0], span.clone());
        self.current_chunk().add_code(jump_bytes[1], span);
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.current_chunk().code.len() - offset - 2;
        if jump >= 2usize.pow(16) {
            panic!("Too much code to jump over.")
        }
        let jump_bytes: [u8; 2] = (jump as u16).to_le_bytes();
        self.current_chunk().code[offset] = jump_bytes[0];
        self.current_chunk().code[offset + 1] = jump_bytes[1];
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
