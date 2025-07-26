use crate::types::Token;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    
    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            },
            '#' => {
                // コメント：行末まで読み飛ばす
                chars.next();
                while let Some(&ch) = chars.peek() {
                    chars.next();
                    if ch == '\n' { break; }
                }
            },
            '[' => {
                chars.next();
                tokens.push(Token::VectorStart);
            },
            ']' => {
                chars.next();
                tokens.push(Token::VectorEnd);
            },
            '"' => {
                chars.next();
                let s = read_string(&mut chars);
                tokens.push(Token::String(s));
            },
            '(' => {
                chars.next();
                let desc = read_description(&mut chars);
                tokens.push(Token::Description(desc));
            },
            _ => {
                let word = read_word(&mut chars);
                if let Some(token) = parse_word(&word) {
                    tokens.push(token);
                }
            }
        }
    }
    
    Ok(tokens)
}

fn read_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    let mut escaped = false;
    
    while let Some(&ch) = chars.peek() {
        chars.next();
        if escaped {
            match ch {
                'n' => s.push('\n'),
                't' => s.push('\t'),
                'r' => s.push('\r'),
                '\\' => s.push('\\'),
                '"' => s.push('"'),
                _ => {
                    s.push('\\');
                    s.push(ch);
                }
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            break;
        } else {
            s.push(ch);
        }
    }
    
    s
}

fn read_description(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    let mut depth = 1;
    
    while depth > 0 && chars.peek().is_some() {
        if let Some(ch) = chars.next() {
            if ch == '(' {
                depth += 1;
                s.push(ch);
            } else if ch == ')' {
                depth -= 1;
                if depth > 0 {
                    s.push(ch);
                }
            } else {
                s.push(ch);
            }
        }
    }
    
    s
}

fn read_word(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut word = String::new();
    
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() || ch == '[' || ch == ']' || ch == '"' || ch == '(' || ch == ')' {
            break;
        }
        word.push(ch);
        chars.next();
    }
    
    word
}

fn parse_word(word: &str) -> Option<Token> {
    if word.is_empty() {
        return None;
    }
    
    // nil
    if word == "nil" {
        return Some(Token::Nil);
    }
    
    // ブール値
    if word == "true" {
        return Some(Token::Boolean(true));
    }
    if word == "false" {
        return Some(Token::Boolean(false));
    }
    
    // 数値のパース
    if let Some((num, den)) = parse_fraction(word) {
        return Some(Token::Number(num, den));
    }
    
    // それ以外はシンボル
    Some(Token::Symbol(word.to_string()))
}

fn parse_fraction(s: &str) -> Option<(i64, i64)> {
    // 分数形式 (例: 1/2)
    if let Some(pos) = s.find('/') {
        let (num_str, den_str) = s.split_at(pos);
        let den_str = &den_str[1..]; // '/'をスキップ
        
        if let (Ok(num), Ok(den)) = (num_str.parse::<i64>(), den_str.parse::<i64>()) {
            if den != 0 {
                return Some((num, den));
            }
        }
        return None;
    }
    
    // 小数形式 (例: 3.14)
    if s.contains('.') {
        if let Ok(f) = s.parse::<f64>() {
            // 簡易的な10進数→分数変換
            let mut denominator = 1i64;
            let mut decimal_places = 0;
            let mut after_dot = false;
            
            for ch in s.chars() {
                if ch == '.' {
                    after_dot = true;
                } else if after_dot && ch.is_digit(10) {
                    decimal_places += 1;
                }
            }
            
            for _ in 0..decimal_places {
                denominator *= 10;
            }
            
            let numerator = (f * denominator as f64).round() as i64;
            return Some((numerator, denominator));
        }
    }
    
    // 整数
    if let Ok(n) = s.parse::<i64>() {
        return Some((n, 1));
    }
    
    None
}
