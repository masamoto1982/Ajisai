// rust/src/tokenizer.rs (全括弧統一入力対応版)

use crate::types::{Token, BracketType};
use std::collections::HashSet;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    tokenize_with_custom_words(input, &HashSet::new())
}

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // 空白文字をスキップ
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        
        // 文字列リテラル（シングルクォート）
        if chars[i] == '\'' {
            if let Some((token, consumed)) = parse_single_quote_string(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
        }
        
        // 機能説明コメント（ダブルクォート）
        if chars[i] == '"' {
            if let Some((token, consumed)) = parse_double_quote_comment(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
        }
        
        // Vector記号（統一入力：[ ] のみ受け付ける）
        match chars[i] {
            '[' => {
                // 開始括弧として一旦Squareで記録（後で深度に応じて変換）
                tokens.push(Token::VectorStart(BracketType::Square));
                i += 1;
                continue;
            },
            ']' => {
                // 終了括弧として一旦Squareで記録（後で深度に応じて変換）
                tokens.push(Token::VectorEnd(BracketType::Square));
                i += 1;
                continue;
            },
            // 他の括弧は通常の文字として扱う（エラーにはしない）
            _ => {}
        }
        
        // 行コメント（#から行末まで）
        if chars[i] == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        
        // 数値チェック（整数、分数、小数）
        if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
            tokens.push(token);
            i += consumed;
            continue;
        }
        
        // カスタムワードチェック（最優先）
        if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
            tokens.push(token);
            i += consumed;
            continue;
        }
        
        // 組み込みワードチェック（英数字）
        if let Some((token, consumed)) = try_parse_ascii_builtin(&chars[i..]) {
            tokens.push(token);
            i += consumed;
            continue;
        }
        
        // 演算子記号チェック
        if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
            tokens.push(token);
            i += consumed;
            continue;
        }
        
        // どれにもマッチしなければ無視して次へ
        i += 1;
    }

    // 括弧の深度に応じた変換を実行
    convert_brackets_by_depth(&mut tokens)?;
    
    Ok(tokens)
}

