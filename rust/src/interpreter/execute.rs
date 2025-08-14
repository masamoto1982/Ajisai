// rust/src/interpreter/execute.rs

use crate::types::{Value, ValueType, Token};
use crate::tokenizer::tokenize;
use super::{Interpreter, error::{AjisaiError, Result}};
use super::{stack_ops::*, arithmetic::*, vector_ops::*, control::*, io::*, register_ops::*};
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

        self.process_line_from_tokens(&tokens)
    }

    pub(super) fn process_line_from_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        console::log_1(&JsValue::from_str("--- process_line_from_tokens ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));

        // 最後が "文字列" DEF のパターンをチェック（明示的な命名）
        if tokens.len() >= 2 {
            let last_idx = tokens.len() - 1;
            if let (Some(Token::Symbol(def_sym)), Some(Token::String(name))) = 
                (tokens.get(last_idx), tokens.get(last_idx - 1)) {
                if def_sym == "DEF" {
                    let body_tokens = &tokens[..last_idx - 1];
                    if body_tokens.is_empty() {
                        return Err(AjisaiError::from("DEF requires a body"));
                    }
                    
                    let rpn_tokens = self.rearrange_tokens(body_tokens);
                    return self.define_named_word(name.clone(), rpn_tokens);
                }
            }
        }
        
        // 単一トークンの場合
        if tokens.len() == 1 {
            return self.handle_single_token(&tokens[0]);
        }
        
        // ベクトルリテラルの特別処理（[ ... ]は直接実行）
        if tokens.first() == Some(&Token::VectorStart) && 
           tokens.last() == Some(&Token::VectorEnd) {
            return self.execute_tokens_with_context(tokens);
        }
        
        // パターン: WORD [ ... ] の場合、特別処理
        if tokens.len() >= 2 {
            if let Token::Symbol(name) = &tokens[0] {
                if tokens[1] == Token::VectorStart && 
                   tokens.last() == Some(&Token::VectorEnd) &&
                   self.dictionary.contains_key(name) {
                    // まずベクトルを評価
                    self.execute_tokens_with_context(&tokens[1..])?;
                    // その後ワードを実行（暗黙の反復が適用される）
                    return self.execute_word_with_implicit_iteration(name);
                }
            }
        }
        
        // パターン: [ ... ] WORD の場合、通常通り実行（暗黙の反復が適用される）
        if tokens.len() >= 2 {
            if tokens.first() == Some(&Token::VectorStart) {
                // ベクトルの終端を探す
                let mut depth = 0;
                let mut vec_end_idx = None;
                for (i, token) in tokens.iter().enumerate() {
                    match token {
                        Token::VectorStart => depth += 1,
                        Token::VectorEnd => {
                            depth -= 1;
                            if depth == 0 {
                                vec_end_idx = Some(i);
                                break;
                            }
                        },
                        _ => {}
                    }
                }
                
                // ベクトルの後に単一のワードがある場合
                if let Some(end_idx) = vec_end_idx {
                    if end_idx == tokens.len() - 2 {
                        if let Token::Symbol(name) = &tokens[tokens.len() - 1] {
                            if self.dictionary.contains_key(name) {
                                // 通常の実行（暗黙の反復が自動的に適用される）
                                return self.execute_tokens_with_context(tokens);
                            }
                        }
                    }
                }
            }
        }
        
        // 二項演算の段階的処理（メインロジック）
        self.process_binary_operations(tokens)
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

    // 二項演算の段階的処理（エラーハンドリング強化）
    fn process_binary_operations(&mut self, tokens: &[Token]) -> Result<()> {
        console::log_1(&JsValue::from_str("--- process_binary_operations ---"));
        
        // 二項演算のパターンを検出して段階的に処理
        let operations = self.parse_binary_operations(tokens)?;
        console::log_1(&JsValue::from_str(&format!("Parsed operations: {:?}", operations)));
        
        if operations.is_empty() {
            // 二項演算が検出されない場合、単一要素かチェック
            if tokens.len() == 1 {
                // 単一トークンは別途処理
                return self.handle_single_token(&tokens[0]);
            } else {
                // 複数トークンで二項演算が検出されない場合はエラー
                let token_strs: Vec<String> = tokens.iter()
                    .map(|t| self.token_to_string(t))
                    .collect();
                return Err(AjisaiError::from(format!(
                    "Cannot parse input as binary operations: [{}]. \
                     Input must be either a single value/word or complete binary operations.",
                    token_strs.join(" ")
                )));
            }
        }

        // 段階的にワード定義を実行
        let mut current_result = String::new();
        
        for (i, op) in operations.iter().enumerate() {
            console::log_1(&JsValue::from_str(&format!("Processing operation {}: {:?}", i, op)));
            
            let word_name = if op.left.is_empty() {
                // 単項演算の場合
                if i == 0 {
                    self.handle_unary_operation(&op.operator, &op.right)?
                } else {
                    self.handle_unary_operation(&op.operator, &current_result)?
                }
            } else {
                // 二項演算の場合
                if i == 0 {
                    self.define_binary_operation(&op.left, &op.operator, &op.right)?
                } else {
                    self.define_binary_operation(&current_result, &op.operator, &op.right)?
                }
            };
            
            current_result = word_name;
            console::log_1(&JsValue::from_str(&format!("Generated word: {}", current_result)));
        }

        // 最終的なワード名を記録
        self.auto_named = true;
        self.last_auto_named_word = Some(current_result);
        
        Ok(())
    }

    // token_to_string メソッドを追加（既存の実装から移動）
    fn token_to_string(&self, token: &Token) -> String {
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

    // フォールバック：従来の方式でのワード定義
    fn fallback_word_definition(&mut self, tokens: &[Token]) -> Result<()> {
        console::log_1(&JsValue::from_str("--- fallback_word_definition ---"));
        
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
        let mut new_dependencies = std::collections::HashSet::new();
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
                .or_insert_with(std::collections::HashSet::new)
                .insert(name.clone());
        }

        self.dictionary.insert(name.clone(), super::WordDefinition {
            tokens: storage_tokens,
            is_builtin: false,
            is_temporary: true,
            description: None,
        });

        let is_producer = self.check_if_value_producer(&name);
        self.word_properties.insert(name.clone(), super::WordProperty {
            is_value_producer: is_producer,
        });

        console::log_1(&JsValue::from_str("--- end fallback_word_definition ---"));
        Ok(())
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

    // 二項演算に基づくワード定義（修正版）
    fn define_binary_operation(&mut self, left: &str, operator: &str, right: &str) -> Result<String> {
        console::log_1(&JsValue::from_str(&format!("--- define_binary_operation: {} {} {} ---", left, operator, right)));
        
        // 演算子を標準名に変換
        let op_name = self.get_operator_name(operator);

        // ワード名を生成（修正：フォーマット文字列を修正）
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

    // 単項演算子の処理（修正版）
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

    // 演算子名の標準化（修正版：Stringを返す）
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
                        // 結果を「値をスタックに加える」形式に変換
                        return Ok(vec![
                            Token::Number(result_num, result_den),
                            Token::Symbol("+".to_string())
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
                        return Ok(vec![
                            Token::Number(result_num, result_den),
                            Token::Symbol("+".to_string())
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

    // 以下は既存のメソッド（変更なし）

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
            self.execute_custom_word_with_iteration(name, &def.tokens)
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
                    // 内部のワードも暗黙の反復を適用
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
}
