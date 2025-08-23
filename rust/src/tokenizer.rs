use crate::types::Token;
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
        
        // 文字列リテラル（""のみ）
        if chars[i] == '"' {
            if let Some((token, consumed)) = parse_string_literal(&chars[i..]) {
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
            // 行末まで無視
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
        
        // 組み込みワードチェック（漢字）
        if let Some((token, consumed)) = try_parse_kanji_builtin(&chars[i..]) {
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

// カスタムワード解析（新機能）
fn try_parse_custom_word(chars: &[char], custom_words: &HashSet<String>) -> Option<(Token, usize)> {
    // 長い単語から優先的にマッチング
    let mut sorted_words: Vec<&String> = custom_words.iter().collect();
    sorted_words.sort_by(|a, b| b.len().cmp(&a.len())); // 長い順でソート
    
    for word in sorted_words {
        if chars.len() >= word.len() {
            let candidate: String = chars[..word.len()].iter().collect();
            if candidate == *word {
                // 単語境界チェック（次の文字が辞書語でない）
                if chars.len() == word.len() || 
                   !is_dictionary_char(chars[word.len()]) {
                    return Some((Token::Symbol(word.clone()), word.len()));
                }
            }
        }
    }
    None
}

// 辞書語の文字かどうかを判定
fn is_dictionary_char(c: char) -> bool {
    // 漢字、英数字、記号かチェック
    match c {
        // 組み込み漢字
        '否' | '且' | '或' | '無' | '有' | '頭' | '尾' | '接' | '離' | '追' | '除' |
        '複' | '復' | '選' | '数' | '在' | '行' | '結' | '切' | '反' | '挿' | '消' |
        '探' | '含' | '換' | '抽' | '変' | '畳' | '並' | '空' | '定' | '削' | '成' | '忘' => true,
        // 英数字
        c if c.is_ascii_alphanumeric() => true,
        // 演算子記号
        '+' | '-' | '*' | '/' | '>' | '<' | '=' => true,
        _ => false,
    }
}

// 文字列リテラル解析
fn parse_string_literal(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() || chars[0] != '"' {
        return None;
    }
    
    let mut string = String::new();
    let mut i = 1; // 開始の"をスキップ
    let mut escaped = false;
    
    while i < chars.len() {
        if escaped {
            string.push(chars[i]);
            escaped = false;
        } else if chars[i] == '\\' {
            escaped = true;
        } else if chars[i] == '"' {
            // 終了の"
            return Some((Token::String(string), i + 1));
        } else {
            string.push(chars[i]);
        }
        i += 1;
    }
    
    // 閉じていない文字列は無効
    None
}

// 数値解析（整数、分数、小数）
fn try_parse_number(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() {
        return None;
    }
    
    // 数字または小数点、負号で始まらない場合は数値ではない
    let first_char = chars[0];
    if !first_char.is_ascii_digit() && first_char != '.' && first_char != '-' {
        return None;
    }
    
    let mut i = 0;
    let mut number_str = String::new();
    
    // 負号の処理
    if chars[i] == '-' {
        // 次の文字が数字または小数点でない場合は演算子として扱う
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
    
    // 数字が全くない場合（例：単独の"-"や"."）
    if number_str.is_empty() || number_str == "-" {
        return None;
    }
    
    // 分数チェック
    if i < chars.len() && chars[i] == '/' {
        number_str.push(chars[i]);
        i += 1;
        
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None; // /の後に数字がない
        }
        
        while i < chars.len() && chars[i].is_ascii_digit() {
            number_str.push(chars[i]);
            i += 1;
        }
        
        // 分数解析
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
        
        // 小数点の後に数字がない場合は有効（例：1.）
        while i < chars.len() && chars[i].is_ascii_digit() {
            number_str.push(chars[i]);
            i += 1;
        }
        
        // 小数→分数変換
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

// 漢字組み込みワード解析
fn try_parse_kanji_builtin(chars: &[char]) -> Option<(Token, usize)> {
    if chars.is_empty() {
        return None;
    }
    
    let kanji = chars[0];
    let kanji_str = kanji.to_string();
    
    // 組み込み漢字ワード辞書
    let builtin_word = match kanji_str.as_str() {
        // 論理演算
        "否" => "否",
        "且" => "且", 
        "或" => "或",
        
        // 存在チェック
        "無" => "無",
        "有" => "有",
        
        // Vector操作（既存）
        "頭" => "頭",
        "尾" => "尾", 
        "接" => "接",
        "離" => "離",
        "追" => "追",
        "除" => "除",
        "複" => "複",
        "復" => "複",  // 復も複製として認識（テストとの互換性）
        "選" => "選",
        "数" => "数",
        "在" => "在",
        "行" => "行",
        
        // Vector操作（新機能）
        "結" => "結",
        "切" => "切",
        "反" => "反", 
        "挿" => "挿",
        "消" => "消",
        "探" => "探",
        "含" => "含",
        "換" => "換",
        "抽" => "抽",
        "変" => "変",
        "畳" => "畳",
        "並" => "並",
        "空" => "空",
        
        // 制御・定義
        "定" => "定",
        "削" => "削",
        "成" => "成",
        "忘" => "忘",
        
        _ => return None,
    };
    
    Some((Token::Symbol(builtin_word.to_string()), 1))
}

// ASCII組み込みワード解析
fn try_parse_ascii_builtin(chars: &[char]) -> Option<(Token, usize)> {
    // 最長マッチング用の候補リスト（長い順）
    let builtin_words = [
        "true", "false", "nil", "NIL", "DEF",
    ];
    
    for word in &builtin_words {
        if chars.len() >= word.len() {
            let candidate: String = chars[..word.len()].iter().collect();
            if candidate == *word {
                // 単語境界チェック（次の文字が英数字でない）
                if chars.len() == word.len() || !chars[word.len()].is_ascii_alphanumeric() {
                    let token = match *word {
                        "true" => Token::Boolean(true),
                        "false" => Token::Boolean(false),
                        "nil" | "NIL" => Token::Nil,
                        "DEF" => Token::Symbol("DEF".to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let input = "1 2 +";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::Number(1, 1));
        assert_eq!(tokens[1], Token::Number(2, 1));
        assert_eq!(tokens[2], Token::Symbol("+".to_string()));
    }

    #[test]
    fn test_custom_word_recognition() {
        let mut custom_words = HashSet::new();
        custom_words.insert("加算複製".to_string());
        
        let tokens = tokenize_with_custom_words("5 加算複製", &custom_words).unwrap();
        
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::Number(5, 1));
        assert_eq!(tokens[1], Token::Symbol("加算複製".to_string()));
    }

    #[test]
    fn test_ignore_non_dictionary_chars() {
        let input = "[ 1 2 3 ]を復シテ、数え2を+";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens.len(), 9);
        assert_eq!(tokens[0], Token::VectorStart);
        assert_eq!(tokens[1], Token::Number(1, 1));
        assert_eq!(tokens[2], Token::Number(2, 1));
        assert_eq!(tokens[3], Token::Number(3, 1));
        assert_eq!(tokens[4], Token::VectorEnd);
        assert_eq!(tokens[5], Token::Symbol("複".to_string()));
        assert_eq!(tokens[6], Token::Symbol("数".to_string()));
        assert_eq!(tokens[7], Token::Number(2, 1));
        assert_eq!(tokens[8], Token::Symbol("+".to_string()));
    }
}