// 深度に応じて括弧タイプを自動変換
fn convert_brackets_by_depth(tokens: &mut [Token]) -> Result<(), String> {
    let mut depth_stack = Vec::new();
    
    for token in tokens.iter_mut() {
        match token {
            Token::VectorStart(_) => {
                let current_depth = depth_stack.len();
                
                // 6重ネスト制限
                if current_depth >= 6 {
                    return Err("Maximum nesting depth of 6 exceeded".to_string());
                }
                
                let bracket_type = match current_depth % 3 {
                    0 => BracketType::Square,  // 1層目、4層目
                    1 => BracketType::Curly,   // 2層目、5層目  
                    2 => BracketType::Round,   // 3層目、6層目
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
    
    // 未閉鎖の括弧チェック
    if !depth_stack.is_empty() {
        return Err(format!(
            "Unclosed bracket(s): {} bracket(s) remain open",
            depth_stack.len()
        ));
    }
    
    Ok(())
}

fn try_parse_custom_word(chars: &[char], custom_words: &HashSet<String>) -> Option<(Token, usize)> {
    // 長い単語から優先的にマッチング
    let mut sorted_words: Vec<&String> = custom_words.iter().collect();
    sorted_words.sort_by(|a, b| {
        let a_len = a.chars().count();
        let b_len = b.chars().count();
        b_len.cmp(&a_len)
    });
    
    for word in sorted_words {
        let word_char_len = word.chars().count();
        
        if chars.len() >= word_char_len {
            let candidate: String = chars[..word_char_len].iter().collect();
            
            if candidate == *word {
                // 単語境界チェック
                if chars.len() == word_char_len || 
                   !is_word_char(chars[word_char_len]) {
                    return Some((Token::Symbol(word.clone()), word_char_len));
                }
            }
        }
    }
    
    None
}

// 単語文字かどうかを判定
fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c.is_alphabetic()
}

// シングルクォート文字列解析
fn parse_single_quote_string(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() || chars[0] != '\'' {
        return None;
    }
    
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

// ダブルクォート機能説明コメント解析
fn parse_double_quote_comment(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() || chars[0] != '"' {
        return None;
    }
    
    let mut comment = String::new();
    let mut i = 1;
    let mut escaped = false;
    
    while i < chars.len() {
        if escaped {
            comment.push(chars[i]);
            escaped = false;
        } else if chars[i] == '\\' {
            escaped = true;
        } else if chars[i] == '"' {
            return Some((Token::FunctionComment(comment.trim().to_string()), i + 1));
        } else {
            comment.push(chars[i]);
        }
        i += 1;
    }
    
    None
}

// 数値解析（整数、分数、小数）
fn try_parse_number(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() {
        return None;
    }
    
    let first_char = chars[0];
    if !first_char.is_ascii_digit() && first_char != '.' && first_char != '-' {
        return None;
    }
    
    let mut i = 0;
    let mut number_str = String::new();
    
    // 負号の処理
    if chars[i] == '-' {
        if i + 1 >= chars.len() || (!chars[i + 1].is_ascii_digit() && chars[i + 1] != '.') {
            return None;
        }
        number_str.push(chars[i]);
        i += 1;
    }
    
    // 整数部分
    while i < chars.len() && chars[i].is_ascii_digit() {
        number_str.push(chars[i]);
        i += 1;
    }
    
    if number_str.is_empty() || number_str == "-" {
        return None;
    }
    
    // 分数チェック
    if i < chars.len() && chars[i] == '/' {
        number_str.push(chars[i]);
        i += 1;
        
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None;
        }
        
        while i < chars.len() && chars[i].is_ascii_digit() {
            number_str.push(chars[i]);
            i += 1;
        }
        
        let parts: Vec<&str> = number_str.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(num), Ok(den)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                if den != 0 {
                    return Some((Token::Number(num, den), i));
                }
            }
        }
        return None;
    }
    
    // 小数チェック
    if i < chars.len() && chars[i] == '.' {
        number_str.push(chars[i]);
        i += 1;
        
        while i < chars.len() && chars[i].is_ascii_digit() {
            number_str.push(chars[i]);
            i += 1;
        }
        
        if let Some((num, den)) = parse_decimal(&number_str) {
            return Some((Token::Number(num, den), i));
        }
        return None;
    }
    
    // 整数
    if let Ok(num) = number_str.parse::<i64>() {
        Some((Token::Number(num, 1), i))
    } else {
        None
    }
}

// 小数を分数に変換
fn parse_decimal(decimal_str: &str) -> Option<(i64, i64)> {
    if let Some(dot_pos) = decimal_str.find('.') {
        let integer_part = &decimal_str[..dot_pos];
        let decimal_part = &decimal_str[dot_pos + 1..];
        
        let is_negative = integer_part.starts_with('-');
        
        let integer_val = if integer_part.is_empty() || integer_part == "-" {
            0
        } else {
            integer_part.parse::<i64>().ok()?
        };
        
        let decimal_val = if decimal_part.is_empty() {
            0
        } else {
            decimal_part.parse::<i64>().ok()?
        };
        
        let decimal_places = decimal_part.len() as u32;
        let denominator = 10_i64.pow(decimal_places);
        
        let numerator = if is_negative {
            integer_val * denominator - decimal_val
        } else {
            integer_val * denominator + decimal_val
        };
        
        Some((numerator, denominator))
    } else {
        None
    }
}

fn try_parse_ascii_builtin(chars: &[char]) -> Option<(Token, usize)> {
    let builtin_words = [
        "true", "false", "nil", "NIL",
        // 英語組み込みワード
        "NTH", "INSERT", "REPLACE", "REMOVE",
        "LENGTH", "TAKE", "DROP", "REPEAT", "SPLIT",
        "CONCAT", "JUMP", "DEF", "DEL", "EVAL",
    ];
    
    for word in &builtin_words {
        if chars.len() >= word.len() {
            let candidate: String = chars[..word.len()].iter().collect();
            if candidate == *word {
                if chars.len() == word.len() || !chars[word.len()].is_ascii_alphanumeric() {
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
    }
    
    None
}

// 演算子記号解析
fn try_parse_operator(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() {
        return None;
    }
    
    // 2文字演算子を先にチェック
    if chars.len() >= 2 {
        let two_char: String = chars[..2].iter().collect();
        match two_char.as_str() {
            ">=" => return Some((Token::Symbol(">=".to_string()), 2)),
            "<=" => return Some((Token::Symbol("<=".to_string()), 2)),
            _ => {}
        }
    }
    
    // 1文字演算子
    match chars[0] {
        '+' => Some((Token::Symbol("+".to_string()), 1)),
        '-' => Some((Token::Symbol("-".to_string()), 1)),
        '*' => Some((Token::Symbol("*".to_string()), 1)),
        '/' => Some((Token::Symbol("/".to_string()), 1)),
        '>' => Some((Token::Symbol(">".to_string()), 1)),
        '<' => Some((Token::Symbol("<".to_string()), 1)),
        '=' => Some((Token::Symbol("=".to_string()), 1)),
        _ => None,
    }
}
