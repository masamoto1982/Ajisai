// rust/src/tokenizer.rs (改行処理修正版)

use crate::types::{Token, BracketType};
use std::collections::HashSet;
use crate::builtins;
use daachorse::DoubleArrayAhoCorasick;

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    
    let builtin_words: Vec<String> = builtins::get_builtin_definitions()
        .iter()
        .map(|(name, _, _)| name.to_string())
        .collect();

    let patterns: Vec<String> = builtin_words.iter()
        .cloned()
        .chain(custom_words.iter().cloned())
        .collect();
    
    let pma = DoubleArrayAhoCorasick::<u32>::new(&patterns).unwrap();
    
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // 1. コメント (行末までスキップ)
        if chars[i] == '#' {
            // コメントの前にトークンがあったかチェック
            // （行末コメントか行頭コメントかを判定）
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

        // 2. 空白と改行
        if chars[i].is_whitespace() {
            if chars[i] == '\n' {
                // 前のトークンが LineBreak でない場合のみ追加 (連続する改行をまとめる)
                if tokens.last() != Some(&Token::LineBreak) {
                    tokens.push(Token::LineBreak);
                }
            }
            i += 1;
            continue;
        }

        // 3. 単一文字トークン（括弧、:、;など）
        if let Some((token, consumed)) = parse_single_char_tokens(chars[i]) {
            tokens.push(token);
            i += consumed;
        } 
        // 4. 引用文字列 (複数行対応)
        else if let Some((token, consumed)) = parse_quote_string(&chars[i..]) {
            tokens.push(token);
            i += consumed;
        } 
        // 5. キーワード（TRUE, FALSE, NIL等）
        else if let Some((token, consumed)) = try_parse_keyword(&chars[i..]) {
            tokens.push(token);
            i += consumed;
        } 
        // 6. 数値
        else if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
            tokens.push(token);
            i += consumed;
        }
        // 7. カスタムワード・組み込みワード（PMA検索）
        // 8. 辞書にないシンボルはスキップ
        else {
            let remaining_slice: String = chars[i..].iter().collect();

            // 位置0から始まる最長マッチを検索
            // 注意: mat.end() はバイト位置を返す
            let mut best_match: Option<(usize, usize)> = None;  // (バイト長, 文字数)

            for mat in pma.find_iter(&remaining_slice) {
                if mat.start() == 0 {
                    let byte_len = mat.end();
                    // バイト位置から文字数を計算
                    let matched_str = &remaining_slice[..byte_len];
                    let char_count = matched_str.chars().count();

                    // ワード境界チェック
                    let remaining_chars: Vec<char> = remaining_slice.chars().collect();
                    let last_char_of_match = matched_str.chars().last().unwrap();
                    let is_at_boundary = if is_word_char(last_char_of_match) {
                        // マッチの最後がワード文字の場合、次もワード文字ならワード境界ではない
                        char_count >= remaining_chars.len()
                            || !is_word_char(remaining_chars[char_count])
                    } else {
                        // マッチの最後がワード文字でない場合（演算子など）、常にワード境界
                        true
                    };

                    if is_at_boundary {
                        // 最長マッチを更新（バイト長で比較）
                        if best_match.as_ref().map_or(true, |(len, _)| byte_len > *len) {
                            best_match = Some((byte_len, char_count));
                        }
                    }
                }
            }

            if let Some((byte_len, char_count)) = best_match {
                let matched_word = &remaining_slice[..byte_len];
                tokens.push(Token::Symbol(matched_word.to_string()));
                i += char_count;  // 文字数で進める
                continue;
            }

            // 辞書にないワード文字列はスキップ（自然言語的な表現を可能にするため）
            // 1文字ずつスキップして、次のイテレーションで辞書のワードがマッチする機会を与える
            if chars[i].is_alphabetic() {
                i += 1;
            } else {
                return Err(format!("Unknown token starting with: {}", chars[i]));
            }
        }
    }
    
    // 最後の不要なLineBreakを削除
    if tokens.last() == Some(&Token::LineBreak) {
        tokens.pop();
    }
    
    Ok(tokens)
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
        '!' => Some((Token::Symbol("!".to_string()), 1)),
        _ => None,
    }
}

