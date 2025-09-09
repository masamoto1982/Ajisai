// rust/src/tokenizer.rs (全面デバッグ強化版)

use crate::types::{Token, BracketType};
use std::collections::HashSet;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** TOKENIZE CALLED ***"));
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Input: '{}'", input)));
    
    let result = tokenize_with_custom_words(input, &HashSet::new());
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Tokenize result: {:?}", result)));
    result
}

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** TOKENIZE_WITH_CUSTOM_WORDS CALLED ***"));
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER INPUT: '{}'", input)));
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Custom words: {:?}", custom_words)));
    
    let mut tokens = Vec::new();
    let lines: Vec<&str> = input.lines().collect();

    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER LINES: {:?}", lines)));
    
    for (line_idx, line) in lines.iter().enumerate() {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("*** Processing line {}: '{}' ***", line_idx, line)));
        
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("*** Processing char at position {}: '{}' ***", i, chars[i])));
            
            // 空白文字をスキップ
            if chars[i].is_whitespace() {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Skipping whitespace"));
                i += 1;
                continue;
            }
            
            // 文字列リテラル（シングルクォート）- 最優先で処理
            if chars[i] == '\'' {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Attempting to parse single quote string"));
                if let Some((token, consumed)) = parse_single_quote_string(&chars[i..]) {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed string token: {:?}, consumed: {}", token, consumed)));
                    tokens.push(token);
                    i += consumed;
                    continue;
                } else {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Failed to parse single quote string"));
                }
            }
            
            // Vector記号
            match chars[i] {
                '[' => {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Found ["));
                    tokens.push(Token::VectorStart(BracketType::Square));
                    i += 1;
                    continue;
                },
                ']' => {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Found ]"));
                    tokens.push(Token::VectorEnd(BracketType::Square));
                    i += 1;
                    continue;
                },
                ':' => {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Found :"));
                    tokens.push(Token::Colon);
                    i += 1;
                    continue;
                },
                _ => {}
            }
            
            // 行コメント（#から行末まで）
            if chars[i] == '#' {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Found comment, breaking"));
                break; // 行の残りをスキップ
            }
            
            // 数値チェック
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Attempting to parse number"));
            if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed number token: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // カスタムワードチェック
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Attempting to parse custom word"));
            if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed custom word token: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // 演算子記号チェック（組み込みワードより先に）
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Attempting to parse operator"));
            if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed operator token: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // 組み込みワードチェック（最後に）
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Attempting to parse builtin from: '{}'", chars[i..].iter().take(10).collect::<String>())));
            if let Some((token, consumed)) = try_parse_ascii_builtin(&chars[i..]) {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed builtin token: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // どれにもマッチしなければ無視して次へ
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("No match, skipping character: '{}'", chars[i])));
            i += 1;
        }
        
        // 各行の終わりに改行トークンを追加（最後の行以外）
        if line_idx < lines.len() - 1 {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Adding line break token"));
            tokens.push(Token::LineBreak);
        }
    }

    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Tokens before bracket conversion: {:?}", tokens)));

    // 括弧の深度に応じた変換を実行（Vector内のみ）
    match convert_vector_brackets_by_depth(&mut tokens) {
        Ok(()) => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Bracket conversion successful"));
        },
        Err(e) => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Bracket conversion error: {}", e)));
            return Err(e);
        }
    }
    
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("TOKENIZER FINAL TOKENS: {:?}", tokens)));
    
    Ok(tokens)
}

// Vector内のみ深度に応じて括弧タイプを自動変換
fn convert_vector_brackets_by_depth(tokens: &mut [Token]) -> Result<(), String> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** CONVERT_VECTOR_BRACKETS_BY_DEPTH ***"));
    
    let mut i = 0;
    
    while i < tokens.len() {
        if matches!(tokens[i], Token::VectorStart(_)) {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Found vector start at position {}", i)));
            
            match find_matching_vector_end(tokens, i) {
                Ok(vector_end) => {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Found matching vector end at position {}", vector_end)));
                    
                    match convert_single_vector_brackets(&mut tokens[i..=vector_end]) {
                        Ok(()) => {
                            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Successfully converted single vector brackets"));
                        },
                        Err(e) => {
                            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Error converting single vector brackets: {}", e)));
                            return Err(e);
                        }
                    }
                    
                    i = vector_end + 1;
                },
                Err(e) => {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Error finding matching vector end: {}", e)));
                    return Err(e);
                }
            }
        } else {
            i += 1;
        }
    }
    
    Ok(())
}

