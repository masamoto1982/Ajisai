use crate::types::Token;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

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

        // 「」文字列リテラル対応
        if ch == '「' {
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
                } else if ch == '」' {
                    break;
                } else {
                    string.push(ch);
                }
            }
            tokens.push(Token::String(string));
            continue;
        }

        // ""文字列リテラル対応（後方互換性）
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

        // []のみサポート
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

        let mut word = String::new();
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() || ['[', ']', '「', '」', '"', '#', '(', ')'].contains(&ch) {
                break;
            }
            word.push(ch);
            chars.next();
        }

        if word.is_empty() {
            continue;
        }

        // 自然日本語解析適用
        let processed_tokens = process_natural_japanese(&word);
        tokens.extend(processed_tokens);
    }
    
    Ok(tokens)
}

// 自然日本語解析（辞書ベース文字抽出＋送り仮名対応）
fn process_natural_japanese(word: &str) -> Vec<Token> {
    // 基本的なワード処理
    match word {
        "true" => return vec![Token::Boolean(true)],
        "false" => return vec![Token::Boolean(false)],
        "NIL" | "nil" => return vec![Token::Nil],
        _ => {}
    }

    // 数値チェック
    if let Ok(num) = word.parse::<i64>() {
        return vec![Token::Number(num, 1)];
    }

    // 分数チェック
    if word.contains('/') {
        let parts: Vec<&str> = word.split('/').collect();
        if parts.len() == 2 {
            if let (Ok(num), Ok(den)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                if den != 0 {
                    return vec![Token::Number(num, den)];
                }
            }
        }
    }

    // 小数チェック
    if word.contains('.') {
        let parts: Vec<&str> = word.split('.').collect();
        if parts.len() == 2 {
            let integer_part = if parts[0].is_empty() { 
                0 
            } else { 
                match parts[0].parse::<i64>() {
                    Ok(n) => n,
                    Err(_) => return vec![Token::Symbol(word.to_uppercase())],
                }
            };
            let decimal_part = if parts[1].is_empty() { 
                0 
            } else {
                match parts[1].parse::<i64>() {
                    Ok(n) => n,
                    Err(_) => return vec![Token::Symbol(word.to_uppercase())],
                }
            };
            
            let decimal_places = parts[1].len() as u32;
            let denominator = 10_i64.pow(decimal_places);
            let numerator = integer_part * denominator + decimal_part;
            
            return vec![Token::Number(numerator, denominator)];
        }
    }

    // 辞書ベース文字抽出（将来実装）
    // 現在は漢字一文字＋送り仮名対応のみ実装
    extract_dictionary_words(word)
}

// 辞書ベース文字抽出＋送り仮名処理
fn extract_dictionary_words(text: &str) -> Vec<Token> {
    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let mut found = false;

        // 漢字一文字の組み込みワードをチェック（送り仮名対応）
        if let Some(kanji_word) = extract_kanji_builtin(&chars[i..]) {
            result.push(Token::Symbol(kanji_word.to_uppercase()));
            i += skip_okurigana(&chars[i..]);
            found = true;
        }

        if !found {
            // 通常のシンボルとして処理
            let symbol: String = chars[i..].iter().collect();
            result.push(Token::Symbol(symbol.to_uppercase()));
            break;
        }
    }

    if result.is_empty() {
        result.push(Token::Symbol(text.to_uppercase()));
    }

    result
}

// 漢字の組み込みワード抽出
fn extract_kanji_builtin(chars: &[char]) -> Option<String> {
    if chars.is_empty() {
        return None;
    }

    let first_char = chars[0];
    let kanji_str = first_char.to_string();

    // 組み込みワードの漢字かチェック（将来的に辞書から動的取得）
    match kanji_str.as_str() {
        // 論理演算
        "否" => Some("NOT".to_string()),
        "且" => Some("AND".to_string()),
        "或" => Some("OR".to_string()),
        
        // 存在チェック
        "無" => Some("NIL?".to_string()),
        "有" => Some("SOME?".to_string()),
        
        // Vector操作
        "頭" => Some("HEAD".to_string()),
        "尾" => Some("TAIL".to_string()),
        "接" => Some("CONS".to_string()),
        "離" => Some("UNCONS".to_string()),
        "追" => Some("APPEND".to_string()),
        "除" => Some("REMOVE_LAST".to_string()),
        "複" => Some("CLONE".to_string()),
        "選" => Some("SELECT".to_string()),
        
        // 統一操作
        "数" => Some("COUNT".to_string()),
        "在" => Some("AT".to_string()),
        "行" => Some("DO".to_string()),
        
        // 制御・定義
        "定" => Some("DEF".to_string()),
        "削" => Some("DEL".to_string()),
        "跳" => Some("LEAP".to_string()),
        "忘" => Some("AMNESIA".to_string()),
        
        _ => None,
    }
}

// 送り仮名をスキップ
fn skip_okurigana(chars: &[char]) -> usize {
    let mut count = 1; // 漢字1文字分

    // 後続のひらがなをスキップ
    while count < chars.len() {
        let ch = chars[count];
        if matches!(ch, 'あ'..='ん' | 'ー' | '。' | '、' | '！' | '？') {
            count += 1;
        } else {
            break;
        }
    }

    count
}
