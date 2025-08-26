// rust/src/tokenizer.rs (丸括弧コメント対応完全版 + 位置情報付きトークン対応)

use crate::types::Token;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub struct TokenWithPosition {
    pub token: Token,
    pub start: usize,
    pub end: usize,
}

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
        
        // 文字列リテラル（""のみ）
        if chars[i] == '"' {
            if let Some((token, consumed)) = parse_string_literal(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
        }
        
        // 丸括弧コメント（機能説明）
        if chars[i] == '(' {
            if let Some((token, consumed)) = parse_paren_comment(&chars[i..]) {
                tokens.push(token);
                i += consumed;
                continue;
            }
        }
        
        // ベクトル記号
        if chars[i] == '[' {
            tokens.push(Token::VectorStart);
            i += 1;
            continue;
        }
        
        if chars[i] == ']' {
            tokens.push(Token::VectorEnd);
            i += 1;
            continue;
        }
        
        // コメント（#から行末まで）
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

        // 新しい組み込み漢字ワード解析
        if let Some((token, consumed)) = try_parse_builtin_kanji(&chars[i..]) {
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
    
    Ok(tokens)
}

pub fn tokenize_with_positions(input: &str) -> Result<Vec<TokenWithPosition>, String> {
    tokenize_with_positions_and_custom_words(input, &HashSet::new())
}

pub fn tokenize_with_positions_and_custom_words(input: &str, custom_words: &HashSet<String>) -> Result<Vec<TokenWithPosition>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut byte_pos = 0;

    while i < chars.len() {
        let start_pos = byte_pos;
        
        // 空白文字をスキップ
        if chars[i].is_whitespace() {
            byte_pos += chars[i].len_utf8();
            i += 1;
            continue;
        }
        
        // 文字列リテラル（""のみ）
        if chars[i] == '"' {
            if let Some((token, consumed)) = parse_string_literal(&chars[i..]) {
                let end_pos = byte_pos + chars[i..i+consumed].iter().map(|c| c.len_utf8()).sum::<usize>();
                tokens.push(TokenWithPosition {
                    token,
                    start: start_pos,
                    end: end_pos,
                });
                byte_pos = end_pos;
                i += consumed;
                continue;
            }
        }
        
        // 丸括弧コメント（機能説明）
        if chars[i] == '(' {
            if let Some((token, consumed)) = parse_paren_comment(&chars[i..]) {
                let end_pos = byte_pos + chars[i..i+consumed].iter().map(|c| c.len_utf8()).sum::<usize>();
                tokens.push(TokenWithPosition {
                    token,
                    start: start_pos,
                    end: end_pos,
                });
                byte_pos = end_pos;
                i += consumed;
                continue;
            }
        }
        
        // ベクトル記号
        if chars[i] == '[' {
            byte_pos += chars[i].len_utf8();
            tokens.push(TokenWithPosition {
                token: Token::VectorStart,
                start: start_pos,
                end: byte_pos,
            });
            i += 1;
            continue;
        }
        
        if chars[i] == ']' {
            byte_pos += chars[i].len_utf8();
            tokens.push(TokenWithPosition {
                token: Token::VectorEnd,
                start: start_pos,
                end: byte_pos,
            });
            i += 1;
            continue;
        }
        
        // コメント（#から行末まで）
        if chars[i] == '#' {
            while i < chars.len() && chars[i] != '\n' {
                byte_pos += chars[i].len_utf8();
                i += 1;
            }
            continue;
        }
        
        // 数値チェック（整数、分数、小数）
        if let Some((token, consumed)) = try_parse_number(&chars[i..]) {
            let end_pos = byte_pos + chars[i..i+consumed].iter().map(|c| c.len_utf8()).sum::<usize>();
            tokens.push(TokenWithPosition {
                token,
                start: start_pos,
                end: end_pos,
            });
            byte_pos = end_pos;
            i += consumed;
            continue;
        }
        
        // カスタムワードチェック（最優先）
        if let Some((token, consumed)) = try_parse_custom_word(&chars[i..], custom_words) {
            let end_pos = byte_pos + chars[i..i+consumed].iter().map(|c| c.len_utf8()).sum::<usize>();
            tokens.push(TokenWithPosition {
                token,
                start: start_pos,
                end: end_pos,
            });
            byte_pos = end_pos;
            i += consumed;
            continue;
        }

        // 新しい組み込みワードチェック（漢字）
        if let Some((token, consumed)) = try_parse_builtin_kanji(&chars[i..]) {
            let end_pos = byte_pos + chars[i..i+consumed].iter().map(|c| c.len_utf8()).sum::<usize>();
            tokens.push(TokenWithPosition {
                token,
                start: start_pos,
                end: end_pos,
            });
            byte_pos = end_pos;
            i += consumed;
            continue;
        }
        
        // 組み込みワードチェック（英数字）
        if let Some((token, consumed)) = try_parse_ascii_builtin(&chars[i..]) {
            let end_pos = byte_pos + chars[i..i+consumed].iter().map(|c| c.len_utf8()).sum::<usize>();
            tokens.push(TokenWithPosition {
                token,
                start: start_pos,
                end: end_pos,
            });
            byte_pos = end_pos;
            i += consumed;
            continue;
        }
        
        // 演算子記号チェック
        if let Some((token, consumed)) = try_parse_operator(&chars[i..]) {
            let end_pos = byte_pos + chars[i..i+consumed].iter().map(|c| c.len_utf8()).sum::<usize>();
            tokens.push(TokenWithPosition {
                token,
                start: start_pos,
                end: end_pos,
            });
            byte_pos = end_pos;
            i += consumed;
            continue;
        }
        
        // どれにもマッチしなければ無視して次へ
        byte_pos += chars[i].len_utf8();
        i += 1;
    }
    
    Ok(tokens)
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
    c.is_ascii_alphanumeric() || c.is_alphabetic() // 全ての文字（漢字、ひらがな、カタカナ含む）を単語文字とする
}

