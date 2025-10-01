// rust/src/tokenizer.rs (修正版)

use crate::types::{Token, BracketType};
use std::collections::HashSet;
use crate::builtins;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    tokenize_with_custom_words(input, &HashSet::new())
}

// rust/src/tokenizer.rs

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    
    let builtin_words: HashSet<String> = builtins::get_builtin_definitions()
        .iter()
        .map(|(name, _, _)| name.to_string())
        .collect();

    for (line_num, line) in lines.iter().enumerate() {
        // 🆕 行の途中の#以降をコメントとして除去
        let line_without_comment = if let Some(pos) = line.find('#') {
            &line[..pos]
        } else {
            line
        };
        
        let trimmed_line = line_without_comment.trim();
        
        // 空行の処理
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
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }

            let mut token_found = false;

            if let Some((token, consumed)) = parse_single_char_tokens(chars[i]) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = parse_single_quote_string(&chars[i..]) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = try_parse_keyword(&chars[i..]) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = try_parse_modifier(&chars[i..]) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = try_parse_ascii_builtin(&chars[i..], &builtin_words) {
                tokens.push(token); i += consumed; token_found = true;
            }
            
            if !token_found {
                return Err(format!("Unknown token starting with: {}", chars[i..].iter().collect::<String>()));
            }
            line_has_tokens = true;
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
        ':' => Some((Token::GuardSeparator, 1)),
        ';' => Some((Token::DefBlockEnd, 1)),
        _ => None,
    }
}

// 修正点: 新しい関数を追加
fn try_parse_keyword(chars: &[char]) -> Option<(Token, usize)> {
    const KEYWORDS: [(&str, Token); 3] = [
        ("TRUE", Token::Boolean(true)),
        ("FALSE", Token::Boolean(false)),
        ("NIL", Token::Nil),
    ];

    for (keyword_str, token) in KEYWORDS.iter() {
        if chars.len() >= keyword_str.len() {
            let potential_match: String = chars[..keyword_str.len()].iter().collect();
            if potential_match.eq_ignore_ascii_case(keyword_str) {
                if chars.len() == keyword_str.len() || !is_word_char(chars[keyword_str.len()]) {
                    return Some((token.clone(), keyword_str.len()));
                }
            }
        }
    }
    None
}

fn try_parse_modifier(chars: &[char]) -> Option<(Token, usize)> {
    let mut i = 0;
    while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
    
    if i > 0 && i < chars.len() {
        let unit: String = chars[i..].iter().take_while(|c| c.is_alphabetic()).collect();
        
        if unit == "x" || unit == "s" || unit == "ms" {
            let end_of_modifier = i + unit.len();
            if end_of_modifier == chars.len() || !is_word_char(chars[end_of_modifier]) {
                let modifier_str: String = chars[..end_of_modifier].iter().collect();
                return Some((Token::Modifier(modifier_str), end_of_modifier));
            }
        }
    }
    None
}

fn try_parse_custom_word(chars: &[char], custom_words: &HashSet<String>) -> Option<(Token, usize)> {
    let mut sorted_words: Vec<&String> = custom_words.iter().collect();
    sorted_words.sort_by(|a, b| b.len().cmp(&a.len()));
    for word in sorted_words {
        if chars.starts_with(&word.chars().collect::<Vec<char>>()) {
            if chars.len() == word.len() || !is_word_char(chars[word.len()]) {
                return Some((Token::Symbol(word.clone()), word.len()));
            }
        }
    }
    None
}

fn is_word_char(c: char) -> bool { c.is_ascii_alphanumeric() || c == '_' || c == '?' || c == '!' }

fn parse_single_quote_string(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() || chars[0] != '\'' { return None; }
    let mut string = String::new(); let mut i = 1;
    while i < chars.len() {
        if chars[i] == '\'' { return Some((Token::String(string), i + 1)); }
        string.push(chars[i]); i += 1;
    }
    None
}

fn try_parse_number(chars: &[char]) -> Option<(Token, usize)> {
    let original_len = chars.len();
    let mut temp_chars = chars;

    if temp_chars.is_empty() { return None; }

    let mut i = 0;
    if i < temp_chars.len() && (temp_chars[i] == '-' || temp_chars[i] == '+') {
        if i + 1 < temp_chars.len() && temp_chars[i+1].is_ascii_digit() {
             i += 1;
        } else {
            return None;
        }
    }
    let start = i;
    while i < temp_chars.len() && temp_chars[i].is_ascii_digit() { i += 1; }
    
    if i < temp_chars.len() && temp_chars[i] == '.' {
        i += 1;
        while i < temp_chars.len() && temp_chars[i].is_ascii_digit() { i += 1; }
    } 
    else if i < temp_chars.len() && temp_chars[i] == '/' {
        i += 1;
        if i == temp_chars.len() || !temp_chars[i].is_ascii_digit() { 
            i -= 1;
        } else {
            while i < temp_chars.len() && temp_chars[i].is_ascii_digit() { i += 1; }
        }
    }
    
    if i < temp_chars.len() && (temp_chars[i] == 'e' || temp_chars[i] == 'E') {
        i += 1;
        if i < temp_chars.len() && (temp_chars[i] == '+' || temp_chars[i] == '-') {
            i += 1;
        }
        let exp_start = i;
        while i < temp_chars.len() && temp_chars[i].is_ascii_digit() { i += 1; }
        if i == exp_start {
            return None;
        }
    }
    
    let end = if start > 0 && i > start { i } else if start == 0 { i } else { 0 };

    if end > 0 && (i == temp_chars.len() || !is_word_char(temp_chars[i])) {
        let num_str: String = temp_chars[..i].iter().collect();
        if crate::types::Fraction::from_str(&num_str).is_ok() {
            return Some((Token::Number(num_str), i));
        }
    }
    None
}

// 修正点: `true`, `false`, `nil` の特別扱いを削除
fn try_parse_ascii_builtin(chars: &[char], builtin_words: &HashSet<String>) -> Option<(Token, usize)> {
    let mut sorted_words: Vec<&String> = builtin_words.iter().collect();
    sorted_words.sort_by(|a, b| b.len().cmp(&a.len()));

    for word in sorted_words {
        if chars.len() >= word.len() && chars[..word.len()].iter().collect::<String>().to_uppercase() == *word {
            if chars.len() > word.len() && is_word_char(chars[word.len()]) { continue; }
            let token = Token::Symbol(word.to_string());
            return Some((token, word.len()));
        }
    }
    None
}

fn try_parse_operator(chars: &[char]) -> Option<(Token, usize)> {
    if chars.len() >= 2 {
        let two_char: String = chars[..2].iter().collect();
        match two_char.as_str() {
            "<=" => return Some((Token::Symbol("<=".to_string()), 2)),
            ">=" => return Some((Token::Symbol(">=".to_string()), 2)),
            "!=" => return Some((Token::Symbol("!=".to_string()), 2)),
            _ => {}
        }
    }
    if !chars.is_empty() {
        match chars[0] {
            '+' | '-' | '*' | '/' | '<' | '>' | '=' | '?' => {
                 if chars.len() > 1 && (chars[1].is_ascii_digit() || chars[1] == '.') && chars[0] != '?' { return None; }
                 Some((Token::Symbol(chars[0].to_string()), 1))
            },
            _ => None
        }
    } else { None }
}
