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
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue; // \n は次のイテレーションで処理
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
        // 7. 演算子（2文字演算子を含む）
        else if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
            tokens.push(token);
            i += consumed;
        }
        // 8. カスタムワード・組み込みワード（PMA検索）
        else {
            let remaining_slice: String = chars[i..].iter().collect();
            if let Some(mat) = pma.find_iter(&remaining_slice).next() {
                if mat.start() == 0 {
                    let word_len = mat.end();
                    if word_len < remaining_slice.len() && is_word_char(remaining_slice.chars().nth(word_len).unwrap()) {
                        // ワード境界でない場合はスキップ
                    } else {
                        let matched_word = &remaining_slice[..word_len];
                        tokens.push(Token::Symbol(matched_word.to_string()));
                        i += word_len;
                        continue;
                    }
                }
            }

            // 9. 通常のシンボル
            if chars[i].is_alphabetic() || chars[i] == '_' {
                let mut j = i;
                while j < chars.len() && is_word_char(chars[j]) {
                    j += 1;
                }
                let word: String = chars[i..j].iter().collect();
                tokens.push(Token::Symbol(word));
                i = j;
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
        _ => None,
    }
}

fn parse_quote_string(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() { return None; }
    
    let quote_char = chars[0];
    if quote_char != '\'' && quote_char != '"' { return None; }
    
    let mut string = String::new();
    let mut i = 1;
    
    // ★ 複数行文字列に対応
    while i < chars.len() {
        if chars[i] == quote_char {
            return Some((Token::String(string), i + 1));
        }
        // TODO: エスケープシーケンス（ \n, \t, \" など）の対応がここにはないが、
        // リテラルの改行は許可する
        string.push(chars[i]);
        i += 1;
    }
    
    // 閉じ引用符が見つからなかった
    None
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

fn is_word_char(c: char) -> bool { c.is_ascii_alphanumeric() || c == '_' || c == '?' || c == '!' }

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
        // 次がワード文字でなければ数値として確定
        if i == chars.len() || !is_word_char(chars[i]) {
            let number_str: String = chars[0..i].iter().collect();
            return Some((Token::Number(number_str), i));
        }
    }
    
    None
}


fn try_parse_operator(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() { return None; }

    let two_char_ops = ["<=", ">="];
    if chars.len() >= 2 {
        let two_char: String = chars[0..2].iter().collect();
        if two_char_ops.contains(&two_char.as_str()) {
            return Some((Token::Symbol(two_char), 2));
        }
    }

    let single_char_ops = ['+', '-', '*', '/', '=', '<', '>'];
    if single_char_ops.contains(&chars[0]) {
        return Some((Token::Symbol(chars[0].to_string()), 1));
    }

    None
}
