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
        // #コメント処理：#以降を除去
        let line_content = line.split('#').next().unwrap_or("").trim();
        
        if line_content.is_empty() {
            if line_num < lines.len() - 1 { // 最終行でなければ改行トークン追加
                tokens.push(Token::LineBreak);
            }
            continue;
        }

        let chars: Vec<char> = line_content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i].is_whitespace() { i += 1; continue; }
            
            if let Some((token, consumed)) = parse_single_char_tokens(chars[i]) {
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = parse_single_quote_string(&chars[i..]) {
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_modifier(&chars[i..]) {
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
                tokens.push(token); i += consumed; continue;
            }
            if let Some((token, consumed)) = try_parse_ascii_builtin(&chars[i..], &builtin_words) {
                tokens.push(token); i += consumed; continue;
            }
            
            i += 1;
        }
        
        // 行末に改行トークン追加（最終行以外）
        if line_num < lines.len() - 1 {
            tokens.push(Token::LineBreak);
        }
    }
    
    convert_vector_brackets_by_depth(&mut tokens)?;
    
    Ok(tokens)
}

fn parse_single_char_tokens(c: char) -> Option<(Token, usize)> {
    match c {
        '[' => Some((Token::VectorStart(BracketType::Square), 1)),
        ']' => Some((Token::VectorEnd(BracketType::Square), 1)),
        ':' => Some((Token::GuardSeparator, 1)), // : は条件分岐記号として使用
        ';' => Some((Token::DefBlockEnd, 1)), // ; は互換性のため残すが使用頻度は減る
        _ => None,
    }
}

fn try_parse_modifier(chars: &[char]) -> Option<(Token, usize)> {
    let mut i = 0;
    while i < chars.len() && chars[i].is_ascii_digit() { i += 1; }
    
    if i > 0 && i < chars.len() {
        let unit: String = chars[i..].iter().take_while(|c| c.is_alphabetic()).collect();
        
        if unit == "x" || unit == "s" || unit == "ms" {
            let end_of_modifier = i + unit.len();
            if end_of_modifier == chars.len() || !chars[end_of_modifier].is_alphanumeric() {
                let modifier_str: String = chars[..end_of_modifier].iter().collect();
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

    if end > start && (i == chars.len() || !is_word_char(chars[i])) {
        let num_str: String = chars[..i].iter().collect();
        return Some((Token::Number(num_str), i));
    }
    None
}

fn try_parse_ascii_builtin(chars: &[char], builtin_words: &HashSet<String>) -> Option<(Token, usize)> {
    let mut sorted_words: Vec<&String> = builtin_words.iter().collect();
    sorted_words.sort_by(|a, b| b.len().cmp(&a.len()));

    for word in sorted_words {
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
            '+' | '-' | '*' | '/' | '<' | '>' | '=' | '?' => {
                 if chars.len() > 1 && chars[1].is_ascii_digit() && chars[0] != '?' { return None; }
                 Some((Token::Symbol(chars[0].to_string()), 1))
            },
            _ => None
        }
    } else { None }
}
