// in: masamoto1982/ajisai/Ajisai-e4b1951ad8cf96ca24706236c945342f04c7cf22/rust/src/tokenizer.rs

use crate::types::Token;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        // 空白をスキップ
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        // 行コメント処理（#から行末まで）
        if ch == '#' {
            chars.next();
            while let Some(&ch) = chars.peek() {
                chars.next();
                if ch == '\n' {
                    break;
                }
            }
            continue;
        }

        // 説明文処理（DEF用）
        if ch == '(' {
            chars.next();
            let mut description = String::new();
            while let Some(&ch) = chars.peek() {
                chars.next();
                if ch == ')' {
                    break;
                }
                description.push(ch);
            }
            tokens.push(Token::Description(description.trim().to_string()));
            continue;
        }

        // 文字列リテラル
        if ch == '"' {
            chars.next();
            let mut string = String::new();
            let mut escaped = false;

            while let Some(&ch) = chars.peek() {
                chars.next();
                if escaped {
                    string.push(ch);
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    break;
                } else {
                    string.push(ch);
                }
            }
            tokens.push(Token::String(string));
            continue;
        }

        // ベクター開始/終了
        if ch == '[' {
            chars.next();
            tokens.push(Token::VectorStart);
            continue;
        }
        if ch == ']' {
            chars.next();
            tokens.push(Token::VectorEnd);
            continue;
        }
        
        // ブロック開始/終了
        if ch == '{' {
            chars.next();
            tokens.push(Token::BlockStart);
            continue;
        }
        if ch == '}' {
            chars.next();
            tokens.push(Token::BlockEnd);
            continue;
        }

        // その他のトークン（数値、真偽値、NIL、シンボル）
        let mut word = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() || ['(', ')', '[', ']', '{', '}', '"', '#'].contains(&ch) {
                break;
            }
            word.push(ch);
            chars.next();
        }

        if word.is_empty() {
            continue;
        }
        
        // デバッグログ
        web_sys::console::log_1(&format!("Tokenizing word: '{}'", word).into());
        
        // まず特殊なケースを処理
        match word.as_str() {
            // 単独の演算子はシンボルとして処理
            "+" | "-" | "*" | "/" | ">" | ">=" | "=" | "<" | "<=" => {
                tokens.push(Token::Symbol(word.to_uppercase()));
                continue;
            },
            // 真偽値
            "true" => {
                tokens.push(Token::Boolean(true));
                continue;
            },
            "false" => {
                tokens.push(Token::Boolean(false));
                continue;
            },
            // NIL
            "NIL" | "nil" => { // nilも受け入れる
                tokens.push(Token::Nil);
                continue;
            },
            _ => {}
        }
        
        // 数値の判定（整数）
        // 数値の判定部分の修正
// まず数値として解析を試みる
if let Ok(num) = word.parse::<i64>() {
    tokens.push(Token::Number(num, 1));
} else if word.contains('.') && !word.starts_with('.') && !word.ends_with('.') {
    // 小数点を含む場合、分数に変換
    let parts: Vec<&str> = word.split('.').collect();
    if parts.len() == 2 {
        if let (Ok(integer_part), Ok(decimal_part)) = (
            parts[0].parse::<i64>().or_else(|_| if parts[0].is_empty() { Ok(0) } else { Err(()) }),
            parts[1].parse::<i64>().or_else(|_| if parts[1].is_empty() { Ok(0) } else { Err(()) })
        ) {
            let decimal_places = parts[1].len() as u32;
            let denominator = 10_i64.pow(decimal_places);
            let numerator = integer_part * denominator + decimal_part;
            
            web_sys::console::log_1(&format!("Parsed decimal {} as fraction {}/{}", word, numerator, denominator).into());
            tokens.push(Token::Number(numerator, denominator));
        } else {
            // 数値として解析できない場合はシンボルとして扱う
            tokens.push(Token::Symbol(word.to_uppercase()));
        }
    } else {
        tokens.push(Token::Symbol(word.to_uppercase()));
    }
} else if word.contains('/') && word != "/" {
    // 分数記法の処理（既存のまま）
    let parts: Vec<&str> = word.split('/').collect();
    if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
        if let (Ok(numerator), Ok(denominator)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
            if denominator != 0 {
                web_sys::console::log_1(&format!("Parsed fraction {} as {}/{}", word, numerator, denominator).into());
                tokens.push(Token::Number(numerator, denominator));
            } else {
                tokens.push(Token::Symbol(word.to_uppercase()));
            }
        } else {
            tokens.push(Token::Symbol(word.to_uppercase()));
        }
    } else {
        tokens.push(Token::Symbol(word.to_uppercase()));
    }
} else {
    // その他はすべてシンボル
    tokens.push(Token::Symbol(word.to_uppercase()));
}
    }
    
    Ok(tokens)
}
