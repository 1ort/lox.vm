use super::super::token::{Token, TokenType};
use super::*;

fn collect_tokens(source: &'_ str) -> Vec<Token<'_>> {
    Lexer::new(source).collect()
}

#[test]
fn empty_source_yields_eof_then_none() {
    let mut lexer = Lexer::new("");
    let eof = lexer.next().unwrap();
    assert!(matches!(eof.token_type, TokenType::Eof));
    assert_eq!(eof.lexeme, "");
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
    assert_eq!(tokens.len(), 6);
    let types: Vec<_> = tokens.iter().map(|t| &t.token_type).collect();
    assert!(matches!(types[0], TokenType::EqualEqual));
    assert!(tokens[0].lexeme == "==");
    assert!(matches!(types[1], TokenType::LessEqual));
    assert!(tokens[1].lexeme == "<=");
    assert!(matches!(types[2], TokenType::GreaterEqual));
    assert!(tokens[2].lexeme == ">=");
    assert!(matches!(types[3], TokenType::BangEqual));
    assert!(tokens[3].lexeme == "!=");
    assert!(matches!(types[4], TokenType::Comment(" comment")));
    assert!(tokens[4].lexeme == "// comment");
    assert!(matches!(types[5], TokenType::Eof));
}

#[test]
fn line_comment_until_newline() {
    let tokens = collect_tokens("// this is a comment(){}+\n+");
    assert_eq!(tokens.len(), 3);
    match tokens[0].token_type {
        TokenType::Comment(" this is a comment(){}+") => {}
        _ => panic!("Expected comment"),
    }
    assert!(matches!(tokens[1].token_type, TokenType::Plus));
    assert!(matches!(tokens[2].token_type, TokenType::Eof));

    assert_eq!(tokens[0].span, 0..25);
    assert_eq!(tokens[0].lexeme, "// this is a comment(){}+");
    assert_eq!(tokens[1].lexeme, "+");
    assert_eq!(tokens[1].span, 26..27)
}

#[test]
fn integer_number_literal() {
    let tokens = collect_tokens("42");
    assert_eq!(tokens.len(), 2);
    match tokens[0].token_type {
        TokenType::Number(s) => assert_eq!(s, "42"),
        _ => panic!("Expected Number"),
    }
    assert_eq!(tokens[0].lexeme, "42");
    assert_eq!(tokens[0].span, 0..2);
}

#[test]
fn integer_number_literal_with_dot() {
    let tokens = collect_tokens("42.");
    assert_eq!(tokens.len(), 3);
    match tokens[0].token_type {
        TokenType::Number(s) => assert_eq!(s, "42"),
        _ => panic!("Expected Number"),
    }
    assert!(matches!(tokens[1].token_type, TokenType::Dot));
    assert_eq!(tokens[0].lexeme, "42");
    assert_eq!(tokens[0].span, 0..2);
    assert_eq!(tokens[1].lexeme, ".");
    assert_eq!(tokens[1].span, 2..3);
}

#[test]
fn fractional_number_literal() {
    let tokens = collect_tokens("42.55");
    assert_eq!(tokens.len(), 2);
    match tokens[0].token_type {
        TokenType::Number(s) => assert_eq!(s, "42.55"),
        _ => panic!("Expected Number"),
    }
    assert_eq!(tokens[0].lexeme, "42.55");
    assert_eq!(tokens[0].span, 0..5);
}

#[test]
fn string_literal() {
    let source = r#""hello world""#;
    let tokens = collect_tokens(source);
    assert_eq!(tokens.len(), 2);
    match tokens[0].token_type {
        TokenType::String(s) => assert_eq!(s, "hello world"),
        _ => panic!("Expected String"),
    }
    assert_eq!(tokens[0].lexeme, source);
    assert_eq!(tokens[0].span, 0..source.len());
}

#[test]
fn unterminated_string_error() {
    let source = r#""missing end"#;
    let tokens = collect_tokens(source);
    assert_eq!(tokens.len(), 2);
    match tokens[0].token_type {
        TokenType::Unexpected(msg) => assert_eq!(msg, "Unterminated string."),
        _ => panic!("Expected Unexpected"),
    }
    assert_eq!(tokens[0].lexeme, "");
    assert_eq!(tokens[0].span, 0..0);
    assert!(matches!(tokens[1].token_type, TokenType::Eof));
}

#[test]
fn string_terminated_in_next_line_error() {
    let source = "\"not terminated\n\"";
    let tokens = collect_tokens(source);
    assert_eq!(tokens.len(), 3);
    println!("{tokens:?}");
    match tokens[0].token_type {
        TokenType::Unexpected(msg) => assert_eq!(msg, "Unterminated string."),
        _ => panic!("Expected Unexpected"),
    }
    match tokens[1].token_type {
        TokenType::Unexpected(msg) => assert_eq!(msg, "Unterminated string."),
        _ => panic!("Expected Unexpected"),
    }
    assert_eq!(tokens[0].lexeme, "");
    assert_eq!(tokens[0].span, 0..0);

    assert_eq!(tokens[1].lexeme, "");
    assert_eq!(tokens[1].span, 16..16);

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
    let expected_names = ["x", "myVar", "_private", "foo", "bar123"];
    for (tok, expected) in tokens.iter().zip(expected_names.iter()) {
        if let TokenType::Identifier(actual) = &tok.token_type {
            assert_eq!(actual, expected);
        } else {
            panic!("Expected identifier, got {:?}", tok.token_type);
        }
    }
    assert!(matches!(tokens[5].token_type, TokenType::Eof));
}

#[test]
fn unknown_character_yields_unknown_token() {
    let tokens = collect_tokens("@");
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens[0].token_type, TokenType::Unknown));
    assert_eq!(tokens[0].lexeme, "@");
    assert!(matches!(tokens[1].token_type, TokenType::Eof));
}

#[test]
fn mixed_tokens_with_spans() {
    let source = "var num = 123;";
    let tokens = collect_tokens(source);
    assert_eq!(tokens.len(), 6);
    assert_eq!(tokens[0].lexeme, "var");
    assert_eq!(tokens[0].span, 0..3);
    assert_eq!(tokens[1].lexeme, "num");
    assert_eq!(tokens[1].span, 4..7);
    assert_eq!(tokens[2].lexeme, "=");
    assert_eq!(tokens[2].span, 8..9);
    assert_eq!(tokens[3].lexeme, "123");
    assert_eq!(tokens[3].span, 10..13);
    assert_eq!(tokens[4].lexeme, ";");
    assert_eq!(tokens[4].span, 13..14);
}
