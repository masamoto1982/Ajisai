// rust/src/tokenizer.rs (空白区切りベース - 伝統的なFORTHスタイル)

use crate::types::Token;
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
        match parse_quote_string(&chars[i..]) {
            QuoteParseResult::Success(token, consumed) => {
                tokens.push(token);
                i += consumed;
                continue;
            }
            QuoteParseResult::Unclosed => {
                let quote_char = chars[i];
                return Err(format!("Unclosed string literal starting with {}", quote_char));
            }
            QuoteParseResult::NotQuote => {
                // 引用文字列ではない、次の処理へ
            }
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
/// ダブルクォートは特殊文字として扱わない（文字列内で使用可能にするため）
fn is_special_char(c: char) -> bool {
    matches!(c, '[' | ']' | '{' | '}' | '(' | ')' | ':' | ';' | '#' | '\'')
}

fn parse_single_char_tokens(c: char) -> Option<(Token, usize)> {
    match c {
        // [], {}, () は全て同等にVectorとして扱う
        // 表示時に深さに応じて適切な括弧に変換される
        '[' | '{' | '(' => Some((Token::VectorStart, 1)),
        ']' | '}' | ')' => Some((Token::VectorEnd, 1)),
        ':' | ';' => Some((Token::GuardSeparator, 1)),
        _ => None,
    }
}

/// 引用文字列のパース結果
enum QuoteParseResult {
    /// 正常にパースできた (トークン, 消費文字数)
    Success(Token, usize),
    /// 閉じ引用符がない
    Unclosed,
    /// 引用文字列ではない
    NotQuote,
}

fn parse_quote_string(chars: &[char]) -> QuoteParseResult {
    if chars.is_empty() { return QuoteParseResult::NotQuote; }

    let quote_char = chars[0];
    // シングルクォートのみを文字列リテラルとして認識
    // ダブルクォートは文字列リテラルとして使用しない
    if quote_char != '\'' { return QuoteParseResult::NotQuote; }

    let mut string = String::new();
    let mut i = 1;

    // 開始と同じクォート文字の後に区切り文字がある場合に終了
    while i < chars.len() {
        if chars[i] == quote_char {
            // 次の文字が区切り文字（または EOF）かチェック
            if i + 1 >= chars.len() || is_delimiter(chars[i + 1]) {
                return QuoteParseResult::Success(Token::String(string), i + 1);
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
    QuoteParseResult::Unclosed
}

/// クォート文字の後の文字が区切り文字かどうかを判定
fn is_delimiter(c: char) -> bool {
    c.is_whitespace() || is_special_char(c)
}

/// キーワード解析（ドット演算子のみ）
/// TRUE/FALSE/NILは組み込みワードとして実装するため、ここでは解析しない
/// すべてのシンボルは統一的に Symbol として扱われる
fn try_parse_keyword_from_string(s: &str) -> Option<Token> {
    match s {
        "." => Some(Token::Symbol(".".to_string())),
        ".." => Some(Token::Symbol("..".to_string())),
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
        if !chars[i + 1].is_ascii_digit() {
            // "+a" や "-foo" のような場合は数値ではない
            return None;
        }
        i += 1;
    }

    // 最初の文字が数字でなければ数値ではない
    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return None;
    }

    let start = i;

    // 整数部分
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    // 分数形式のチェック (例: 1/3, -1/3)
    if i < chars.len() && chars[i] == '/' {
        let _slash_pos = i;
        i += 1;

        // スラッシュ後の符号（負の分母は許可しない設計だが、パースは許可）
        // 分母部分
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            // "1/" のような不完全な分数は数値ではない
            // シンボルとして扱う
            return None;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }

        // 全体を読み切ったかチェック
        if i == chars.len() {
            return Some(Token::Number(s.to_string()));
        } else {
            // 余分な文字があるので数値ではない (例: "1/3abc")
            return None;
        }
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
