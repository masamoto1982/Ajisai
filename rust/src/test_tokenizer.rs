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

// 柔軟な文字列クォートのテスト
#[test]
fn test_flexible_quotes_single_with_double_inside() {
    let custom_words = HashSet::new();

    // シングルクォートで囲み、内部にダブルクォート
    let result = tokenize_with_custom_words("'私の名前は\"昌幹\"です。'", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("私の名前は\"昌幹\"です。".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_double_with_single_inside() {
    let custom_words = HashSet::new();

    // ダブルクォートで囲み、内部にシングルクォート
    let result = tokenize_with_custom_words("\"私の名前は'昌幹'です。\"", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("私の名前は'昌幹'です。".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_single_with_single_inside() {
    let custom_words = HashSet::new();

    // シングルクォートで囲み、内部にもシングルクォート
    let result = tokenize_with_custom_words("'私の名前は'昌幹'です。'", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("私の名前は'昌幹'です。".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_double_with_double_inside() {
    let custom_words = HashSet::new();

    // ダブルクォートで囲み、内部にもダブルクォート
    let result = tokenize_with_custom_words("\"私の名前は\"昌幹\"です。\"", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("私の名前は\"昌幹\"です。".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_complex_nested() {
    let custom_words = HashSet::new();

    // 複雑な入れ子構造
    let result = tokenize_with_custom_words("'私の名前は\"山城'やましろ'昌幹'まさもと'\"です。'", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("私の名前は\"山城'やましろ'昌幹'まさもと'\"です。".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_with_space_delimiter() {
    let custom_words = HashSet::new();

    // スペース区切りでの複数文字列
    let result = tokenize_with_custom_words("'Hello' 'World'", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("Hello".to_string()),
        Token::String("World".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_with_bracket_delimiter() {
    let custom_words = HashSet::new();

    // 括弧区切りでの文字列
    let result = tokenize_with_custom_words("['test']", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::VectorStart(crate::types::BracketType::Square),
        Token::String("test".to_string()),
        Token::VectorEnd(crate::types::BracketType::Square),
    ]);
}
