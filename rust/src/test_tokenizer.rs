// 一時的なテストファイル：コメント処理の検証用

use crate::tokenizer::tokenize_with_custom_words;
use crate::types::Token;
use std::collections::HashSet;

#[test]
fn test_comment_basic() {
    let custom_words = HashSet::new();

    // 基本的なコメント
    let result = tokenize_with_custom_words("# これはコメント\n[ 1 ]", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::VectorStart(crate::types::BracketType::Square),
        Token::Number("1".to_string()),
        Token::VectorEnd(crate::types::BracketType::Square),
    ]);
}

#[test]
fn test_comment_inline() {
    let custom_words = HashSet::new();

    // 行末コメント
    let result = tokenize_with_custom_words("[ 1 ] # コメント\n[ 2 ]", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::VectorStart(crate::types::BracketType::Square),
        Token::Number("1".to_string()),
        Token::VectorEnd(crate::types::BracketType::Square),
        Token::LineBreak,
        Token::VectorStart(crate::types::BracketType::Square),
        Token::Number("2".to_string()),
        Token::VectorEnd(crate::types::BracketType::Square),
    ]);
}

#[test]
fn test_comment_no_newline() {
    let custom_words = HashSet::new();

    // 改行なしコメント
    let result = tokenize_with_custom_words("[ 1 ] # コメント", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::VectorStart(crate::types::BracketType::Square),
        Token::Number("1".to_string()),
        Token::VectorEnd(crate::types::BracketType::Square),
    ]);
}

#[test]
fn test_multiple_comments() {
    let custom_words = HashSet::new();

    // 複数のコメント
    let result = tokenize_with_custom_words(
        "# コメント1\n# コメント2\n[ 1 ]",
        &custom_words
    ).unwrap();
    assert_eq!(result, vec![
        Token::VectorStart(crate::types::BracketType::Square),
        Token::Number("1".to_string()),
        Token::VectorEnd(crate::types::BracketType::Square),
    ]);
}

#[test]
fn test_comment_with_sharp_in_string() {
    let custom_words = HashSet::new();

    // 文字列内の#は無視されない
    let result = tokenize_with_custom_words("'text#comment'", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("text#comment".to_string()),
    ]);
}