fn find_matching_vector_end(tokens: &[Token], start: usize) -> Result<usize, String> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("*** FIND_MATCHING_VECTOR_END from position {} ***", start)));
    
    let mut depth = 0;
    
    for i in start..tokens.len() {
        match &tokens[i] {
            Token::VectorStart(_) => {
                depth += 1;
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Vector start at {}, depth now {}", i, depth)));
            },
            Token::VectorEnd(_) => {
                depth -= 1;
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Vector end at {}, depth now {}", i, depth)));
                if depth == 0 {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Found matching end at position {}", i)));
                    return Ok(i);
                }
            },
            _ => {}
        }
    }
    
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("ERROR: Unclosed vector found"));
    Err("Unclosed vector found".to_string())
}

fn convert_single_vector_brackets(vector_tokens: &mut [Token]) -> Result<(), String> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** CONVERT_SINGLE_VECTOR_BRACKETS ***"));
    
    let mut depth_stack = Vec::new();
    
    for (i, token) in vector_tokens.iter_mut().enumerate() {
        match token {
            Token::VectorStart(_) => {
                let current_depth = depth_stack.len();
                
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Vector start at position {}, current depth: {}", i, current_depth)));
                
                // 6重ネスト制限
                if current_depth >= 6 {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("ERROR: Maximum nesting depth exceeded"));
                    return Err("Maximum nesting depth of 6 exceeded".to_string());
                }
                
                let bracket_type = match current_depth % 3 {
                    0 => BracketType::Square,  // 1層目、4層目
                    1 => BracketType::Curly,   // 2層目、5層目
                    2 => BracketType::Round,   // 3層目、6層目
                    _ => unreachable!(),
                };
                
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Assigning bracket type: {:?}", bracket_type)));
                
                *token = Token::VectorStart(bracket_type.clone());
                depth_stack.push(bracket_type);
            },
            Token::VectorEnd(_) => {
                if let Some(opening_type) = depth_stack.pop() {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Vector end at position {}, matching with: {:?}", i, opening_type)));
                    *token = Token::VectorEnd(opening_type);
                } else {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("ERROR: Unexpected closing bracket"));
                    return Err("Unexpected closing bracket".to_string());
                }
            },
            _ => {}
        }
    }
    
    Ok(())
}

fn try_parse_custom_word(chars: &[char], custom_words: &HashSet<String>) -> Option<(Token, usize)> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** TRY_PARSE_CUSTOM_WORD ***"));
    
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
            
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Checking custom word '{}' against candidate '{}'", word, candidate)));
            
            if candidate == *word {
                // 単語境界チェック
                if chars.len() == word_char_len || 
                   !is_word_char(chars[word_char_len]) {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Matched custom word: {}", word)));
                    return Some((Token::Symbol(word.clone()), word_char_len));
                }
            }
        }
    }
    
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("No custom word matched"));
    None
}

// 単語文字かどうかを判定
fn is_word_char(c: char) -> bool {
    let result = c.is_ascii_alphanumeric() || c.is_alphabetic();
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("is_word_char('{}') = {}", c, result)));
    result
}

// シングルクォート文字列解析
fn parse_single_quote_string(chars: &[char]) -> Option<(Token, usize)> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** PARSE_SINGLE_QUOTE_STRING ***"));
    
    if chars.is_empty() || chars[0] != '\'' {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Not a single quote string"));
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
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed string: '{}'", string)));
            return Some((Token::String(string), i + 1));
        } else {
            string.push(chars[i]);
        }
        i += 1;
    }
    
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Unclosed string"));
    None
}

