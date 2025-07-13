#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(i64, i64),  // 分子, 分母
    String(String),
    Boolean(bool),
    Symbol(String),
    VectorStart,
    VectorEnd,
    Nil,
    Description(String),
}

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
        
        // ベクトル開始/終了
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
        
        // その他のトークン（数値、真偽値、NIL、シンボル）
        let mut word = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() || ch == '(' || ch == '[' || ch == ']' || ch == '"' || ch == '#' {
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
            "NIL" => {
                tokens.push(Token::Nil);
                continue;
            },
            _ => {}
        }
        
        // 数値の判定（整数）
        if let Ok(num) = word.parse::<i64>() {
            tokens.push(Token::Number(num, 1));
        } else if word.contains('.') && !word.starts_with('.') && !word.ends_with('.') {
            // 小数点を含む場合、分数に変換
            let parts: Vec<&str> = word.split('.').collect();
            if parts.len() == 2 {
                // 整数部と小数部を別々に処理
                let integer_part = if parts[0].is_empty() { 0 } else { 
                    parts[0].parse::<i64>().map_err(|_| format!("Invalid number: {}", word))? 
                };
                let decimal_part = if parts[1].is_empty() { 0 } else {
                    parts[1].parse::<i64>().map_err(|_| format!("Invalid number: {}", word))?
                };
                
                let decimal_places = parts[1].len() as u32;
                let denominator = 10_i64.pow(decimal_places);
                let numerator = integer_part * denominator + decimal_part;
                
                web_sys::console::log_1(&format!("Parsed decimal {} as fraction {}/{}", word, numerator, denominator).into());
                tokens.push(Token::Number(numerator, denominator));
            } else {
                return Err(format!("Invalid number: {}", word));
            }
        } else if word.contains('/') && word != "/" {
            // 分数記法（例: 1/2）- 単独の / は既に処理済み
            let parts: Vec<&str> = word.split('/').collect();
            if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                let numerator = parts[0].parse::<i64>()
                    .map_err(|_| format!("Invalid fraction numerator: {}", word))?;
                let denominator = parts[1].parse::<i64>()
                    .map_err(|_| format!("Invalid fraction denominator: {}", word))?;
                
                if denominator == 0 {
                    return Err("Division by zero in fraction".to_string());
                }
                
                web_sys::console::log_1(&format!("Parsed fraction {} as {}/{}", word, numerator, denominator).into());
                tokens.push(Token::Number(numerator, denominator));
            } else {
                // 分数として解析できない場合はシンボルとして扱う
                tokens.push(Token::Symbol(word.to_uppercase()));
            }
        } else {
            // その他はすべてシンボル
            tokens.push(Token::Symbol(word.to_uppercase()));
        }
    }
    
    Ok(tokens)
}
