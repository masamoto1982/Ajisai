// rust/src/interpreter/word_def.rs

use std::collections::{HashSet, VecDeque};
use crate::types::Token;
use super::{Interpreter, WordDefinition, WordProperty, error::{AjisaiError, Result}};
use wasm_bindgen::JsValue;
use web_sys::console;

impl Interpreter {
    pub(super) fn define_named_word(&mut self, name: String, body_tokens: Vec<Token>) -> Result<()> {
        console::log_1(&JsValue::from_str("--- define_named_word ---"));
        console::log_1(&JsValue::from_str(&format!("Defining word: {}", name)));
        console::log_1(&JsValue::from_str(&format!("Body tokens (RPN): {:?}", body_tokens)));
        
        let name = name.to_uppercase();

        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
            }
        }

        if self.dictionary.contains_key(&name) {
            if let Some(dependents) = self.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(AjisaiError::ProtectedWord {
                        name: name.clone(),
                        dependents: dependent_list,
                    });
                }
            }
        }

        let mut new_dependencies = HashSet::new();
        for token in &body_tokens {
            if let Token::Symbol(s) = token {
                if self.dictionary.contains_key(s) {
                    new_dependencies.insert(s.clone());
                }
            }
        }

        for dep_name in &new_dependencies {
            self.dependencies
                .entry(dep_name.clone())
                .or_insert_with(HashSet::new)
                .insert(name.clone());
        }

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: body_tokens,
            is_builtin: false,
            is_temporary: false,  // 明示的に命名されたワードは永続的
            description: None,
        });

        let is_producer = self.check_if_value_producer(&name);
        self.word_properties.insert(name.clone(), WordProperty {
            is_value_producer: is_producer,
        });

        self.append_output(&format!("Defined: {}\n", name));
        console::log_1(&JsValue::from_str("--- end define_named_word ---"));

        Ok(())
    }
    
    // 一時的なワードとその依存関係を再帰的に削除するメソッド
    pub(super) fn delete_temporary_word_cascade(&mut self, word_name: &str) {
        // 削除対象のワードを収集
        let mut words_to_delete = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(word_name.to_string());
        
        while let Some(current_word) = queue.pop_front() {
            // すでに処理済みならスキップ
            if !words_to_delete.insert(current_word.clone()) {
                continue;
            }
            
            // このワードが使用しているワードを探す
            if let Some(def) = self.dictionary.get(&current_word) {
                // 一時的なワードのみ対象
                if def.is_temporary {
                    // トークンから依存しているワードを抽出
                    for token in &def.tokens {
                        if let Token::Symbol(dep_name) = token {
                            if let Some(dep_def) = self.dictionary.get(dep_name) {
                                // 依存先も一時的なワードなら削除対象に追加
                                if dep_def.is_temporary {
                                    queue.push_back(dep_name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 収集したワードをすべて削除
        for word in words_to_delete {
            console::log_1(&JsValue::from_str(&format!("Deleting temporary word: {}", word)));
            
            // 辞書から削除
            self.dictionary.remove(&word);
            self.word_properties.remove(&word);
            
            // 依存関係のクリーンアップ
            self.dependencies.remove(&word);
            for (_, deps) in self.dependencies.iter_mut() {
                deps.remove(&word);
            }
        }
    }

    pub(super) fn define_from_tokens(&mut self, tokens: &[Token]) -> Result<()> {
    console::log_1(&JsValue::from_str("--- define_from_tokens (auto-naming) ---"));
    console::log_1(&JsValue::from_str(&format!("Original tokens: {:?}", tokens)));

    // 名前は元のトークンから生成
    let name = self.generate_word_name(tokens);
    console::log_1(&JsValue::from_str(&format!("Generated name: {}", name)));
    
    if self.dictionary.contains_key(&name) {
        // 既存のワードがある場合
        if let Some(def) = self.dictionary.get(&name).cloned() {
            if def.is_temporary {
                console::log_1(&JsValue::from_str(&format!("Executing temporary word: {}", name)));
                self.execute_word_with_implicit_iteration(&name)?;
                // 実行後に連鎖削除
                self.delete_temporary_word_cascade(&name);
            } else {
                // 永続的なワードの場合は単に実行
                console::log_1(&JsValue::from_str(&format!("Executing permanent word: {}", name)));
                self.execute_word_with_implicit_iteration(&name)?;
            }
        }
        return Ok(());
    }

    // 新規の自動命名ワードを定義（実行はしない）
    self.auto_named = true;
    self.last_auto_named_word = Some(name.clone());

    // 定数式の事前評価を行う
    let processed_tokens = self.preprocess_constant_expressions(tokens)?;
    
    // 処理済みトークンをRPNに変換
    let storage_tokens = self.rearrange_tokens(&processed_tokens);
    console::log_1(&JsValue::from_str(&format!("Storage tokens (RPN): {:?}", storage_tokens)));

    // 依存関係の記録
    let mut new_dependencies = HashSet::new();
    for token in &storage_tokens {
        if let Token::Symbol(s) = token {
            if self.dictionary.contains_key(s) {
                new_dependencies.insert(s.clone());
            }
        }
    }

    for dep_name in &new_dependencies {
        self.dependencies
            .entry(dep_name.clone())
            .or_insert_with(HashSet::new)
            .insert(name.clone());
    }

    self.dictionary.insert(name.clone(), WordDefinition {
        tokens: storage_tokens,
        is_builtin: false,
        is_temporary: true,  // 自動生成されたワードは一時的
        description: None,
    });

    let is_producer = self.check_if_value_producer(&name);
    self.word_properties.insert(name.clone(), WordProperty {
        is_value_producer: is_producer,
    });

    console::log_1(&JsValue::from_str("--- end define_from_tokens ---"));
    Ok(())
}
    
    // 定数式を事前評価するメソッド
fn preprocess_constant_expressions(&self, tokens: &[Token]) -> Result<Vec<Token>> {
    // カスタムワードを含む場合は事前評価しない
    for token in tokens {
        if let Token::Symbol(s) = token {
            if self.dictionary.contains_key(s) && !self.is_operator(s) {
                // カスタムワードが含まれている場合は、そのまま返す
                return Ok(tokens.to_vec());
            }
        }
    }
    
    // 純粋な定数式のみ事前評価
    if tokens.len() == 3 {
        // 中置記法: n1 op n2
        if let (Token::Number(n1, d1), Token::Symbol(op), Token::Number(n2, d2)) = 
            (&tokens[0], &tokens[1], &tokens[2]) {
            if self.is_operator(op) {
                if let Some((result_num, result_den)) = self.evaluate_constant_expression(*n1, *d1, op, *n2, *d2) {
                    console::log_1(&JsValue::from_str(&format!(
                        "Pre-evaluated (infix): {} {} {} = {}/{}", 
                        n1, op, n2, result_num, result_den
                    )));
                    // 結果を単一の値として返す（+を付けない）
                    return Ok(vec![
                        Token::Number(result_num, result_den)
                    ]);
                }
            }
        }
        
        // RPN記法: n1 n2 op
        if let (Token::Number(n1, d1), Token::Number(n2, d2), Token::Symbol(op)) = 
            (&tokens[0], &tokens[1], &tokens[2]) {
            if self.is_operator(op) {
                if let Some((result_num, result_den)) = self.evaluate_constant_expression(*n1, *d1, op, *n2, *d2) {
                    console::log_1(&JsValue::from_str(&format!(
                        "Pre-evaluated (RPN): {} {} {} = {}/{}", 
                        n1, n2, op, result_num, result_den
                    )));
                    // 結果を単一の値として返す（+を付けない）
                    return Ok(vec![
                        Token::Number(result_num, result_den)
                    ]);
                }
            }
        }
    }
    
    Ok(tokens.to_vec())
}
    
    fn evaluate_constant_expression(&self, n1: i64, d1: i64, op: &str, n2: i64, d2: i64) -> Option<(i64, i64)> {
        use crate::types::Fraction;
        
        let f1 = Fraction::new(n1, d1);
        let f2 = Fraction::new(n2, d2);
        
        let result = match op {
            "+" => f1.add(&f2),
            "-" => f1.sub(&f2),
            "*" => f1.mul(&f2),
            "/" => {
                if f2.numerator == 0 {
                    return None;
                }
                f1.div(&f2)
            },
            _ => return None,
        };
        
        Some((result.numerator, result.denominator))
    }

    pub(super) fn generate_word_name(&self, tokens: &[Token]) -> String {
        console::log_1(&JsValue::from_str("--- generate_word_name ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens for naming: {:?}", tokens)));

        // 入力順序のまま名前を生成（RPN変換せず）
        let name_parts: Vec<String> = tokens.iter()
            .map(|token| match token {
                Token::Number(n, d) => {
                    if *d == 1 {
                        n.to_string()
                    } else {
                        format!("{}D{}", n, d)
                    }
                },
                Token::Symbol(s) => {
                    match s.as_str() {
                        "+" => "ADD".to_string(),
                        "-" => "SUB".to_string(),
                        "*" => "MUL".to_string(),
                        "/" => "DIV".to_string(),
                        ">" => "GT".to_string(),
                        ">=" => "GE".to_string(),
                        "=" => "EQ".to_string(),
                        "<" => "LT".to_string(),
                        "<=" => "LE".to_string(),
                        _ => s.clone()
                    }
                },
                Token::VectorStart => "VSTART".to_string(),
                Token::VectorEnd => "VEND".to_string(),
                Token::String(s) => format!("STR_{}", s.replace(" ", "_")),
                Token::Boolean(b) => b.to_string().to_uppercase(),
                Token::Nil => "NIL".to_string(),
            })
            .collect();
        
        let name = name_parts.join("_");
        console::log_1(&JsValue::from_str(&format!("Generated name: {}", name)));
        name
    }

    pub(super) fn check_if_value_producer(&self, word_name: &str) -> bool {
        let mut dummy = Interpreter::new();
        dummy.dictionary = self.dictionary.clone();
        
        if let Some(def) = self.dictionary.get(word_name) {
            if !def.is_builtin {
                match dummy.execute_tokens_with_context(&def.tokens) {
                    Ok(_) => !dummy.stack.is_empty(),
                    Err(_) => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn restore_custom_word(&mut self, name: String, tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        let name = name.to_uppercase();
        
        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(AjisaiError::from(format!("Cannot restore builtin word: {}", name)));
            }
        }

        let mut new_dependencies = HashSet::new();
        for token in &tokens {
            if let Token::Symbol(s) = token {
                if self.dictionary.contains_key(s) {
                    new_dependencies.insert(s.clone());
                }
            }
        }

        for dep_name in &new_dependencies {
            self.dependencies
                .entry(dep_name.clone())
                .or_insert_with(HashSet::new)
                .insert(name.clone());
        }

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens,
            is_builtin: false,
            is_temporary: false,  // 復元されたワードは永続的
            description,
        });

        let is_producer = self.check_if_value_producer(&name);
        self.word_properties.insert(name.clone(), WordProperty {
            is_value_producer: is_producer,
        });

        Ok(())
    }
   
    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        if let Some(def) = self.dictionary.get(name) {
            if !def.is_builtin {
                let body_string = def.tokens.iter()
                    .map(|token| self.token_to_string(token))
                    .collect::<Vec<String>>()
                    .join(" ");
                return Some(format!("{{ {} }}", body_string));
            }
        }
        None
    }
}
