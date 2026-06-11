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
            ("1 + 2 + 3;", "(1 + 2) + 3;"),
            ("1 - 2 - 3;", "(1 - 2) - 3;"),
            ("1 * 2 * 3;", "(1 * 2) * 3;"),
            ("1 / 2 / 3;", "(1 / 2) / 3;"),
            ("1 + 2 * 3;", "1 + (2 * 3);"),
            ("1 * 2 + 3;", "(1 * 2) + 3;"),
            ("1 - 2 / 3;", "1 - (2 / 3);"),
            ("1 * 2 > 3;", "(1 * 2) > 3;"),
            ("1 + 2 == 3;", "(1 + 2) == 3;"),
            ("!1 * 2;", "(!1) * 2;"),
            ("!1 + 2;", "(!1) + 2;"),
            ("!1 > 2;", "(!1) > 2;"),
            ("!!1;", "!(!1);"),
            ("1 - 2 + 3;", "(1 - 2) + 3;"),
            ("1 * 2 / 3;", "(1 * 2) / 3;"),
            ("1 > 2 + 3;", "1 > (2 + 3);"),
            ("1 != 2 * 3;", "1 != (2 * 3);"),
            ("1 * !2;", "1 * (!2);"),
            ("1 + 2 * 3 - 4 / 5;", "1 + (2 * 3) - (4 / 5);"),
            ("!1 + 2 * 3;", "(!1) + (2 * 3);"),
            ("1 - 2 - 3 - 4;", "((1 - 2) - 3) - 4;"),
            ("1 + 2 > 3 + 4;", "(1 + 2) > (3 + 4);"),
            ("1 >= 2 + 3;", "1 >= (2 + 3);"),
            ("1 + 2 <= 3;", "(1 + 2) <= 3;"),
            ("1 < 2 > 3;", "(1 < 2) > 3;"),
        ];
        for (index, &(left, right)) in pairs.iter().enumerate() {
            let chunk_left = compile(left, &mut Interner::default()).unwrap();
            let chunk_right = compile(right, &mut Interner::default()).unwrap();

            assert_eq!(chunk_left.code, chunk_right.code, "case # {index}")
        }
    }
}