fn parse_quote_string(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() { return None; }

    let quote_char = chars[0];
    if quote_char != '\'' && quote_char != '"' { return None; }

    let mut string = String::new();
    let mut i = 1;

    // ★ 新しい仕様：開始と同じクォート文字の後に区切り文字がある場合に終了
    while i < chars.len() {
        if chars[i] == quote_char {
            // 次の文字が区切り文字（または EOF）かチェック
            if i + 1 >= chars.len() || is_delimiter(chars[i + 1]) {
                // 文字列の終端
                return Some((Token::String(string), i + 1));
            } else {
                // 区切り文字ではないので、クォート文字を文字列に含める
                string.push(chars[i]);
                i += 1;
            }
        } else {
            // 通常の文字（リテラルの改行も許可）
            string.push(chars[i]);
            i += 1;
        }
    }

    // 閉じ引用符が見つからなかった
    None
}

// クォート文字の後の文字が区切り文字かどうかを判定
fn is_delimiter(c: char) -> bool {
    c.is_whitespace()         // スペース、タブ、改行など
        || c == '['           // 角括弧開始
        || c == ']'           // 角括弧終了
        || c == '{'           // 波括弧開始
        || c == '}'           // 波括弧終了
        || c == '('           // 丸括弧開始
        || c == ')'           // 丸括弧終了
        || c == ':'           // ガード区切り
        || c == ';'           // ガード区切り
        || c == '#'           // コメント開始
}

fn try_parse_keyword(chars: &[char]) -> Option<(Token, usize)> {
    let keywords: [(&str, fn() -> Token); 5] = [
        ("TRUE", || Token::Boolean(true)),
        ("FALSE", || Token::Boolean(false)),
        ("NIL", || Token::Nil),
        ("STACK", || Token::Symbol("STACK".to_string())),
        ("STACKTOP", || Token::Symbol("STACKTOP".to_string())),
    ];

    for (keyword_str, token_fn) in keywords.iter() {
        if chars.len() >= keyword_str.len() {
            let potential_match: String = chars[..keyword_str.len()].iter().collect();
            if potential_match.eq_ignore_ascii_case(keyword_str) {
                if chars.len() == keyword_str.len() || !is_word_char(chars[keyword_str.len()]) {
                    return Some((token_fn(), keyword_str.len()));
                }
            }
        }
    }
    None
}

fn is_word_char(c: char) -> bool { c.is_alphanumeric() || c == '_' || c == '?' || c == '!' }

fn try_parse_number(chars: &[char]) -> Option<(Token, usize)> {
    let mut i = 0;
    if i < chars.len() && (chars[i] == '-' || chars[i] == '+') {
        if i + 1 < chars.len() && chars[i+1].is_ascii_digit() {
             i += 1;
        } else {
            // + や - だけの場合は演算子として処理させる
            return None;
        }
    }
    let start = i;
    while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
    
    let mut has_dot = false;
    if i < chars.len() && chars[i] == '.' {
        // "1." のようにドットで終わる場合、"1.e3" のように続く場合
        if i + 1 == chars.len() || chars[i+1].is_whitespace() || chars[i+1] == 'e' || chars[i+1] == 'E' {
             has_dot = true;
             i += 1;
        } else if chars[i+1].is_ascii_digit() {
            has_dot = true;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
        } else {
            // "1.foo" のような場合は "1" と ".foo"
            if i > start {
                 let number_str: String = chars[0..i].iter().collect();
                 return Some((Token::Number(number_str), i));
            } else {
                return None;
            }
        }
    }

    // "1.e3" や "1e3" のような科学的記数法
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        if i + 1 < chars.len() {
            let mut j = i + 1;
            if j < chars.len() && (chars[j] == '-' || chars[j] == '+') {
                j += 1;
            }
            if j < chars.len() && chars[j].is_ascii_digit() {
                i = j + 1;
                while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
            } else {
                 // "1.e" や "1e+" のように指数が続かない場合は、"e" の手前までを数値とする
            }
        }
    }
    
    // (start > 0) は "1" のような符号なし数値
    // (i > start) は "+1" のような符号あり数値
    // has_dot は "1." のような小数をカバー
    if i > start || (i > 0 && start > 0) || has_dot {
        // 次がASCII英数字やアンダースコアでなければ数値として確定
        // 日本語文字が続く場合は数値として認識する
        if i == chars.len() || !(chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
            let number_str: String = chars[0..i].iter().collect();
            return Some((Token::Number(number_str), i));
        }
    }
    
    None
}


