// rust/src/tokenizer.rs

use crate::types::{Token, BracketType};
use std::collections::HashSet;
use crate::builtins;
use daachorse::DoubleArrayAhoCorasick;

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    
    let builtin_words: Vec<String> = builtins::get_builtin_definitions()
        .iter()
        .map(|(name, _, _)| name.to_string())
        .collect();

    let patterns: Vec<String> = builtin_words.iter()
        .cloned()
        .chain(custom_words.iter().cloned())
        .collect();
    
    let pma = DoubleArrayAhoCorasick::<u32>::new(&patterns).unwrap();

    for (line_num, line) in lines.iter().enumerate() {
        let line_without_comment = if let Some(pos) = line.find('#') {
            &line[..pos]
        } else {
            line
        };
        
        let trimmed_line = line_without_comment.trim();
        
        if trimmed_line.is_empty() {
            if line_num < lines.len() - 1 {
                tokens.push(Token::LineBreak);
            }
            continue;
        }

        let chars: Vec<char> = line_without_comment.chars().collect();
        let mut i = 0;
        let mut line_has_tokens = false;

        while i < chars.len() {
            let current_slice = &line_without_comment[i..];
            if current_slice.trim().is_empty() {
                break;
            }
            i = line_without_comment.len() - current_slice.len() + (current_slice.len() - current_slice.trim_start().len());

            if i >= chars.len() { break; }

            // 1. 単一文字トークン（括弧、:、;など）
            if let Some((token, consumed)) = parse_single_char_tokens(chars[i]) {
                tokens.push(token); i += consumed; line_has_tokens = true;
            } 
            // 2. 引用文字列
            else if let Some((token, consumed)) = parse_quote_string(&chars[i..]) {
                tokens.push(token); i += consumed; line_has_tokens = true;
            } 
            // 3. キーワード（TRUE, FALSE, NIL等）
            else if let Some((token, consumed)) = try_parse_keyword(&chars[i..]) {
                tokens.push(token); i += consumed; line_has_tokens = true;
            } 
            // 4. 数値
            else if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
                tokens.push(token); i += consumed; line_has_tokens = true;
            } 
            // 5. 演算子（2文字演算子を含む）
            else if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
                tokens.push(token); i += consumed; line_has_tokens = true;
            }
            // 6. カスタムワード・組み込みワード（PMA検索）
            else {
                let remaining = &line_without_comment[i..];
                if let Some(mat) = pma.find_iter(remaining).next() {
                    if mat.start() == 0 {
                        let word_len = mat.end();
                        if word_len < remaining.len() && is_word_char(remaining.chars().nth(word_len).unwrap()) {
                            // ワード境界でない場合はスキップ
                        } else {
                            let matched_word = &remaining[..word_len];
                            tokens.push(Token::Symbol(matched_word.to_string()));
                            i += word_len;
                            line_has_tokens = true;
                            continue;
                        }
                    }
                }

                // 7. 通常のシンボル
                if chars[i].is_alphabetic() || chars[i] == '_' {
                    let mut j = i;
                    while j < chars.len() && is_word_char(chars[j]) {
                        j += 1;
                    }
                    let word: String = chars[i..j].iter().collect();
                    tokens.push(Token::Symbol(word));
                    i = j;
                    line_has_tokens = true;
                } else {
                    return Err(format!("Unknown token starting with: {}", &line_without_comment[i..]));
                }
            }
        }
        
        if line_has_tokens && line_num < lines.len() - 1 {
            tokens.push(Token::LineBreak);
        }
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
    
    while i < chars.len() {
        if chars[i] == quote_char {
            return Some((Token::String(string), i + 1));
        }
        string.push(chars[i]);
        i += 1;
    }
    
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
            return None;
        }
    }
    let start = i;
    while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
    
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
    }
    
    if i > start || (i > 0 && start > 0) {
        let number_str: String = chars[0..i].iter().collect();
        return Some((Token::Number(number_str), i));
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
