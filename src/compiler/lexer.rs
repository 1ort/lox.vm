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
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
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

    fn lex(self: &mut Lexer<'a>) -> Token<'a> {
        self.skip_spaces();

        let tok_start = self.pos;
        if let Some(c) = self.peek_char() {
            let token_type: TokenType<'a> = {
                if c.is_ascii_digit() {
                    self.lex_number()
                } else if c == &'"' {
                    self.lex_string()
                } else if c.is_ascii_alphanumeric() || matches!(c, '_') {
                    self.lex_keyword_or_identifier()
                } else {
                    self.lex_symbol()
                }
            };
            match token_type {
                TokenType::Unexpected(_) => Token {
                    token_type,
                    lexeme: "",
                    span: tok_start..tok_start,
                },
                _ => {
                    let span = tok_start..self.pos;
                    Token {
                        token_type,
                        span: span.clone(),
                        lexeme: &self.source[span],
                    }
                }
            }
        } else {
            Token {
                token_type: TokenType::Eof,
                lexeme: "",
                span: tok_start..tok_start,
            }
        }
    }

    fn lex_number(self: &mut Lexer<'a>) -> TokenType<'a> {
        let tok_start = self.pos;
        let integer_part = self.take_till(|c| c.is_ascii_digit());
        if self.peek_char().is_some_and(|c| c == &'.')
            && self.peek_two_chars().is_some_and(|c| c.is_ascii_digit())
        {
            self.next_char();
            self.take_till(|c| c.is_ascii_digit());
            let span = tok_start..self.pos;
            TokenType::Number(&self.source[span])
        } else {
            TokenType::Number(integer_part)
        }
    }

    fn lex_string(self: &mut Lexer<'a>) -> TokenType<'a> {
        self.next_char()
            .expect("should check that input is not empty");
        let content = self.take_till(|c| c.ne(&'"') && c.ne(&'\n'));
        if self.match_next_char('"') {
            TokenType::String(content)
        } else {
            TokenType::Unexpected("Unterminated string.")
        }
    }

    fn lex_keyword_or_identifier(self: &mut Lexer<'a>) -> TokenType<'a> {
        let lexeme = self.take_till(|c| c.is_ascii_alphanumeric() || matches!(c, '_'));
        match lexeme {
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
            _ => TokenType::Identifier(lexeme),
        }
    }

    fn lex_symbol(self: &mut Lexer<'a>) -> TokenType<'a> {
        let c = self
            .next_char()
            .expect("should check that input is not empty");
        match c {
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
                    let lexeme = self.take_till(|c| c.ne(&'\n'));
                    TokenType::Comment(lexeme)
                } else {
                    TokenType::Slash
                }
            }
            _ => TokenType::Unknown,
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
mod tests;
