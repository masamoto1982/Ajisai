// rust/src/tokenizer.rs

use crate::types::{Token, BracketType};
use std::collections::HashSet;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    tokenize_with_custom_words(input, &HashSet::new())
}

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Input: {:?}", input).into());
    
    let mut tokens = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Lines: {:?}", lines).into());

    for (line_idx, line) in lines.iter().enumerate() {
        let preprocessed_line = line.split('#').next().unwrap_or("").trim();
        web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Line {}: {:?} -> {:?}", line_idx, line, preprocessed_line).into());
        
        if preprocessed_line.is_empty() {
            if line_idx < lines.len() - 1 { 
                tokens.push(Token::LineBreak); 
                web_sys::console::log_1(&"[TOKENIZER DEBUG] Added LineBreak".into());
            }
            continue;
        }

        let chars: Vec<char> = preprocessed_line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i].is_whitespace() { i += 1; continue; }

            let start_pos = i;
            
            if let Some((token, consumed)) = parse_single_char_tokens(chars[i]) {
                web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Single char token at {}: {:?}", i, token).into());
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = parse_single_quote_string(&chars[i..]) {
                web_sys::console::log_1(&format!("[TOKENIZER DEBUG] String token at {}: {:?}", i, token).into());
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_modifier(&chars[i..]) {
                web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Modifier token at {}: {:?}", i, token).into());
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
                web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Number token at {}: {:?}", i, token).into());
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
                web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Custom word token at {}: {:?}", i, token).into());
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
                web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Operator token at {}: {:?}", i, token).into());
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_ascii_builtin(&chars[i..]) {
                web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Builtin token at {}: {:?}", i, token).into());
                tokens.push(token); i += consumed; continue;
            }
            
            web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Unrecognized character at position {}: '{}'", i, chars[i]).into());
            i += 1;
        }
        
        if line_idx < lines.len() - 1 { 
            tokens.push(Token::LineBreak);
            web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Added LineBreak at end of line {}", line_idx).into());
        }
    }
    
    web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Before bracket conversion: {:?}", tokens).into());
    convert_vector_brackets_by_depth(&mut tokens)?;
    web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Final tokens: {:?}", tokens).into());
    
    Ok(tokens)
}

fn parse_single_char_tokens(c: char) -> Option<(Token, usize)> {
    match c {
        '[' => Some((Token::VectorStart(BracketType::Square), 1)),
        ']' => Some((Token::VectorEnd(BracketType::Square), 1)),
        ':' => Some((Token::DefBlockStart, 1)),
        ';' => Some((Token::DefBlockEnd, 1)),
        '$' => Some((Token::GuardSeparator, 1)),
        _ => None,
    }
}

fn try_parse_modifier(chars: &[char]) -> Option<(Token, usize)> {
    web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Trying to parse modifier from: {:?}", chars.iter().take(10).collect::<String>()).into());
    
    let mut i = 0;
    while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
    
    if i > 0 && i < chars.len() {
        let unit: String = chars[i..].iter().take_while(|c| c.is_alphabetic()).collect();
        web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Found digits: {}, unit: '{}'", i, unit).into());
        
        if unit == "x" || unit == "s" || unit == "ms" {
            let end_of_modifier = i + unit.len();
            if end_of_modifier == chars.len() || !chars[end_of_modifier].is_alphanumeric() {
                let modifier_str: String = chars[..end_of_modifier].iter().collect();
                web_sys::console::log_1(&format!("[TOKENIZER DEBUG] Successfully parsed modifier: '{}'", modifier_str).into());
                return Some((Token::Modifier(modifier_str), end_of_modifier));
            }
        }
    }
    None
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
                if depth == 0 { return Ok(i); }
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
                let bracket_type = match depth_stack.len() % 3 {
                    0 => BracketType::Square, 1 => BracketType::Curly, 2 => BracketType::Round, _ => unreachable!(),
                };
                *token = Token::VectorStart(bracket_type.clone());
                depth_stack.push(bracket_type);
            },
            Token::VectorEnd(_) => {
                if let Some(opening_type) = depth_stack.pop() { *token = Token::VectorEnd(opening_type); }
                else { return Err("Unexpected closing bracket".to_string()); }
            },
            _ => {}
        }
    }
    Ok(())
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

fn is_word_char(c: char) -> bool { c.is_alphanumeric() || c == '_' }

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
    
    // 小数点の処理
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
    } 
    // 分数の処理
    else if i < chars.len() && chars[i] == '/' {
        i += 1;
        if i == chars.len() || !chars[i].is_ascii_digit() { 
            i -= 1;
        } else {
            while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
        }
    }
    
    // 科学的記数法の処理を追加
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        i += 1;
        if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
            i += 1;
        }
        let exp_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
        if i == exp_start {
            // 指数部に数字がない場合は無効
            return None;
        }
    }
    
    let end = if start > 0 && i > start { i } else if start == 0 { i } else { 0 };

    if end > start && (i == chars.len() || !is_word_char(chars[i])) {
        let num_str: String = chars[..i].iter().collect();
        return Some((Token::Number(num_str), i));
    }
    None
}

fn try_parse_ascii_builtin(chars: &[char]) -> Option<(Token, usize)> {
    let builtin_words = ["TRUE", "FALSE", "NIL", "DUP", "SWAP", "ROT", "GET", "INSERT", "REPLACE", "REMOVE", "LENGTH", "TAKE", "DROP", "SPLIT", "CONCAT", "REVERSE", "AND", "OR", "NOT", "PRINT", "DEF", "DEL", "RESET", "GOTO"];
    for word in &builtin_words {
        if chars.len() >= word.len() && chars[..word.len()].iter().collect::<String>().to_uppercase() == *word {
            if chars.len() > word.len() && is_word_char(chars[word.len()]) { continue; }
            let token = match word.to_lowercase().as_str() {
                "true" => Token::Boolean(true),
                "false" => Token::Boolean(false),
                "nil" => Token::Nil,
                _ => Token::Symbol(word.to_string()),
            };
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
            '+' | '-' | '*' | '/' | '<' | '>' | '=' => {
                 if chars.len() > 1 && chars[1].is_ascii_digit() { return None; }
                 Some((Token::Symbol(chars[0].to_string()), 1))
            },
            _ => None
        }
    } else { None }
}
