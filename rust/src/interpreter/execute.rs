// rust/src/interpreter/execute.rs

use crate::types::{Value, ValueType, Token};
use crate::tokenizer::tokenize;
use super::{Interpreter, error::{AjisaiError, Result}};
use super::{stack_ops::*, arithmetic::*, vector_ops::*, control::*, io::*, register_ops::*};
use super::{WordDefinition, WordProperty};
use std::collections::HashSet;
use wasm_bindgen::JsValue;
use web_sys::console;

// 二項演算を表す構造体
#[derive(Debug, Clone)]
struct BinaryOperation {
    left: String,
    operator: String,
    right: String,
}

impl Interpreter {
    pub fn execute(&mut self, code: &str) -> Result<()> {
        self.auto_named = false;
        self.last_auto_named_word = None;
        
        let lines: Vec<&str> = code.split('\n')
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        for line in lines {
            self.process_line(line)?;
        }
        
        Ok(())
    }

    pub(super) fn process_line(&mut self, line: &str) -> Result<()> {
        let tokens = tokenize(line).map_err(AjisaiError::from)?;
        if tokens.is_empty() {
            return Ok(());
        }

        // 明示的DEF構文を最優先でチェック（行全体を渡す）
        if let Some((name, body_tokens, description)) = self.parse_explicit_def(&tokens, line) {
            return self.define_explicit_word(name, body_tokens, description);
        }

        self.process_line_from_tokens(&tokens)
    }

    pub(super) fn process_line_from_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        console::log_1(&JsValue::from_str("--- process_line_from_tokens ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));

        if tokens.is_empty() {
            return Ok(());
        }

        // 1. 単一トークンまたは即時実行パターン
        if self.should_execute_immediately(tokens) {
            return self.execute_tokens_with_context(tokens);
        }

        // 2. 二項演算パターンとして自動ワード定義を試行
        if let Ok(()) = self.try_binary_operation_auto_define(tokens) {
            return Ok(());
        }

        // 3. フォールバック：通常実行
        self.execute_tokens_with_context(tokens)
    }

    // 明示的DEF構文の解析（行全体と機能説明対応）
    fn parse_explicit_def(&self, tokens: &[Token], line: &str) -> Option<(String, Vec<Token>, Option<String>)> {
        if tokens.len() >= 2 {
            let last_idx = tokens.len() - 1;
            if let (Some(Token::String(name)), Some(Token::Symbol(def_sym))) = 
                (tokens.get(last_idx - 1), tokens.get(last_idx)) {
                if def_sym == "DEF" {
                    let body_tokens = tokens[..last_idx - 1].to_vec();
                    
                    // DEF以降の機能説明を抽出
                    let description = self.extract_description_from_line(line, tokens);
                    
                    return Some((name.clone(), body_tokens, description));
                }
            }
        }
        None
    }

    fn extract_description_from_line(&self, line: &str, tokens: &[Token]) -> Option<String> {
        // DEF の位置を特定
        let def_position = self.find_def_position_in_line(line, tokens)?;
        
        // DEF以降のテキストを取得
        let after_def_start = def_position + 3; // "DEF"の3文字分
        if after_def_start >= line.len() {
            return None;
        }
        
        let after_def = line[after_def_start..].trim();
        if after_def.is_empty() {
            return None;
        }
        
        // #コメントを除去
        let description = if let Some(comment_pos) = after_def.find('#') {
            after_def[..comment_pos].trim()
        } else {
            after_def.trim()
        };
        
        if description.is_empty() {
            None
        } else {
            Some(description.to_string())
        }
    }

    fn find_def_position_in_line(&self, line: &str, _tokens: &[Token]) -> Option<usize> {
        // 行内で "DEF" キーワードの位置を検索
        // " DEF " または行末の " DEF" を探す
        if let Some(pos) = line.rfind(" DEF ") {
            Some(pos + 1) // " DEF "の開始位置
        } else if line.ends_with(" DEF") {
            Some(line.len() - 3) // "DEF"の開始位置
        } else {
            None
        }
    }

    // 明示的ワード定義（任意の内容を許可）
    fn define_explicit_word(&mut self, name: String, body_tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        let name = name.to_uppercase();
        
        // 既存チェック（ビルトイン保護など）
        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
            }
        }

