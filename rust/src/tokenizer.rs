// rust/src/tokenizer.rs (BigInt対応・完全修正版)

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

            // 文字列リテラル
            if chars[i] == '\'' {
                if let Some((token, consumed)) = parse_single_quote_string(&chars[i..]) {
                    tokens.push(token);
                    i += consumed;
                    continue;
                }
            }
            
            // 記号
            match chars[i] {
                '[' => { 
                    tokens.push(Token::VectorStart(BracketType::Square)); 
                    i += 1; 
                    continue; 
                },
                ']' => { 
                    tokens.push(Token::VectorEnd(BracketType::Square)); 
                    i += 1; 
                    continue; 
                },
                ':' => { 
                    tokens.push(Token::QuotationStart); 
                    i += 1; 
                    continue; 
                },
                ';' => {
                    tokens.push(Token::QuotationEnd);
                    i += 1;
                    continue;
                },
                '@' => {
                    tokens.push(Token::At);
                    i += 1;
                    continue;
                }
                _ => {}
            }
            
            // コメント
            if chars[i] == '#' {
                break;
            }
            
            // 数値のパース（最優先）
            if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // カスタムワード
            if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // 演算子
            if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // ビルトインワード
            if let Some((token, consumed)) = try_parse_ascii_builtin(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // 認識できない文字はスキップ
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
    c.is_ascii_alphanumeric() || c.is_alphabetic() || c == '_'
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
    let start = i;
    
    // 符号の処理
    if chars[i] == '-' || chars[i] == '+' {
        i += 1;
        // 符号の後に数字が続かない場合は演算子として扱う
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None;
        }
    }
    
    // 整数部
    let int_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    
    // 少なくとも1つの数字が必要
    if i == int_start {
        return None;
    }
    
    // 小数点の処理
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        // 小数部
        let _frac_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        // 小数点の後に数字がない場合も有効（例: "3."）
    }
    
    // 分数の処理
    else if i < chars.len() && chars[i] == '/' {
        i += 1;
        // 分母は必須
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            // 分母がない場合は整数として扱う
            i -= 1;
        } else {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
    }
    
    // 次の文字が数字やアルファベットの場合は無効
    if i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.') {
        return None;
    }
    
    let number_str: String = chars[start..i].iter().collect();
    Some((Token::Number(number_str), i - start))
}

fn try_parse_ascii_builtin(chars: &[char]) -> Option<(Token, usize)> {
    let builtin_words = [
        "true", "false", "nil", "NIL", 
        "DUP", "SWAP", "ROT", 
        "GET", "INSERT", "REPLACE", "REMOVE", 
        "LENGTH", "TAKE", "DROP", "SPLIT",
        "CONCAT", "REVERSE", 
        "AND", "OR", "NOT", 
        "PRINT", "DEF", "DEL", "RESET",
        "CALL"
    ];
    
    for word in &builtin_words {
        if chars.len() >= word.len() {
            let candidate: String = chars[..word.len()].iter().collect();
            if candidate.to_uppercase() == *word {
                // 次の文字がアルファベットや数字でないことを確認
                if chars.len() > word.len() && is_word_char(chars[word.len()]) {
                    continue;
                }
                
                let token = match word.to_lowercase().as_str() {
                    "true" => Token::Boolean(true),
                    "false" => Token::Boolean(false),
                    "nil" => Token::Nil,
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
    
    // 2文字演算子
    if chars.len() >= 2 {
        let two_char: String = chars[..2].iter().collect();
        match two_char.as_str() {
            "<=" => return Some((Token::Symbol("<=".to_string()), 2)),
            ">=" => return Some((Token::Symbol(">=".to_string()), 2)),
            _ => {}
        }
    }
    
    // 1文字演算子（単独の+/-は数値の符号と区別する必要がある）
    match chars[0] {
        '+' | '-' => {
            // 次の文字が数字の場合は数値の符号として扱うため、ここでは処理しない
            if chars.len() > 1 && chars[1].is_ascii_digit() {
                None
            } else {
                Some((Token::Symbol(chars[0].to_string()), 1))
            }
        },
        '*' | '/' | '<' | '>' | '=' => {
            Some((Token::Symbol(chars[0].to_string()), 1))
        },
        _ => None
    }
}
