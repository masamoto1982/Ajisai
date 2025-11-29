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
    let result = tokenize_with_custom_words("'彼は\"天才\"と呼ばれている。'", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("彼は\"天才\"と呼ばれている。".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_double_with_single_inside() {
    let custom_words = HashSet::new();

    // ダブルクォートで囲み、内部にシングルクォート
    let result = tokenize_with_custom_words("\"これは'重要'な情報です。\"", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("これは'重要'な情報です。".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_single_with_single_inside() {
    let custom_words = HashSet::new();

    // シングルクォートで囲み、内部にもシングルクォート
    let result = tokenize_with_custom_words("'今日は'晴れ'です。'", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("今日は'晴れ'です。".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_double_with_double_inside() {
    let custom_words = HashSet::new();

    // ダブルクォートで囲み、内部にもダブルクォート
    let result = tokenize_with_custom_words("\"彼女は\"素晴らしい\"演技をした。\"", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("彼女は\"素晴らしい\"演技をした。".to_string()),
    ]);
}

#[test]
fn test_flexible_quotes_complex_nested() {
    let custom_words = HashSet::new();

    // 複雑な入れ子構造
    let result = tokenize_with_custom_words("'彼の言葉は\"人生'じんせい'哲学'てつがく'\"である。'", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("彼の言葉は\"人生'じんせい'哲学'てつがく'\"である。".to_string()),
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

// 日本語ワードサポートのテスト
#[test]
fn test_japanese_word_recognition() {
    let mut custom_words = HashSet::new();
    custom_words.insert("足す".to_string());

    // 日本語ワードの認識
    let result = tokenize_with_custom_words("2 3 足す", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::Number("2".to_string()),
        Token::Number("3".to_string()),
        Token::Symbol("足す".to_string()),
    ]);
}

#[test]
fn test_natural_language_style_japanese() {
    let mut custom_words = HashSet::new();
    custom_words.insert("足す".to_string());

    // 自然言語風の入力（「と」「を」はスキップされる）
    let result = tokenize_with_custom_words("2と3を足す", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::Number("2".to_string()),
        Token::Number("3".to_string()),
        Token::Symbol("足す".to_string()),
    ]);
}

#[test]
fn test_japanese_word_boundary() {
    let mut custom_words = HashSet::new();
    custom_words.insert("足す".to_string());
    custom_words.insert("掛ける".to_string());

    // 日本語ワードだけの入力
    let result = tokenize_with_custom_words("足す", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::Symbol("足す".to_string()),
    ]);

    // 複数の日本語ワード
    let result2 = tokenize_with_custom_words("2 足す 3 掛ける 4", &custom_words).unwrap();
    assert_eq!(result2, vec![
        Token::Number("2".to_string()),
        Token::Symbol("足す".to_string()),
        Token::Number("3".to_string()),
        Token::Symbol("掛ける".to_string()),
        Token::Number("4".to_string()),
    ]);
}

#[test]
fn test_mixed_japanese_english() {
    let mut custom_words = HashSet::new();
    custom_words.insert("出力する".to_string());

    let result = tokenize_with_custom_words("'Hello' 出力する", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::String("Hello".to_string()),
        Token::Symbol("出力する".to_string()),
    ]);
}

#[test]
fn test_skip_unregistered_japanese() {
    let mut custom_words = HashSet::new();
    custom_words.insert("合計".to_string());

    // 「と」「を」は辞書にないのでスキップ
    let result = tokenize_with_custom_words("2と3を合計", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::Number("2".to_string()),
        Token::Number("3".to_string()),
        Token::Symbol("合計".to_string()),
    ]);
}

#[test]
fn test_operator_with_japanese() {
    let custom_words = HashSet::new();

    // 演算子の後に日本語が続く場合のテスト
    let result = tokenize_with_custom_words("1と2を+しなさい", &custom_words).unwrap();
    assert_eq!(result, vec![
        Token::Number("1".to_string()),
        Token::Number("2".to_string()),
        Token::Symbol("+".to_string()),
    ]);
}
