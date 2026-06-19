use super::token::{Token, TokenType};
use std::{iter::Peekable, str::Chars};

pub(super) struct Lexer<'a> {
    source: &'a str,
    source_iter: Peekable<Chars<'a>>,
    pos: usize,
    finished: bool,
    peeked_next: Option<Option<char>>,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if self.finished {
            return None;
        }
        let tok = self.lex();
        if matches!(tok.token_type, TokenType::Eof) {
            self.finished = true;
        }
        Some(tok)
    }
}

impl<'a> Lexer<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        Lexer {
            source,
            source_iter: source.chars().peekable(),
            pos: 0,
            finished: false,
            peeked_next: None,
        }
    }

    fn lex(self: &mut Lexer<'a>) -> Token {
        self.skip_spaces();
        if let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.lex_number()
            } else if c == &'"' {
                self.lex_string()
            } else if c.is_ascii_alphanumeric() || matches!(c, '_') {
                self.lex_keyword_or_identifier()
            } else {
                self.lex_symbol()
            }
        } else {
            Token {
                token_type: TokenType::Eof,
                span: self.pos..self.pos,
            }
        }
    }

    fn lex_number(self: &mut Lexer<'a>) -> Token {
        let token_start = self.pos;
        self.take_till(|c| c.is_ascii_digit());
        if self.peek_char().is_some_and(|c| c == &'.')
            && self.peek_two_chars().is_some_and(|c| c.is_ascii_digit())
        {
            self.next_char();
            self.take_till(|c| c.is_ascii_digit());
        }
        Token {
            token_type: TokenType::Number,
            span: token_start..self.pos,
        }
    }

    fn lex_string(self: &mut Lexer<'a>) -> Token {
        self.next_char()
            .expect("should check that input is not empty");

        let content_start = self.pos;
        self.take_till(|c| c.ne(&'"') && c.ne(&'\n'));
        let content_span = content_start..self.pos;

        if self.match_next_char('"') {
            Token {
                token_type: TokenType::String,
                span: content_span,
            }
        } else {
            Token {
                token_type: TokenType::UnterminatedString,
                span: content_start..content_start,
            }
        }
    }

    fn lex_keyword_or_identifier(self: &mut Lexer<'a>) -> Token {
        let token_start = self.pos;
        let lexeme = self.take_till(|c| c.is_ascii_alphanumeric() || matches!(c, '_'));
        Token {
            token_type: match lexeme {
                "print" => TokenType::Print,
                "var" => TokenType::Var,
                "and" => TokenType::And,
                "class" => TokenType::Class,
                "else" => TokenType::Else,
                "false" => TokenType::False,
                "fun" => TokenType::Fun,
                "for" => TokenType::For,
                "if" => TokenType::If,
                "nil" => TokenType::Nil,
                "or" => TokenType::Or,
                "return" => TokenType::Return,
                "super" => TokenType::Super,
                "this" => TokenType::This,
                "true" => TokenType::True,
                "while" => TokenType::While,
                "break" => TokenType::Break,
                "continue" => TokenType::Continue,
                _ => TokenType::Identifier,
            },
            span: token_start..self.pos,
        }
    }

    fn lex_symbol(self: &mut Lexer<'a>) -> Token {
        let token_start = self.pos;
        let c = self
            .next_char()
            .expect("should check that input is not empty");
        let token_type = match c {
            '(' => TokenType::LeftParen,
            ')' => TokenType::RightParen,
            '{' => TokenType::LeftBrace,
            '}' => TokenType::RightBrace,
            ',' => TokenType::Comma,
            '.' => TokenType::Dot,
            '-' => TokenType::Minus,
            '+' => TokenType::Plus,
            ';' => TokenType::Semicolon,
            '*' => TokenType::Star,
            '=' => {
                if self.match_next_char('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                }
            }
            '<' => {
                if self.match_next_char('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                }
            }
            '>' => {
                if self.match_next_char('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                }
            }
            '!' => {
                if self.match_next_char('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                }
            }
            '/' => {
                if self.match_next_char('/') {
                    self.take_till(|c| c.ne(&'\n'));
                    return self.lex();
                } else {
                    TokenType::Slash
                }
            }
            _ => TokenType::Unknown,
        };

        Token {
            token_type,
            span: token_start..self.pos,
        }
    }

    fn take_till(self: &mut Lexer<'a>, till: impl Fn(&char) -> bool) -> &'a str {
        let start = self.pos;
        while let Some(c) = self.peek_char()
            && till(c)
        {
            self.next_char();
        }
        &self.source[start..self.pos]
    }

    fn skip_spaces(self: &mut Lexer<'a>) {
        self.skip_till(|c| c.is_whitespace());
    }

    fn skip_till(self: &mut Lexer<'a>, till: impl Fn(&char) -> bool) {
        while let Some(c) = self.peek_char()
            && till(c)
        {
            self.next_char();
        }
    }

    fn next_char(&mut self) -> Option<char> {
        match self.peeked_next {
            Some(x) => {
                self.peeked_next = None;
                self.pos += 1;
                x
            }
            None => self.source_iter.next().inspect(|_| {
                self.pos += 1;
            }),
        }
    }

    fn peek_char(&mut self) -> Option<&char> {
        match self.peeked_next {
            Some(ref x) => x.as_ref(),
            None => self.source_iter.peek(),
        }
    }

    fn peek_two_chars(&mut self) -> Option<&char> {
        if self.peeked_next.is_none() {
            self.peeked_next = Some(self.source_iter.next());
        }
        self.source_iter.peek()
    }

    fn match_next_char(self: &mut Lexer<'a>, expected: char) -> bool {
        if let Some(next) = self.peek_char()
            && *next == expected
        {
            self.next_char();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::token::{Token, TokenType};
    use super::*;

    fn collect_tokens(source: &'_ str) -> Vec<Token> {
        Lexer::new(source).collect()
    }

    #[test]
    fn empty_source_yields_eof_then_none() {
        let mut lexer = Lexer::new("");
        let eof = lexer.next().unwrap();
        assert!(matches!(eof.token_type, TokenType::Eof));
        assert_eq!(eof.span, 0..0);
        assert!(lexer.next().is_none());
        assert!(lexer.next().is_none());
    }

    #[test]
    fn whitespace_is_skipped() {
        let tokens = collect_tokens("   \t\n  ");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].token_type, TokenType::Eof));
    }

    #[test]
    fn single_symbol_tokens() {
        let tokens = collect_tokens("(){},.-+;*");
        let expected = vec![
            TokenType::LeftParen,
            TokenType::RightParen,
            TokenType::LeftBrace,
            TokenType::RightBrace,
            TokenType::Comma,
            TokenType::Dot,
            TokenType::Minus,
            TokenType::Plus,
            TokenType::Semicolon,
            TokenType::Star,
            TokenType::Eof,
        ];
        assert_eq!(tokens.len(), expected.len());
        for (tok, exp) in tokens.iter().zip(expected.iter()) {
            assert!(std::mem::discriminant(&tok.token_type) == std::mem::discriminant(exp));
        }
    }

    #[test]
    fn two_char_symbols() {
        let tokens = collect_tokens("== <= >= != // comment\n");
        assert_eq!(tokens.len(), 5);
        let types: Vec<_> = tokens.iter().map(|t| &t.token_type).collect();
        assert!(matches!(types[0], TokenType::EqualEqual));
        assert!(matches!(types[1], TokenType::LessEqual));
        assert!(matches!(types[2], TokenType::GreaterEqual));
        assert!(matches!(types[3], TokenType::BangEqual));
        assert!(matches!(types[4], TokenType::Eof));
    }

    #[test]
    fn line_comment_until_newline() {
        let tokens = collect_tokens("// this is a comment(){}+\n+");
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token_type, TokenType::Plus));
        assert!(matches!(tokens[1].token_type, TokenType::Eof));

        assert_eq!(tokens[0].span, 26..27)
    }

    #[test]
    fn integer_number_literal() {
        let tokens = collect_tokens("42");
        assert_eq!(tokens.len(), 2);
        match tokens[0].token_type {
            TokenType::Number => {}
            _ => panic!("Expected Number"),
        }
        assert_eq!(tokens[0].span, 0..2);
    }

    #[test]
    fn integer_number_literal_with_dot() {
        let tokens = collect_tokens("42.");
        assert_eq!(tokens.len(), 3);
        match tokens[0].token_type {
            TokenType::Number => {}
            _ => panic!("Expected Number"),
        }
        assert!(matches!(tokens[1].token_type, TokenType::Dot));
        assert_eq!(tokens[0].span, 0..2);
        assert_eq!(tokens[1].span, 2..3);
    }

    #[test]
    fn fractional_number_literal() {
        let tokens = collect_tokens("42.55");
        assert_eq!(tokens.len(), 2);
        match tokens[0].token_type {
            TokenType::Number => {}
            _ => panic!("Expected Number"),
        }
        assert_eq!(tokens[0].span, 0..5);
    }

    #[test]
    fn string_literal() {
        let source = r#""1234567890""#;
        let tokens = collect_tokens(source);
        assert_eq!(tokens.len(), 2);
        match tokens[0].token_type {
            TokenType::String => {}
            _ => panic!("Expected String"),
        }
        assert_eq!(tokens[0].span, 1..11);
    }

    #[test]
    fn unterminated_string_error() {
        let source = r#""missing end"#;
        let tokens = collect_tokens(source);
        assert_eq!(tokens.len(), 2);
        match tokens[0].token_type {
            TokenType::UnterminatedString => {}
            _ => panic!("Expected UnterminatedString"),
        }
        assert_eq!(tokens[0].span, 1..1);
        assert!(matches!(tokens[1].token_type, TokenType::Eof));
    }

    #[test]
    fn string_terminated_in_next_line_error() {
        let source = "\"not terminated\n\"";
        let tokens = collect_tokens(source);
        assert_eq!(tokens.len(), 3);
        println!("{tokens:?}");
        match tokens[0].token_type {
            TokenType::UnterminatedString => {}
            _ => panic!("Expected UnterminatedString"),
        }
        match tokens[1].token_type {
            TokenType::UnterminatedString => {}
            _ => panic!("Expected UnterminatedString"),
        }
        assert_eq!(tokens[0].span, 1..1);
        assert_eq!(tokens[1].span, 17..17);

        assert!(matches!(tokens[2].token_type, TokenType::Eof));
    }

    #[test]
    fn keywords() {
        let source =
            "var and class else false fun for if nil or return super this true while break";
        let tokens = collect_tokens(source);
        let expected = vec![
            TokenType::Var,
            TokenType::And,
            TokenType::Class,
            TokenType::Else,
            TokenType::False,
            TokenType::Fun,
            TokenType::For,
            TokenType::If,
            TokenType::Nil,
            TokenType::Or,
            TokenType::Return,
            TokenType::Super,
            TokenType::This,
            TokenType::True,
            TokenType::While,
            TokenType::Break,
            TokenType::Eof,
        ];
        assert_eq!(tokens.len(), expected.len());
        for (tok, exp) in tokens.iter().zip(expected.iter()) {
            assert_eq!(
                std::mem::discriminant(&tok.token_type),
                std::mem::discriminant(exp)
            );
        }
    }

    #[test]
    fn identifiers() {
        let source = "x myVar _private foo bar123";
        let tokens = collect_tokens(source);
        assert_eq!(tokens.len(), 6);
        for tok in &tokens[..5] {
            assert!(matches!(tok.token_type, TokenType::Identifier));
        }
        assert!(matches!(tokens[5].token_type, TokenType::Eof));
    }

    #[test]
    fn unknown_character_yields_unknown_token() {
        let tokens = collect_tokens("@");
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0].token_type, TokenType::Unknown));
        assert!(matches!(tokens[1].token_type, TokenType::Eof));
    }

    #[test]
    fn mixed_tokens_with_spans() {
        let source = "var num = 123;";
        let tokens = collect_tokens(source);
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].span, 0..3);
        assert_eq!(tokens[1].span, 4..7);
        assert_eq!(tokens[2].span, 8..9);
        assert_eq!(tokens[3].span, 10..13);
        assert_eq!(tokens[4].span, 13..14);
    }
}
