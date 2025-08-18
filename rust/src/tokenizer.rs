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

        // 括弧コメント処理（( から ) まで）
        if ch == '(' {
            chars.next();
            let mut depth = 1;
            while let Some(&ch) = chars.peek() {
                chars.next();
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
            }
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

        // クオーテーション開始/終了
        if ch == '{' {
            chars.next();
            tokens.push(Token::QuotationStart);
            continue;
        }
        if ch == '}' {
            chars.next();
            tokens.push(Token::QuotationEnd);
            continue;
        }

        // ワード（数値、真偽値、NIL、シンボル、タイムスタンプラベル）
        let mut word = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() || ['[', ']', '{', '}', '"', '#', '(', ')'].contains(&ch) {
                break;
            }
            word.push(ch);
            chars.next();
        }

        if word.is_empty() {
            continue;
        }

        // タイムスタンプラベル（コロン付き）- 14桁対応
        if word.ends_with(':') && word.len() > 1 {
            let label = word[..word.len()-1].to_string();
            // Base62の4文字英数字ラベルに対応  ← これに変更
    if label.len() == 4 && label.chars().all(|c| c.is_ascii_alphanumeric()) {
        tokens.push(Token::Label(label));
        continue;
    }
        }

        // 特殊ワード
        match word.as_str() {
            "true" => tokens.push(Token::Boolean(true)),
            "false" => tokens.push(Token::Boolean(false)),
            "NIL" | "nil" => tokens.push(Token::Nil),
            _ => {
                // 数値判定
                if let Ok(num) = word.parse::<i64>() {
                    tokens.push(Token::Number(num, 1));
                } else if word.contains('/') {
                    // 分数記法
                    let parts: Vec<&str> = word.split('/').collect();
                    if parts.len() == 2 {
                        if let (Ok(num), Ok(den)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                            if den != 0 {
                                tokens.push(Token::Number(num, den));
                            } else {
                                return Err("Division by zero in fraction".to_string());
                            }
                        } else {
                            tokens.push(Token::Symbol(word.to_uppercase()));
                        }
                    } else {
                        tokens.push(Token::Symbol(word.to_uppercase()));
                    }
                } else if word.contains('.') {
                    // 小数点記法
                    let parts: Vec<&str> = word.split('.').collect();
                    if parts.len() == 2 {
                        let integer_part = if parts[0].is_empty() { 
                            0 
                        } else { 
                            match parts[0].parse::<i64>() {
                                Ok(n) => n,
                                Err(_) => {
                                    tokens.push(Token::Symbol(word.to_uppercase()));
                                    continue;
                                }
                            }
                        };
                        let decimal_part = if parts[1].is_empty() { 
                            0 
                        } else {
                            match parts[1].parse::<i64>() {
                                Ok(n) => n,
                                Err(_) => {
                                    tokens.push(Token::Symbol(word.to_uppercase()));
                                    continue;
                                }
                            }
                        };
                        
                        let decimal_places = parts[1].len() as u32;
                        let denominator = 10_i64.pow(decimal_places);
                        let numerator = integer_part * denominator + decimal_part;
                        
                        tokens.push(Token::Number(numerator, denominator));
                    } else {
                        tokens.push(Token::Symbol(word.to_uppercase()));
                    }
                } else {
                    tokens.push(Token::Symbol(word.to_uppercase()));
                }
            }
        }
    }
    
    Ok(tokens)
}