        // 依存関係の記録
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

        // 機能説明が省略された場合は、ワード内容を使用
        let final_description = description.or_else(|| {
            let body_string = body_tokens.iter()
                .map(|token| self.token_to_string(token))
                .collect::<Vec<String>>()
                .join(" ");
            Some(format!("{{ {} }}", body_string))
        });

        // 永続的なワードとして定義
        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: body_tokens,
            is_builtin: false,
            is_temporary: false,
            description: final_description,  // 機能説明を保存
        });

        self.word_properties.insert(name.clone(), WordProperty {
            is_value_producer: self.check_if_value_producer(&name),
        });

        self.append_output(&format!("Defined: {}\n", name));
        Ok(())
    }

    // 二項演算として自動定義を試行
    fn try_binary_operation_auto_define(&mut self, tokens: &[Token]) -> Result<()> {
        // 既存のprocess_binary_operationsロジックを使用
        let operations = self.parse_binary_operations(tokens)?;
        
        if operations.is_empty() {
            return Err(AjisaiError::from("Not a binary operation pattern"));
        }

        // 二項演算として処理（既存ロジック）
        let mut current_result = String::new();
        
        for (i, op) in operations.iter().enumerate() {
            let word_name = if op.left.is_empty() {
                if i == 0 {
                    self.handle_unary_operation(&op.operator, &op.right)?
                } else {
                    self.handle_unary_operation(&op.operator, &current_result)?
                }
            } else {
                if i == 0 {
                    self.define_binary_operation(&op.left, &op.operator, &op.right)?
                } else {
                    self.define_binary_operation(&current_result, &op.operator, &op.right)?
                }
            };
            current_result = word_name;
        }

        self.auto_named = true;
        self.last_auto_named_word = Some(current_result);
        Ok(())
    }

    // 即時実行判定
    fn should_execute_immediately(&self, tokens: &[Token]) -> bool {
        // 単一リテラル値
        if tokens.len() == 1 {
            match &tokens[0] {
                Token::Number(_, _) | Token::String(_) | Token::Boolean(_) | Token::Nil => true,
                Token::Symbol(name) => self.dictionary.contains_key(name),
                _ => false,
            }
        }
        // ベクトルリテラル
        else if tokens.first() == Some(&Token::VectorStart) && 
                tokens.last() == Some(&Token::VectorEnd) {
            true
        }
        else {
            false
        }
    }

    fn handle_single_token(&mut self, token: &Token) -> Result<()> {
        match token {
            // リテラル値は直接実行
            Token::Number(_, _) | Token::String(_) | Token::Boolean(_) | Token::Nil => {
                self.execute_tokens_with_context(&[token.clone()])
            },
            // 既存のワードは実行
            Token::Symbol(name) => {
                if self.dictionary.contains_key(name) {
                    // 一時的なワードの実行と削除
                    if let Some(def) = self.dictionary.get(name).cloned() {
                        if def.is_temporary {
                            // 一時ワードの実行（暗黙の反復あり）
                            self.execute_word_with_implicit_iteration(name)?;
                            // 連鎖削除
                            self.delete_temporary_word_cascade(name);
                            return Ok(());
                        } else if !def.is_builtin {
                            // 永続的なカスタムワードの場合、暗黙の反復を試みる
                            return self.execute_word_with_implicit_iteration(name);
                        }
                    }
                    return self.execute_tokens_with_context(&[token.clone()]);
                } else {
                    return Err(AjisaiError::UnknownWord(name.clone()));
                }
            },
            // ベクトルの開始/終了だけならエラー
            Token::VectorStart | Token::VectorEnd => {
                return Err(AjisaiError::from("Incomplete vector notation"));
            }
        }
    }

    // 二項演算のパースロジック（完全性チェック付き）
    fn parse_binary_operations(&self, tokens: &[Token]) -> Result<Vec<BinaryOperation>> {
        console::log_1(&JsValue::from_str("--- parse_binary_operations ---"));
        
        let mut operations = Vec::new();
        let mut consumed_tokens = 0;
        let mut i = 0;

        while i < tokens.len() {
            let initial_i = i;
            
            // 単項演算のチェック: NOT a
            if i + 1 < tokens.len() {
                if let Token::Symbol(op) = &tokens[i] {
                    if op == "NOT" {
                        let operand = self.token_to_operand_name(&tokens[i + 1]);
                        operations.push(BinaryOperation {
                            left: "".to_string(),
                            operator: op.clone(),
                            right: operand,
                        });
                        i += 2;
                        consumed_tokens += 2;
                        continue;
                    }
                }
            }

            // 前置記法のチェック: op a b
            if i + 2 < tokens.len() {
                if let Token::Symbol(op) = &tokens[i] {
                    if self.is_operator(op) && op != "NOT" {
                        let left = self.token_to_operand_name(&tokens[i + 1]);
                        let right = self.token_to_operand_name(&tokens[i + 2]);
                        operations.push(BinaryOperation {
                            left,
                            operator: op.clone(),
                            right,
                        });
                        i += 3;
                        consumed_tokens += 3;
                        continue;
                    }
                }
            }

            // 中置記法のチェック: a op b
            if i + 2 < tokens.len() {
                if let Token::Symbol(op) = &tokens[i + 1] {
                    if self.is_operator(op) {
                        let left = self.token_to_operand_name(&tokens[i]);
                        let right = self.token_to_operand_name(&tokens[i + 2]);
                        operations.push(BinaryOperation {
                            left,
                            operator: op.clone(),
                            right,
                        });
                        i += 3;
                        consumed_tokens += 3;
                        continue;
                    }
                }
            }

            // 後置記法のチェック: a b op
            if i + 2 < tokens.len() {
                if let Token::Symbol(op) = &tokens[i + 2] {
                    if self.is_operator(op) {
                        let left = self.token_to_operand_name(&tokens[i]);
                        let right = self.token_to_operand_name(&tokens[i + 1]);
                        operations.push(BinaryOperation {
                            left,
                            operator: op.clone(),
                            right,
                        });
                        i += 3;
                        consumed_tokens += 3;
                        continue;
                    }
                }
            }

            // 二項演算が検出されなかった場合
            if i == initial_i {
                // 進歩がない場合、処理できないトークンがある
                break;
            }
        }

        console::log_1(&JsValue::from_str(&format!("Found {} operations, consumed {} tokens out of {}", 
            operations.len(), consumed_tokens, tokens.len())));

        // 完全性チェック：すべてのトークンが消費されたか確認
        if !operations.is_empty() && consumed_tokens < tokens.len() {
            let remaining_tokens: Vec<String> = tokens[consumed_tokens..].iter()
                .map(|t| self.token_to_string(t))
                .collect();
            
            return Err(AjisaiError::from(format!(
                "Incomplete binary operation detected. Remaining unprocessed tokens: [{}]. \
                 All input must form complete binary operations.",
                remaining_tokens.join(" ")
            )));
        }

        Ok(operations)
    }

    // 演算子判定（拡張版）
    pub(super) fn is_operator(&self, name: &str) -> bool {
        matches!(name, 
            // 基本算術演算子
            "+" | "-" | "*" | "/" |
            // 比較演算子  
            ">" | ">=" | "=" | "<" | "<=" |
            // 論理演算子
            "AND" | "OR" | "NOT" |
            // ベクトル操作
            "CONS" | "APPEND" | "NTH" |
            // レジスタ演算
            "R+" | "R-" | "R*" | "R/" |
            // 条件・Nil操作
            "WHEN" | "DEFAULT" |
            // 数学関数（将来拡張）
            "POW" | "MOD" | "MAX" | "MIN" |
            // 文字列操作（将来拡張）  
            "CONCAT" | "CONTAINS" |
            // 型操作（将来拡張）
            "AS" | "IS"
        )
    }

    // トークンをオペランド名に変換
    fn token_to_operand_name(&self, token: &Token) -> String {
        match token {
            Token::Number(n, d) => {
                if *d == 1 {
                    n.to_string()
                } else {
                    format!("{}D{}", n, d)
                }
            },
            Token::Symbol(s) => s.clone(),
            Token::String(s) => format!("STR_{}", s.replace(" ", "_")),
            Token::Boolean(b) => b.to_string().to_uppercase(),
            Token::Nil => "NIL".to_string(),
            Token::VectorStart => "VSTART".to_string(),
            Token::VectorEnd => "VEND".to_string(),
        }
    }

    // 二項演算に基づくワード定義
    fn define_binary_operation(&mut self, left: &str, operator: &str, right: &str) -> Result<String> {
        console::log_1(&JsValue::from_str(&format!("--- define_binary_operation: {} {} {} ---", left, operator, right)));
        
        // 演算子を標準名に変換
        let op_name = self.get_operator_name(operator);

        // ワード名を生成
        let word_name = format!("{}_{}_{}", left, right, op_name);
        console::log_1(&JsValue::from_str(&format!("Generated word name: {}", word_name)));

        // RPN形式でトークンを構築
        let mut rpn_tokens = Vec::new();
        
        // 左オペランドを追加
        if self.dictionary.contains_key(left) {
            // 既存のカスタムワードの場合
            rpn_tokens.push(Token::Symbol(left.to_string()));
        } else {
            // リテラル値の場合
            rpn_tokens.extend(self.parse_operand_to_tokens(left)?);
        }
        
        // 右オペランドを追加
        if self.dictionary.contains_key(right) {
            // 既存のカスタムワードの場合
            rpn_tokens.push(Token::Symbol(right.to_string()));
        } else {
            // リテラル値の場合
            rpn_tokens.extend(self.parse_operand_to_tokens(right)?);
        }
        
        // 演算子を追加
        rpn_tokens.push(Token::Symbol(operator.to_string()));

        console::log_1(&JsValue::from_str(&format!("RPN tokens: {:?}", rpn_tokens)));

        // ワード定義を実行
        self.define_named_word(word_name.clone(), rpn_tokens)?;
        
        Ok(word_name)
    }

    // 単項演算子の処理
    fn handle_unary_operation(&mut self, operator: &str, operand: &str) -> Result<String> {
        console::log_1(&JsValue::from_str(&format!("--- handle_unary_operation: {} {} ---", operator, operand)));
        
        let op_name = self.get_operator_name(operator);
        let word_name = format!("{}_{}", operand, op_name);
        
        let mut rpn_tokens = Vec::new();
        
        if self.dictionary.contains_key(operand) {
            rpn_tokens.push(Token::Symbol(operand.to_string()));
        } else {
            rpn_tokens.extend(self.parse_operand_to_tokens(operand)?);
        }
        
        rpn_tokens.push(Token::Symbol(operator.to_string()));

        self.define_named_word(word_name.clone(), rpn_tokens)?;
        Ok(word_name)
    }

    // 演算子名の標準化
    fn get_operator_name(&self, operator: &str) -> String {
        match operator {
            // 算術
            "+" => "ADD".to_string(), 
            "-" => "SUB".to_string(), 
            "*" => "MUL".to_string(), 
            "/" => "DIV".to_string(),
            // 比較
            ">" => "GT".to_string(), 
            ">=" => "GE".to_string(), 
            "=" => "EQ".to_string(), 
            "<" => "LT".to_string(), 
            "<=" => "LE".to_string(),
            // 論理
            "AND" => "AND".to_string(), 
            "OR" => "OR".to_string(), 
            "NOT" => "NOT".to_string(),
            // ベクトル
            "CONS" => "CONS".to_string(), 
            "APPEND" => "APPEND".to_string(), 
            "NTH" => "NTH".to_string(),
            // レジスタ
            "R+" => "RADD".to_string(), 
            "R-" => "RSUB".to_string(), 
            "R*" => "RMUL".to_string(), 
            "R/" => "RDIV".to_string(),
            // 条件
            "WHEN" => "WHEN".to_string(), 
            "DEFAULT" => "DEFAULT".to_string(),
            // 数学
            "POW" => "POW".to_string(), 
            "MOD" => "MOD".to_string(), 
            "MAX" => "MAX".to_string(), 
            "MIN" => "MIN".to_string(),
            // 文字列
            "CONCAT" => "CONCAT".to_string(), 
            "CONTAINS" => "CONTAINS".to_string(),
            // 型
            "AS" => "AS".to_string(), 
            "IS" => "IS".to_string(),
            // デフォルト
            _ => operator.to_string()
        }
    }

    // オペランド文字列をトークンに変換
    fn parse_operand_to_tokens(&self, operand: &str) -> Result<Vec<Token>> {
        // 数値の場合
        if let Ok(num) = operand.parse::<i64>() {
            return Ok(vec![Token::Number(num, 1)]);
        }
        
        // 分数の場合 (例: "3D4" → 3/4)
        if operand.contains('D') {
            let parts: Vec<&str> = operand.split('D').collect();
            if parts.len() == 2 {
                if let (Ok(num), Ok(den)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
                    return Ok(vec![Token::Number(num, den)]);
                }
            }
        }
        
        // 真偽値の場合
        if operand == "TRUE" {
            return Ok(vec![Token::Boolean(true)]);
        }
        if operand == "FALSE" {
            return Ok(vec![Token::Boolean(false)]);
        }
        
        // NILの場合
        if operand == "NIL" {
            return Ok(vec![Token::Nil]);
        }
        
        // 文字列の場合 (例: "STR_hello_world")
        if operand.starts_with("STR_") {
            let content = &operand[4..].replace("_", " ");
            return Ok(vec![Token::String(content.to_string())]);
        }
        
        // シンボルの場合
        Ok(vec![Token::Symbol(operand.to_string())])
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

    pub fn execute_tokens_with_context(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;

        while i < tokens.len() {
            let token = &tokens[i];
            match token {
                Token::Number(num, den) => {
                    self.stack.push(Value {
                        val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                    });
                },
                Token::String(s) => {
                    self.stack.push(Value {
                        val_type: ValueType::String(s.clone()),
                    });
                },
                Token::Boolean(b) => {
                    self.stack.push(Value {
                        val_type: ValueType::Boolean(*b),
                    });
                },
                Token::Nil => {
                    self.stack.push(Value {
                        val_type: ValueType::Nil,
                    });
                },
                Token::VectorStart => {
                    let (vector_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                    self.stack.push(Value {
                        val_type: ValueType::Vector(vector_values),
                    });
                    i += consumed - 1;
                },
                Token::Symbol(name) => {
                    self.execute_word_with_implicit_iteration(name)?;
                },
                Token::VectorEnd => {
                    return Err(AjisaiError::from("Unexpected closing delimiter found."));
                },
            }
            i += 1;
        }
        Ok(())
    }

    // ワード実行の統一インターフェース（暗黙の反復を適用）
    pub(super) fn execute_word_with_implicit_iteration(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(name)
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?
            .clone();
        
        if def.is_builtin {
            // ビルトインは既に暗黙の反復が実装されている
            self.execute_builtin(name)
        } else {
            // カスタムワードに暗黙の反復を適用
            let result = self.execute_custom_word_with_iteration(name, &def.tokens);
            
            // 一時的なワードの場合、実行後に連鎖削除
            if def.is_temporary {
                console::log_1(&JsValue::from_str(&format!("Executing and deleting temporary word: {}", name)));
                self.delete_temporary_word_cascade(name);
            }
            
            result
        }
    }

    // 暗黙の反復機能を持つカスタムワード実行（ネスト対応版）
    pub(super) fn execute_custom_word_with_iteration(&mut self, name: &str, tokens: &[Token]) -> Result<()> {
        // スタックトップがベクトルかチェック
        if let Some(top_value) = self.stack.last() {
            if let ValueType::Vector(_) = &top_value.val_type.clone() {
                // ベクトルの場合、再帰的に処理
                let vector = self.stack.pop().unwrap();
                let result = self.apply_word_to_value(name, tokens, &vector)?;
                self.stack.push(result);
                return Ok(());
            }
        }
        
        // ベクトルでない場合は通常の実行
        self.execute_custom_word_normal(name, tokens)
    }

    // 値に対してワードを適用（再帰的にベクトルを処理）
    fn apply_word_to_value(&mut self, name: &str, tokens: &[Token], value: &Value) -> Result<Value> {
        match &value.val_type {
            ValueType::Vector(elements) => {
                // ベクトルの各要素に対して再帰的に適用
                let mut results = Vec::new();
                for elem in elements {
                    let result = self.apply_word_to_value(name, tokens, elem)?;
                    results.push(result);
                }
                Ok(Value {
                    val_type: ValueType::Vector(results)
                })
            },
            _ => {
                // スカラー値の場合、ワードを実行
                self.stack.push(value.clone());
                
                // トークンを実行（内部のカスタムワードも暗黙の反復が適用される）
                self.execute_custom_word_tokens(name, tokens)?;
                
                // 結果を取得（スタックトップから）
                self.stack.pop()
                    .ok_or_else(|| AjisaiError::from("No result from word execution"))
            }
        }
    }
    
    // カスタムワードのトークンを実行（内部の暗黙の反復を維持）
    fn execute_custom_word_tokens(&mut self, name: &str, tokens: &[Token]) -> Result<()> {
        self.call_stack.push(name.to_string());
        
        // トークンを1つずつ実行（Symbol トークンも暗黙の反復を適用）
        let mut i = 0;
        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(num, den) => {
                    self.stack.push(Value {
                        val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)),
                    });
                },
                Token::String(s) => {
                    self.stack.push(Value {
                        val_type: ValueType::String(s.clone()),
                    });
                },
                Token::Boolean(b) => {
                    self.stack.push(Value {
                        val_type: ValueType::Boolean(*b),
                    });
                },
                Token::Nil => {
                    self.stack.push(Value {
                        val_type: ValueType::Nil,
                    });
                },
                Token::VectorStart => {
                    let (vector_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                    self.stack.push(Value {
                        val_type: ValueType::Vector(vector_values),
                    });
                    i += consumed - 1;
                },
                Token::Symbol(sym_name) => {
                    // 内部のワードも暗黙の反復を適用（一時ワードの削除も含む）
                    self.execute_word_with_implicit_iteration(sym_name)
                        .map_err(|e| e.with_context(&self.call_stack))?;
                },
                Token::VectorEnd => {
                    self.call_stack.pop();
                    return Err(AjisaiError::from("Unexpected closing delimiter found."));
                },
            }
            i += 1;
        }
        
        self.call_stack.pop();
        Ok(())
    }
    
    // 通常のカスタムワード実行（暗黙の反復なし）
    fn execute_custom_word_normal(&mut self, name: &str, tokens: &[Token]) -> Result<()> {
        self.call_stack.push(name.to_string());
        let result = self.execute_tokens_with_context(tokens);
        self.call_stack.pop();
        result.map_err(|e| e.with_context(&self.call_stack))
    }

    pub(super) fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name {
            // スタック操作
            "DUP" => op_dup(self),
            "DROP" => op_drop(self),
            "SWAP" => op_swap(self),
            "OVER" => op_over(self),
            "ROT" => op_rot(self),
            "NIP" => op_nip(self),
            ">R" => op_to_r(self),
            "R>" => op_from_r(self),
            "R@" => op_r_fetch(self),
            
            // 算術・比較・論理
            "+" => op_add(self),
            "-" => op_sub(self),
            "*" => op_mul(self),
            "/" => op_div(self),
            ">" => op_gt(self),
            ">=" => op_ge(self),
            "=" => op_eq(self),
            "<" => op_lt(self),
            "<=" => op_le(self),
            "NOT" => op_not(self),
            "AND" => op_and(self),
            "OR" => op_or(self),
            
            // レジスタ演算
            "R+" => op_r_add(self),
            "R-" => op_r_sub(self),
            "R*" => op_r_mul(self),
            "R/" => op_r_div(self),
            
            // ベクトル操作
            "LENGTH" => op_length(self),
            "HEAD" => op_head(self),
            "TAIL" => op_tail(self),
            "CONS" => op_cons(self),
            "APPEND" => op_append(self),
            "REVERSE" => op_reverse(self),
            "NTH" => op_nth(self),
            "UNCONS" => op_uncons(self),
            "EMPTY?" => op_empty(self),
            
            // 制御構造
            "DEL" => op_del(self),
            "DEF" => op_def(self),
            "?" => op_if_select(self),
            "WHEN" => op_when(self),
            
            // Nil関連
            "NIL?" => op_nil_check(self),
            "NOT-NIL?" => op_not_nil_check(self),
            "KNOWN?" => op_not_nil_check(self),
            "DEFAULT" => op_default(self),
            
            // 入出力
            "." => op_dot(self),
            "PRINT" => op_print(self),
            "CR" => op_cr(self),
            "SPACE" => op_space(self),
            "SPACES" => op_spaces(self),
            "EMIT" => op_emit(self),
            
            // データベース操作
            "AMNESIA" => super::op_amnesia(self),
            
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }

    pub(super) fn rearrange_tokens(&self, tokens: &[Token]) -> Vec<Token> {
        console::log_1(&JsValue::from_str("--- rearrange_tokens ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));
        
        // 演算子の位置を特定
        let mut operator_positions = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            if let Token::Symbol(name) = token {
                if self.is_operator(name) {
                    operator_positions.push(i);
                }
            }
        }
        
        if operator_positions.is_empty() {
            console::log_1(&JsValue::from_str("No operators found, returning as-is"));
            return tokens.to_vec();
        }
        
        // 演算子が1つの場合の処理
        if operator_positions.len() == 1 {
            let op_pos = operator_positions[0];
            let op = &tokens[op_pos];
            
            // 後置記法: a b + (既にRPN)
            if op_pos == tokens.len() - 1 && tokens.len() >= 2 {
                console::log_1(&JsValue::from_str("Already in RPN format"));
                return tokens.to_vec();
            }
            
            // 前置記法: + a b
            if op_pos == 0 && tokens.len() >= 3 {
                let mut result = vec![tokens[1].clone(), tokens[2].clone(), op.clone()];
                // 残りのトークンを追加
                for i in 3..tokens.len() {
                    result.push(tokens[i].clone());
                }
                console::log_1(&JsValue::from_str(&format!("Prefix notation converted to RPN: {:?}", result)));
                return result;
            }
            
            // 中置記法: a + b
            if op_pos > 0 && op_pos < tokens.len() - 1 {
                let mut result = vec![tokens[op_pos - 1].clone(), tokens[op_pos + 1].clone(), op.clone()];
                // 残りのトークンを追加（前の部分）
                for i in 0..op_pos-1 {
                    result.insert(i, tokens[i].clone());
                }
                // 残りのトークンを追加（後の部分）
                for i in op_pos + 2..tokens.len() {
                    result.push(tokens[i].clone());
                }
                console::log_1(&JsValue::from_str(&format!("Infix notation converted to RPN: {:?}", result)));
                return result;
            }
            
            // 部分的な式: "2 +" → そのまま（スタックにある値と組み合わせる）
            if op_pos == tokens.len() - 1 && tokens.len() == 2 {
                console::log_1(&JsValue::from_str("Partial expression (value op), keeping as-is"));
                return tokens.to_vec();
            }
        }
        
        console::log_1(&JsValue::from_str("Default: returning as-is"));
        tokens.to_vec()
    }

    pub(super) fn collect_vector_as_data(&self, tokens: &[Token]) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut i = 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorEnd => return Ok((values, i + 1)),
                Token::VectorStart => {
                    let (nested_values, consumed) = self.collect_vector_as_data(&tokens[i..])?;
                    values.push(Value { val_type: ValueType::Vector(nested_values) });
                    i += consumed;
                    continue;
                },
                Token::Number(num, den) => values.push(Value { 
                    val_type: ValueType::Number(crate::types::Fraction::new(*num, *den)) 
                }),
                Token::String(s) => values.push(Value { val_type: ValueType::String(s.clone()) }),
                Token::Boolean(b) => values.push(Value { val_type: ValueType::Boolean(*b) }),
                Token::Nil => values.push(Value { val_type: ValueType::Nil }),
                Token::Symbol(s) => values.push(Value { val_type: ValueType::Symbol(s.clone()) }),
            }
            i += 1;
        }

        Err(AjisaiError::from("Unclosed vector"))
    }

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
            is_temporary: true,  // 二項演算で生成されたワードは一時的
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
        let mut queue = std::collections::VecDeque::new();
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

    pub(super) fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n, d) => if *d == 1 { n.to_string() } else { format!("{}/{}", n, d) },
            Token::String(s) => format!("\"{}\"", s),
            Token::Boolean(b) => b.to_string(),
            Token::Nil => "nil".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
        }
    }
}
