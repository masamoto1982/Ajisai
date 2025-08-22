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

// 自然日本語解析（辞書ベース完全実装）
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
                    Err(_) => return extract_dictionary_words_only(word),
                }
            };
            let decimal_part = if parts[1].is_empty() { 
                0 
            } else {
                match parts[1].parse::<i64>() {
                    Ok(n) => n,
                    Err(_) => return extract_dictionary_words_only(word),
                }
            };
            
            let decimal_places = parts[1].len() as u32;
            let denominator = 10_i64.pow(decimal_places);
            let numerator = integer_part * denominator + decimal_part;
            
            return vec![Token::Number(numerator, denominator)];
        }
    }

    // 辞書ベース文字抽出（辞書語のみ）
    extract_dictionary_words_only(word)
}

// 辞書語のみ抽出（辞書にない文字は完全無視）
fn extract_dictionary_words_only(text: &str) -> Vec<Token> {
    let mut result = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let mut found = false;

        // 漢字一文字の組み込みワードをチェック
        if let Some(kanji_word) = extract_kanji_builtin(&chars[i..]) {
            result.push(Token::Symbol(kanji_word));
            i += skip_okurigana(&chars[i..]);
            found = true;
        } else {
            // 辞書にない文字は無視して次へ
            i += 1;
        }
    }

    // 辞書語が一つも見つからなかった場合
    if result.is_empty() {
        // 完全に英数字のワードかチェック
        if text.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            // 英語ワードとして処理
            result.push(Token::Symbol(text.to_uppercase()));
        }
        // その他（ひらがな・カタカナのみなど）は無視（空のVecを返す）
    }

    result
}

// 漢字の組み込みワード抽出（漢字そのまま返す）
fn extract_kanji_builtin(chars: &[char]) -> Option<String> {
    if chars.is_empty() {
        return None;
    }

    let first_char = chars[0];
    let kanji_str = first_char.to_string();

    // 組み込みワードの漢字かチェック（漢字そのまま返す）
    match kanji_str.as_str() {
        // 論理演算
        "否" => Some("否".to_string()),
        "且" => Some("且".to_string()),
        "或" => Some("或".to_string()),
        
        // 存在チェック
        "無" => Some("無".to_string()),
        "有" => Some("有".to_string()),
        
        // Vector操作（既存）
        "頭" => Some("頭".to_string()),
        "尾" => Some("尾".to_string()),
        "接" => Some("接".to_string()),
        "離" => Some("離".to_string()),
        "追" => Some("追".to_string()),
        "除" => Some("除".to_string()),
        "複" => Some("複".to_string()),
        "選" => Some("選".to_string()),
        "数" => Some("数".to_string()),
        "在" => Some("在".to_string()),
        "行" => Some("行".to_string()),
        
        // Vector操作（新機能）
        "結" => Some("結".to_string()),
        "切" => Some("切".to_string()),
        "反" => Some("反".to_string()),
        "挿" => Some("挿".to_string()),
        "消" => Some("消".to_string()),
        "探" => Some("探".to_string()),
        "含" => Some("含".to_string()),
        "換" => Some("換".to_string()),
        "抽" => Some("抽".to_string()),
        "変" => Some("変".to_string()),
        "畳" => Some("畳".to_string()),
        "並" => Some("並".to_string()),
        "空" => Some("空".to_string()),
        
        // 制御・定義
        "定" => Some("定".to_string()),
        "削" => Some("削".to_string()),
        "跳" => Some("跳".to_string()),
        "忘" => Some("忘".to_string()),
        
        _ => None,
    }
}

// 送り仮名をスキップ（ひらがな、カタカナ、句読点を無視）
fn skip_okurigana(chars: &[char]) -> usize {
    let mut count = 1; // 漢字1文字分

    // 後続の無視すべき文字をスキップ
    while count < chars.len() {
        let ch = chars[count];
        if should_ignore_char(ch) {
            count += 1;
        } else {
            break;
        }
    }

    count
}

// 無視すべき文字の判定
fn should_ignore_char(c: char) -> bool {
    matches!(c, 
        // ひらがな
        'あ'..='ん' | 'ー' |
        // カタカナ  
        'ア'..='ン' | 'ャ'..='ョ' | 'ッ' | 'ー' |
        // 句読点・記号
        '。' | '、' | '！' | '？' | '：' | '；' | 
        // その他の記号
        '〜' | 'ゝ' | 'ゞ' | 'ヽ' | 'ヾ'
    )
}