// ダブルクォート機能説明コメント解析
fn parse_double_quote_comment(chars: &[char]) -> Option<(Token, usize)> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** PARSE_DOUBLE_QUOTE_COMMENT ***"));
    
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
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed comment: '{}'", comment)));
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
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** TRY_PARSE_NUMBER ***"));
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Input chars: '{}'", chars.iter().take(10).collect::<String>())));
    
    if chars.is_empty() {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Empty input"));
        return None;
    }
    
    let first_char = chars[0];
    if !first_char.is_ascii_digit() && first_char != '.' && first_char != '-' {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("First char '{}' is not numeric", first_char)));
        return None;
    }
    
    let mut i = 0;
    let mut number_str = String::new();
    
    // 負号の処理
    if chars[i] == '-' {
        if i + 1 >= chars.len() || (!chars[i + 1].is_ascii_digit() && chars[i + 1] != '.') {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Invalid negative number"));
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
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Empty or invalid number string"));
        return None;
    }
    
    // 分数チェック
    if i < chars.len() && chars[i] == '/' {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Found fraction"));
        number_str.push(chars[i]);
        i += 1;
        
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Invalid fraction format"));
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
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed fraction: {}/{}", num, den)));
                    return Some((Token::Number(num, den), i));
                }
            }
        }
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Failed to parse fraction"));
        return None;
    }
    
    // 小数チェック
    if i < chars.len() && chars[i] == '.' {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Found decimal"));
        number_str.push(chars[i]);
        i += 1;
        
        while i < chars.len() && chars[i].is_ascii_digit() {
            number_str.push(chars[i]);
            i += 1;
        }
        
        if let Some((num, den)) = parse_decimal(&number_str) {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed decimal as fraction: {}/{}", num, den)));
            return Some((Token::Number(num, den), i));
        }
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Failed to parse decimal"));
        return None;
    }
    
    // 整数
    if let Ok(num) = number_str.parse::<i64>() {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Parsed integer: {}", num)));
        Some((Token::Number(num, 1), i))
    } else {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Failed to parse integer"));
        None
    }
}

// 小数を分数に変換
fn parse_decimal(decimal_str: &str) -> Option<(i64, i64)> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("*** PARSE_DECIMAL: '{}' ***", decimal_str)));
    
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
        
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Decimal conversion result: {}/{}", numerator, denominator)));
        Some((numerator, denominator))
    } else {
        None
    }
}

fn try_parse_ascii_builtin(chars: &[char]) -> Option<(Token, usize)> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** TRY_PARSE_ASCII_BUILTIN ***"));
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Input: '{}'", chars.iter().take(15).collect::<String>())));
    
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
        "PRINT",
        // ワード管理・システム
        "DEF", "DEL", "RESET"
    ];
    
    for word in &builtin_words {
        if chars.len() >= word.len() {
            let candidate: String = chars[..word.len()].iter().collect();
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Checking builtin '{}' against candidate '{}'", word, candidate)));
            
            if candidate == *word {
                if chars.len() == word.len() || !chars[word.len()].is_ascii_alphanumeric() {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Matched builtin: {}", word)));
                    
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
    
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("No builtin matched"));
    None
}

// 演算子記号解析（> と >= 復活）
fn try_parse_operator(chars: &[char]) -> Option<(Token, usize)> {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("*** TRY_PARSE_OPERATOR ***"));
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Input: '{}'", chars.iter().take(5).collect::<String>())));
    
    if chars.is_empty() {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Empty input"));
        return None;
    }
    
    // 2文字演算子を先にチェック
    if chars.len() >= 2 {
        let two_char: String = chars[..2].iter().collect();
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Checking two-char operator: '{}'", two_char)));
        match two_char.as_str() {
            "<=" => {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Matched <="));
                return Some((Token::Symbol("<=".to_string()), 2));
            },
            ">=" => {
                web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Matched >="));
                return Some((Token::Symbol(">=".to_string()), 2));
            },
            _ => {}
        }
    }
    
    // 1文字演算子
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!("Checking single-char operator: '{}'", chars[0])));
    match chars[0] {
        '+' => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Matched +"));
            Some((Token::Symbol("+".to_string()), 1))
        },
        '-' => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Matched -"));
            Some((Token::Symbol("-".to_string()), 1))
        },
        '*' => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Matched *"));
            Some((Token::Symbol("*".to_string()), 1))
        },
        '/' => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Matched /"));
            Some((Token::Symbol("/".to_string()), 1))
        },
        '<' => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Matched <"));
            Some((Token::Symbol("<".to_string()), 1))
        },
        '>' => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Matched >"));
            Some((Token::Symbol(">".to_string()), 1))
        },
        '=' => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("Matched ="));
            Some((Token::Symbol("=".to_string()), 1))
        },
        _ => {
            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("No operator matched"));
            None
        }
    }
}
