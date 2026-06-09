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
    let source = "var and class else false fun for if nil or return super this true while break";
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
