// rust/src/tokenizer.rs

use crate::types::{Token, BracketType};
use std::collections::HashSet;
use crate::builtins;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    tokenize_with_custom_words(input, &HashSet::new())
}

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    
    let builtin_words: HashSet<String> = builtins::get_builtin_definitions()
        .iter()
        .map(|(name, _, _)| name.to_string())
        .collect();

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
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }

            let mut token_found = false;

            if let Some((token, consumed)) = parse_single_char_tokens(chars[i]) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = parse_quote_string(&chars[i..]) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
                tokens.push(token); i += consumed; token_found = true;
            } else if let Some((token, consumed)) = try_parse_keyword(&chars[i..]) {
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
        ':' | ';' => Some((Token::GuardSeparator, 1)),
        _ => None,
    }
}

// ' または " で囲まれた文字列をパース
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
    const KEYWORDS: [(&str, Token); 5] = [
        ("TRUE", Token::Boolean(true)),
        ("FALSE", Token::Boolean(false)),
        ("NIL", Token::Nil),
        ("STACK", Token::Symbol("STACK".to_string())),
        ("STACKTOP", Token::Symbol("STACKTOP".to_string())),
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
    else if i < chars.len() && chars[i] == '/' {
        i += 1;
        if i == chars.len() || !chars[i].is_ascii_digit() { 
            i -= 1;
        } else {
            while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
        }
    }
    
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        i += 1;
        if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
            i += 1;
        }
        let exp_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
        if i == exp_start {
            return None;
        }
    }
    
    let end = if start > 0 && i > start { i } else if start == 0 { i } else { 0 };

    if end > 0 && (i == chars.len() || !is_word_char(chars[i])) {
        let num_str: String = chars[..i].iter().collect();
        if crate::types::Fraction::from_str(&num_str).is_ok() {
            return Some((Token::Number(num_str), i));
        }
    }
    None
}

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
