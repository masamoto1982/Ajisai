// rust/src/tokenizer.rs (複数行定義自動判定対応版)

use crate::types::{Token, BracketType};
use std::collections::HashSet;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    tokenize_with_custom_words(input, &HashSet::new())
}

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER INPUT: '{}'", input)));
    
    let mut tokens = Vec::new();
    let lines: Vec<&str> = input.lines().collect();

    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER LINES: {:?}", lines)));
    
    for (line_idx, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER processing line {}: '{}'", line_idx, line)));

        while i < chars.len() {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER at position {}: '{}'", i, chars[i])));
            
            // 空白文字をスキップ
            if chars[i].is_whitespace() {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("TOKENIZER: skipping whitespace"));
                i += 1;
                continue;
            }
            
            // 文字列リテラル（シングルクォート）- 最優先で処理
            if chars[i] == '\'' {
                if let Some((token, consumed)) = parse_single_quote_string(&chars[i..]) {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER: parsed string token: {:?}", token)));
                    tokens.push(token);
                    i += consumed;
                    continue;
                }
            }
            
            // 機能説明コメント（ダブルクォート）- 次に処理
            if chars[i] == '"' {
                if let Some((token, consumed)) = parse_double_quote_comment(&chars[i..]) {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER: parsed comment token: {:?}", token)));
                    tokens.push(token);
                    i += consumed;
                    continue;
                }
            }
            
            // Vector記号
            match chars[i] {
                '[' => {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("TOKENIZER: found ["));
                    tokens.push(Token::VectorStart(BracketType::Square));
                    i += 1;
                    continue;
                },
                ']' => {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("TOKENIZER: found ]"));
                    tokens.push(Token::VectorEnd(BracketType::Square));
                    i += 1;
                    continue;
                },
                ':' => {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("TOKENIZER: found :"));
                    tokens.push(Token::Colon);
                    i += 1;
                    continue;
                },
                _ => {}
            }
            
            // 行コメント（#から行末まで）
            if chars[i] == '#' {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("TOKENIZER: found comment, breaking"));
                break; // 行の残りをスキップ
            }
            
            // 数値チェック
            if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER: parsed number token: {:?}", token)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // カスタムワードチェック
            if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER: parsed custom word token: {:?}", token)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // 演算子記号チェック（組み込みワードより先に）
            if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER: parsed operator token: {:?}", token)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // 組み込みワードチェック（最後に）
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER: trying builtin parse from: '{}'", chars[i..].iter().take(10).collect::<String>())));
            if let Some((token, consumed)) = try_parse_ascii_builtin(&chars[i..]) {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER: parsed builtin token: {:?}", token)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // どれにもマッチしなければ無視して次へ
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER: no match, skipping character: '{}'", chars[i])));
            i += 1;
        }
        
        // 各行の終わりに改行トークンを追加（最後の行以外）
        if line_idx < lines.len() - 1 {
            tokens.push(Token::LineBreak);
        }
    }

    // 括弧の深度に応じた変換を実行（Vector内のみ）
    convert_vector_brackets_by_depth(&mut tokens)?;
    
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER FINAL TOKENS: {:?}", tokens)));
    
    Ok(tokens)
}

// Vector内のみ深度に応じて括弧タイプを自動変換
fn convert_vector_brackets_by_depth(tokens: &mut [Token]) -> Result<(), String> {
    let mut i = 0;
    
    while i < tokens.len() {
        if matches!(tokens[i], Token::VectorStart(_)) {
            let vector_end = find_matching_vector_end(tokens, i)?;
            convert_single_vector_brackets(&mut tokens[i..=vector_end])?;
            i = vector_end + 1;
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

// tokenizer.rs の try_parse_ascii_builtin 関数にデバッグ追加
fn try_parse_ascii_builtin(chars: &[char]) -> Option<(Token, usize)> {
    let builtin_words = [
        "true", "false", "nil", "NIL",
        // ワークスペース操作
        "DUP", "SWAP", "ROT",
        // 位置指定操作
        "GET", "INSERT", "REPLACE", "REMOVE",
        // 量指定操作
        "LENGTH", "TAKE", "DROP", "REPEAT", "SPLIT",
        // Vector構造操作
        "CONCAT", "REVERSE",
        // 算術演算
        "+", "-", "*", "/",
        // 比較演算
        "=", "<", "<=", ">", ">=",
        // 論理演算
        "AND", "OR", "NOT",
        // 入出力
        "PRINT",  // ← これが抜けていた！
        // ワード管理・システム
        "DEF", "DEL", "RESET"
    ];
    
    for word in &builtin_words {
        if chars.len() >= word.len() {
            let candidate: String = chars[..word.len()].iter().collect();
            if candidate == *word {
                if chars.len() == word.len() || !chars[word.len()].is_ascii_alphanumeric() {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                        "TOKENIZER DEBUG: Matched builtin '{}' from chars starting with '{}'", 
                        word, 
                        chars.iter().take(10).collect::<String>()
                    )));
                    
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

// 演算子記号解析（> と >= 復活）
fn try_parse_operator(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() {
        return None;
    }
    
    // 2文字演算子を先にチェック
    if chars.len() >= 2 {
        let two_char: String = chars[..2].iter().collect();
        match two_char.as_str() {
            "<=" => return Some((Token::Symbol("<=".to_string()), 2)),
            ">=" => return Some((Token::Symbol(">=".to_string()), 2)),
            _ => {}
        }
    }
    
    // 1文字演算子
    match chars[0] {
        '+' => Some((Token::Symbol("+".to_string()), 1)),
        '-' => Some((Token::Symbol("-".to_string()), 1)),
        '*' => Some((Token::Symbol("*".to_string()), 1)),
        '/' => Some((Token::Symbol("/".to_string()), 1)),
        '<' => Some((Token::Symbol("<".to_string()), 1)),
        '>' => Some((Token::Symbol(">".to_string()), 1)),
        '=' => Some((Token::Symbol("=".to_string()), 1)),
        _ => None,
    }
}
