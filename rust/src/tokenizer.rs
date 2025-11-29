// rust/src/tokenizer.rs (空白区切りベース - 伝統的なFORTHスタイル)

use crate::types::{Token, BracketType};
use std::collections::HashSet;

#[allow(unused_variables)]

/// 伝統的なFORTHスタイルの空白区切りトークナイザー
/// マルチバイト文字（日本語など）のワード名をサポート
pub fn tokenize_with_custom_words(input: &str, _custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // 1. 空白と改行
        if chars[i].is_whitespace() {
            if chars[i] == '\n' {
                // 前のトークンが LineBreak でない場合のみ追加
                if tokens.last() != Some(&Token::LineBreak) {
                    tokens.push(Token::LineBreak);
                }
            }
            i += 1;
            continue;
        }

        // 2. コメント (行末までスキップ)
        if chars[i] == '#' {
            let had_token_before = !tokens.is_empty() && tokens.last() != Some(&Token::LineBreak);

            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            // 行頭コメントの場合のみ、改行もスキップ
            if !had_token_before && i < chars.len() && chars[i] == '\n' {
                i += 1;
            }
            continue;
        }

        // 3. 単一文字トークン（括弧、:、;など）
        if let Some((token, consumed)) = parse_single_char_tokens(chars[i]) {
            tokens.push(token);
            i += consumed;
            continue;
        }

        // 4. 引用文字列
        if let Some((token, consumed)) = parse_quote_string(&chars[i..]) {
            tokens.push(token);
            i += consumed;
            continue;
        }

        // 5. トークンの読み取り（空白または特殊文字まで）
        let start = i;
        while i < chars.len()
            && !chars[i].is_whitespace()
            && !is_special_char(chars[i]) {
            i += 1;
        }

        if i == start {
            // 処理できない文字
            return Err(format!("Unexpected character: {}", chars[i]));
        }

        let token_str: String = chars[start..i].iter().collect();

        // 6. キーワードチェック
        if let Some(token) = try_parse_keyword_from_string(&token_str) {
            tokens.push(token);
            continue;
        }

        // 7. 数値チェック
        if let Some(token) = try_parse_number_from_string(&token_str) {
            tokens.push(token);
            continue;
        }

        // 8. シンボル（すべての残り - マルチバイト文字を含む）
        tokens.push(Token::Symbol(token_str));
    }

    // 最後の不要なLineBreakを削除
    if tokens.last() == Some(&Token::LineBreak) {
        tokens.pop();
    }

    Ok(tokens)
}

/// 特殊文字（トークン境界となる文字）の判定
fn is_special_char(c: char) -> bool {
    matches!(c, '[' | ']' | '{' | '}' | '(' | ')' | ':' | ';' | '#' | '\'' | '"')
}

fn parse_single_char_tokens(c: char) -> Option<(Token, usize)> {
    match c {
        '[' => Some((Token::VectorStart(BracketType::Square), 1)),
        ']' => Some((Token::VectorEnd(BracketType::Square), 1)),
        '{' => Some((Token::VectorStart(BracketType::Curly), 1)),
        '}' => Some((Token::VectorEnd(BracketType::Curly), 1)),
        '(' => Some((Token::VectorStart(BracketType::Round), 1)),
        ')' => Some((Token::VectorEnd(BracketType::Round), 1)),
        ':' | ';' => Some((Token::GuardSeparator, 1)),
        _ => None,
    }
}

fn parse_quote_string(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() { return None; }

    let quote_char = chars[0];
    if quote_char != '\'' && quote_char != '"' { return None; }

    let mut string = String::new();
    let mut i = 1;

    // 開始と同じクォート文字の後に区切り文字がある場合に終了
    while i < chars.len() {
        if chars[i] == quote_char {
            // 次の文字が区切り文字（または EOF）かチェック
            if i + 1 >= chars.len() || is_delimiter(chars[i + 1]) {
                return Some((Token::String(string), i + 1));
            } else {
                // 区切り文字ではないので、クォート文字を文字列に含める
                string.push(chars[i]);
                i += 1;
            }
        } else {
            string.push(chars[i]);
            i += 1;
        }
    }

    // 閉じ引用符が見つからなかった
    None
}

/// クォート文字の後の文字が区切り文字かどうかを判定
fn is_delimiter(c: char) -> bool {
    c.is_whitespace() || is_special_char(c)
}

/// 文字列からキーワードを解析
fn try_parse_keyword_from_string(s: &str) -> Option<Token> {
    match s.to_uppercase().as_str() {
        "TRUE" => Some(Token::Boolean(true)),
        "FALSE" => Some(Token::Boolean(false)),
        "NIL" => Some(Token::Nil),
        "STACK" => Some(Token::Symbol("STACK".to_string())),
        "STACKTOP" => Some(Token::Symbol("STACKTOP".to_string())),
        _ => None,
    }
}

/// 文字列から数値を解析
fn try_parse_number_from_string(s: &str) -> Option<Token> {
    // 空文字列チェック
    if s.is_empty() {
        return None;
    }

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    // 符号のチェック
    if chars[i] == '-' || chars[i] == '+' {
        if chars.len() == 1 {
            // "+" や "-" だけの場合は演算子
            return None;
        }
        if !chars[i + 1].is_ascii_digit() && chars[i + 1] != '.' {
            // "+a" や "-foo" のような場合は数値ではない
            return None;
        }
        i += 1;
    }

    let start = i;

    // 整数部分
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    // 小数部分
    let mut has_dot = false;
    if i < chars.len() && chars[i] == '.' {
        has_dot = true;
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }

    // 指数部分
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        i += 1;
        if i < chars.len() && (chars[i] == '-' || chars[i] == '+') {
            i += 1;
        }
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            // "1e" や "1e+" のような不完全な指数は数値ではない
            return None;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }

    // 数値として有効かチェック
    if i == start && !has_dot {
        // 符号だけ、または何も読めなかった
        return None;
    }

    // 全体を読み切ったかチェック
    if i == chars.len() {
        Some(Token::Number(s.to_string()))
    } else {
        // 余分な文字があるので数値ではない
        None
    }
}
