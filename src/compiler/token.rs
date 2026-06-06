use std::ops::Range;

#[derive(Debug, PartialEq, Clone)]
pub enum TokenType<'a> {
    Eof,
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Literals.
    String(&'a str),
    Number(&'a str),
    Identifier(&'a str),
    // Keywords.
    Print,
    Var,
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Return,
    Super,
    This,
    True,
    While,
    Break,
    Unexpected(&'a str),
    Unknown,
    Comment(&'a str),
}

#[derive(Debug, Clone)]
pub struct Token<'a> {
    pub token_type: TokenType<'a>,
    pub lexeme: &'a str,
    pub span: Range<usize>,
}
