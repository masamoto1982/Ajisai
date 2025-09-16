// rust/src/tokenizer.rs - 新しい構文対応版

use crate::types::{Token, RepeatControl, TimeControl};
use std::collections::HashSet;
use web_sys::console;
use wasm_bindgen::JsValue;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    console::log_1(&JsValue::from_str(&format!("=== TOKENIZER START ===\nInput: '{}'", input)));
    
    let result = tokenize_with_custom_words(input, &HashSet::new());
    
    match &result {
        Ok(tokens) => console::log_1(&JsValue::from_str(&format!("Tokenized successfully: {:?}", tokens))),
        Err(e) => console::log_1(&JsValue::from_str(&format!("Tokenization error: {}", e))),
    }
    
    console::log_1(&JsValue::from_str("=== TOKENIZER END ==="));
    result
}

pub fn tokenize_with_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let lines: Vec<&str> = input.lines().collect();

    console::log_1(&JsValue::from_str(&format!("Processing {} lines", lines.len())));

    for (line_idx, line) in lines.iter().enumerate() {
        console::log_1(&JsValue::from_str(&format!("Line {}: '{}'", line_idx, line)));
        
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i].is_whitespace() {
                i += 1;
                continue;
            }

            // コメント
            if chars[i] == '#' {
                console::log_1(&JsValue::from_str(&format!("Comment found at position {}", i)));
                break;
            }
            
            // ブラケット記号
            if chars[i] == '[' {
                console::log_1(&JsValue::from_str("Found VectorStart"));
                tokens.push(Token::VectorStart);
                i += 1;
                continue;
            }
            if chars[i] == ']' {
                console::log_1(&JsValue::from_str("Found VectorEnd"));
                tokens.push(Token::VectorEnd);
                i += 1;
                continue;
            }
            
            // 文字列リテラル
            if chars[i] == '\'' {
                console::log_1(&JsValue::from_str(&format!("String literal found at position {}", i)));
                if let Some((token, consumed)) = parse_single_quote_string(&chars[i..]) {
                    console::log_1(&JsValue::from_str(&format!("Parsed string: {:?}, consumed: {}", token, consumed)));
                    tokens.push(token);
                    i += consumed;
                    continue;
                }
            }
            
            // 反復制御単位
            if let Some((token, consumed)) = try_parse_repeat_unit(&chars[i..]) {
                console::log_1(&JsValue::from_str(&format!("Parsed repeat unit: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // 時間制御単位
            if let Some((token, consumed)) = try_parse_time_unit(&chars[i..]) {
                console::log_1(&JsValue::from_str(&format!("Parsed time unit: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // 数値のパース
            if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
                console::log_1(&JsValue::from_str(&format!("Parsed number: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // カスタムワード
            if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
                console::log_1(&JsValue::from_str(&format!("Parsed custom word: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // ビルトインワード
            if let Some((token, consumed)) = try_parse_builtin_word(&chars[i..]) {
                console::log_1(&JsValue::from_str(&format!("Parsed builtin word: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            // 演算子
            if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
                console::log_1(&JsValue::from_str(&format!("Parsed operator: {:?}, consumed: {}", token, consumed)));
                tokens.push(token);
                i += consumed;
                continue;
            }
            
            console::log_1(&JsValue::from_str(&format!("Unrecognized character at position {}: '{}'", i, chars[i])));
            i += 1;
        }
    }

    console::log_1(&JsValue::from_str(&format!("Final tokens: {:?}", tokens)));
    Ok(tokens)
}

fn try_parse_repeat_unit(chars: &[char]) -> Option<(Token, usize)> {
    console::log_1(&JsValue::from_str("=== try_parse_repeat_unit ==="));
    
    // 特殊キーワード
    let special_units = [
        ("WHILE", RepeatControl::While),
        ("UNTIL", RepeatControl::Until),
        ("FOREVER", RepeatControl::Forever),
        ("ONCE", RepeatControl::Once),
    ];
    
    for (keyword, unit) in &special_units {
        if chars.len() >= keyword.len() {
            let candidate: String = chars[..keyword.len()].iter().collect();
            if candidate == *keyword && (chars.len() == keyword.len() || !is_word_char(chars[keyword.len()])) {
                console::log_1(&JsValue::from_str(&format!("Found special repeat unit: {}", keyword)));
                return Some((Token::RepeatUnit(unit.clone()), keyword.len()));
            }
        }
    }
    
    // 数値+単位のパターン
    let mut i = 0;
    
    // 数値部分の解析
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    
    if i == 0 {
        return None; // 数値がない
    }
    
    let number_str: String = chars[..i].iter().collect();
    let number = match number_str.parse::<u32>() {
        Ok(n) => n,
        Err(_) => return None,
    };
    
    // 単位部分の解析
    if i >= chars.len() {
        return None; // 単位がない
    }
    
    let remaining: String = chars[i..].iter().collect();
    
    if remaining.starts_with("x") && (remaining.len() == 1 || !is_word_char(remaining.chars().nth(1).unwrap())) {
        console::log_1(&JsValue::from_str(&format!("Found Times unit: {}x", number)));
        return Some((Token::RepeatUnit(RepeatControl::Times(number)), i + 1));
    }
    
    if remaining.starts_with("rep") && (remaining.len() == 3 || !is_word_char(remaining.chars().nth(3).unwrap())) {
        console::log_1(&JsValue::from_str(&format!("Found Repetitions unit: {}rep", number)));
        return Some((Token::RepeatUnit(RepeatControl::Repetitions(number)), i + 3));
    }
    
    if remaining.starts_with("iter") && (remaining.len() == 4 || !is_word_char(remaining.chars().nth(4).unwrap())) {
        console::log_1(&JsValue::from_str(&format!("Found Iterations unit: {}iter", number)));
        return Some((Token::RepeatUnit(RepeatControl::Iterations(number)), i + 4));
    }
    
    None
}

fn try_parse_time_unit(chars: &[char]) -> Option<(Token, usize)> {
    console::log_1(&JsValue::from_str("=== try_parse_time_unit ==="));
    
    let mut i = 0;
    
    // 数値部分の解析（小数点対応）
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
        i += 1;
    }
    
    if i == 0 {
        return None; // 数値がない
    }
    
    let number_str: String = chars[..i].iter().collect();
    
    if i >= chars.len() {
        return None; // 単位がない
    }
    
    let remaining: String = chars[i..].iter().collect();
    
    if remaining.starts_with("s") && (remaining.len() == 1 || !is_word_char(remaining.chars().nth(1).unwrap())) {
        if let Ok(seconds) = number_str.parse::<f64>() {
            console::log_1(&JsValue::from_str(&format!("Found Seconds unit: {}s", seconds)));
            return Some((Token::TimeUnit(TimeControl::Seconds(seconds)), i + 1));
        }
    }
    
    if remaining.starts_with("ms") && (remaining.len() == 2 || !is_word_char(remaining.chars().nth(2).unwrap())) {
        if let Ok(ms) = number_str.parse::<u32>() {
            console::log_1(&JsValue::from_str(&format!("Found Milliseconds unit: {}ms", ms)));
            return Some((Token::TimeUnit(TimeControl::Milliseconds(ms)), i + 2));
        }
    }
    
    if remaining.starts_with("fps") && (remaining.len() == 3 || !is_word_char(remaining.chars().nth(3).unwrap())) {
        if let Ok(fps) = number_str.parse::<u32>() {
            console::log_1(&JsValue::from_str(&format!("Found FPS unit: {}fps", fps)));
            return Some((Token::TimeUnit(TimeControl::FPS(fps)), i + 3));
        }
    }
    
    None
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
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None;
        }
    }
    
    // 整数部
    let int_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    
    if i == int_start {
        return None;
    }
    
    // 小数点の処理
    if i < chars.len() && chars[i] == '.' {
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }
    // 分数の処理
    else if i < chars.len() && chars[i] == '/' {
        i += 1;
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            i -= 1;
        } else {
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
        }
    }
    
    if i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.') {
        return None;
    }
    
    let number_str: String = chars[start..i].iter().collect();
    Some((Token::Number(number_str), i - start))
}

fn try_parse_builtin_word(chars: &[char]) -> Option<(Token, usize)> {
    let builtin_words = [
        "true", "false", "nil", "NIL", "DEF",
        "DUP", "SWAP", "ROT", 
        "GET", "INSERT", "REPLACE", "REMOVE", 
        "LENGTH", "TAKE", "DROP", "REPEAT", "SPLIT",
        "CONCAT", "REVERSE", 
        "AND", "OR", "NOT", 
        "PRINT", "DEL", "RESET"
    ];
    
    for word in &builtin_words {
        if chars.len() >= word.len() {
            let candidate: String = chars[..word.len()].iter().collect();
            if candidate == *word {
                if chars.len() > word.len() && is_word_char(chars[word.len()]) {
                    continue;
                }
                
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
    
    // 2文字演算子
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
        '+' | '-' => {
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