// 文字列リテラル解析
fn parse_string_literal(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() || chars[0] != '"' {
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
        } else if chars[i] == '"' {
            return Some((Token::String(string), i + 1));
        } else {
            string.push(chars[i]);
        }
        i += 1;
    }
    
    None
}

// 丸括弧コメント（機能説明）解析
fn parse_paren_comment(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() || chars[0] != '(' {
        return None;
    }
    
    let mut comment = String::new();
    let mut i = 1; // '(' の次から開始
    let mut depth = 1; // ネストした丸括弧に対応
    
    while i < chars.len() && depth > 0 {
        match chars[i] {
            '(' => {
                depth += 1;
                comment.push(chars[i]);
            },
            ')' => {
                depth -= 1;
                if depth > 0 {
                    comment.push(chars[i]);
                }
            },
            c => {
                comment.push(c);
            }
        }
        i += 1;
    }
    
    if depth == 0 {
        // 前後の空白を除去
        Some((Token::ParenComment(comment.trim().to_string()), i))
    } else {
        // 閉じ括弧がない場合はエラーとして扱わず、無視する
        None
    }
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

// 新しい組み込み漢字ワード解析
fn try_parse_builtin_kanji(chars: &[char]) -> Option<(Token, usize)> {
    // 2文字の組み込みワードを先にチェック
    if chars.len() >= 2 {
        let two_char: String = chars[..2].iter().collect();
        match two_char.as_str() {
            "頁数" => return Some((Token::Symbol("頁数".to_string()), 2)),
            "挿入" => return Some((Token::Symbol("挿入".to_string()), 2)),
            "置換" => return Some((Token::Symbol("置換".to_string()), 2)),
            "削除" => return Some((Token::Symbol("削除".to_string()), 2)),
            "合併" => return Some((Token::Symbol("合併".to_string()), 2)),
            "分離" => return Some((Token::Symbol("分離".to_string()), 2)),
            "待機" => return Some((Token::Symbol("待機".to_string()), 2)),
            "複製" => return Some((Token::Symbol("複製".to_string()), 2)),
            "破棄" => return Some((Token::Symbol("破棄".to_string()), 2)),
            "雇用" => return Some((Token::Symbol("雇用".to_string()), 2)),
            "解雇" => return Some((Token::Symbol("解雇".to_string()), 2)),
            "交代" => return Some((Token::Symbol("交代".to_string()), 2)),
            _ => {}
        }
    }
    
    // 1文字の組み込みワードをチェック
    if !chars.is_empty() {
        let one_char = chars[0];
        let one_char_str = one_char.to_string();
        
        match one_char_str.as_str() {
            "頁" => Some((Token::Symbol("頁".to_string()), 1)),
            _ => None,
        }
    } else {
        None
    }
}

// ASCII組み込みワード解析
fn try_parse_ascii_builtin(chars: &[char]) -> Option<(Token, usize)> {
    let builtin_words = [
        "true", "false", "nil", "NIL", "DEF", "DEL",
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
                        "DEF" => Token::Symbol("雇用".to_string()),  // DEF → 雇用
                        "DEL" => Token::Symbol("解雇".to_string()),  // DEL → 解雇
                        _ => Token::Symbol(word.to_uppercase()),
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
