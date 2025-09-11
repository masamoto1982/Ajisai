// rust/src/tokenizer.rs (BigInt対応版)

use crate::types::{Token, BracketType};
use std::collections::HashSet;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    tokenize_with_custom_words(input, &HashSet::new())
}

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = input.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }

            if chars[i] == '\'' {
                if let Some((token, consumed)) = parse_single_quote_string(&chars[i..]) {
                    tokens.push(token);
                    i += consumed;
                    continue;
                }
            }
            
            match chars[i] {
                '[' => { tokens.push(Token::VectorStart(BracketType::Square)); i += 1; continue; },
                ']' => { tokens.push(Token::VectorEnd(BracketType::Square)); i += 1; continue; },
                ':' => { tokens.push(Token::Colon); i += 1; continue; },
                _ => {}
            }
            
            if chars[i] == '#' {
                break;
            }
            
            if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            if let Some((token, consumed)) = try_parse_ascii_builtin(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            i += 1;
        }
        
        if line_idx < lines.len() - 1 {
            tokens.push(Token::LineBreak);
        }
    }

    convert_vector_brackets_by_depth(&mut tokens)?;
    
    Ok(tokens)
}

fn convert_vector_brackets_by_depth(tokens: &mut [Token]) -> Result<(), String> {
    let mut i = 0;
    while i < tokens.len() {
        if matches!(tokens[i], Token::VectorStart(_)) {
            match find_matching_vector_end(tokens, i) {
                Ok(vector_end) => {
                    convert_single_vector_brackets(&mut tokens[i..=vector_end])?;
                    i = vector_end + 1;
                },
                Err(e) => return Err(e),
            }
        } else {
            i += 1;
        }
    }
    Ok(())
}

fn find_matching_vector_end(tokens: &[Token], start: usize) -> Result<usize, String> {
    let mut depth = 0;
    for i in start..tokens.len() {
        match &tokens[i] {
            Token::VectorStart(_) => depth += 1,
            Token::VectorEnd(_) => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            },
            _ => {}
        }
    }
    Err("Unclosed vector found".to_string())
}

fn convert_single_vector_brackets(vector_tokens: &mut [Token]) -> Result<(), String> {
    let mut depth_stack = Vec::new();
    for token in vector_tokens.iter_mut() {
        match token {
            Token::VectorStart(_) => {
                let current_depth = depth_stack.len();
                if current_depth >= 6 {
                    return Err("Maximum nesting depth of 6 exceeded".to_string());
                }
                let bracket_type = match current_depth % 3 {
                    0 => BracketType::Square,
                    1 => BracketType::Curly,
                    2 => BracketType::Round,
                    _ => unreachable!(),
                };
                *token = Token::VectorStart(bracket_type.clone());
                depth_stack.push(bracket_type);
            },
            Token::VectorEnd(_) => {
                if let Some(opening_type) = depth_stack.pop() {
                    *token = Token::VectorEnd(opening_type);
                } else {
                    return Err("Unexpected closing bracket".to_string());
                }
            },
            _ => {}
        }
    }
    Ok(())
}


fn try_parse_custom_word(chars: &[char], custom_words: &HashSet<String>) -> Option<(Token, usize)> {
    let mut sorted_words: Vec<&String> = custom_words.iter().collect();
    sorted_words.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()));
    
    for word in sorted_words {
        let word_char_len = word.chars().count();
        if chars.len() >= word_char_len {
            let candidate: String = chars[..word_char_len].iter().collect();
            if candidate == *word && (chars.len() == word_char_len || !is_word_char(chars[word_char_len])) {
                return Some((Token::Symbol(word.clone()), word_char_len));
            }
        }
    }
    None
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c.is_alphabetic()
}

fn parse_single_quote_string(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() || chars[0] != '\'' { return None; }
    let mut string = String::new();
    let mut i = 1;
    let mut escaped = false;
    while i < chars.len() {
        if escaped {
            string.push(chars[i]);
            escaped = false;
        } else if chars[i] == '\\' {
            escaped = true;
        } else if chars[i] == '\'' {
            return Some((Token::String(string), i + 1));
        } else {
            string.push(chars[i]);
        }
        i += 1;
    }
    None
}

fn try_parse_number(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() { return None; }
    
    let mut i = 0;
    
    // 符号
    if chars[i] == '-' {
        i += 1;
    }
    
    // 数字が続くかチェック
    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return None; // 符号のみは数値ではない
    }
    
    // 整数部
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    
    // 小数点
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }
    
    // 分数
    if i < chars.len() && chars[i] == '/' {
        i += 1;
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None; // 分母がない
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }
    
    let number_str: String = chars[..i].iter().collect();
    Some((Token::Number(number_str), i))
}

fn try_parse_ascii_builtin(chars: &[char]) -> Option<(Token, usize)> {
    let builtin_words = [
        "true", "false", "nil", "NIL", "DUP", "SWAP", "ROT", "GET", "INSERT", 
        "REPLACE", "REMOVE", "LENGTH", "TAKE", "DROP", "REPEAT", "SPLIT",
        "CONCAT", "REVERSE", "+", "-", "*", "/", "=", "<", "<=", ">", ">=",
        "AND", "OR", "NOT", "PRINT", "DEF", "DEL", "RESET"
    ];
    
    for word in &builtin_words {
        if chars.len() >= word.len() {
            let candidate: String = chars[..word.len()].iter().collect();
            if candidate == *word && (chars.len() == word.len() || !chars[word.len()].is_ascii_alphanumeric()) {
                let token = match *word {
                    "true" => Token::Boolean(true),
                    "false" => Token::Boolean(false),
                    "nil" | "NIL" => Token::Nil,
                    _ => Token::Symbol(word.to_string()),
                };
                return Some((token, word.len()));
            }
        }
    }
    None
}

fn try_parse_operator(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() { return None; }
    
    if chars.len() >= 2 {
        let two_char: String = chars[..2].iter().collect();
        match two_char.as_str() {
            "<=" => return Some((Token::Symbol("<=".to_string()), 2)),
            ">=" => return Some((Token::Symbol(">=".to_string()), 2)),
            _ => {}
        }
    }
    
    match chars[0] {
        '+' | '-' | '*' | '/' | '<' | '>' | '=' => Some((Token::Symbol(chars[0].to_string()), 1)),
        _ => None
    }
}
