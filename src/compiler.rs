mod lexer;
mod parser;
mod token;

use crate::{chunk::Chunk, compiler::parser::SyntaxError, interner::Interner};
use lexer::Lexer;
use parser::Parser;

pub fn compile(source: &str, interner: &mut Interner) -> Result<Chunk, Vec<SyntaxError>> {
    let lexer = Lexer::new(source);
    let mut chunk = Chunk::new();
    let parser = Parser::new(source, lexer.peekable(), &mut chunk, interner);
    parser.compile()?;
    Ok(chunk)
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
            let chunk_left = compile(left, &mut Interner::default()).unwrap();
            let chunk_right = compile(right, &mut Interner::default()).unwrap();
            assert_eq!(chunk_left.code, chunk_right.code, "case # {index}")
        }
    }
}
