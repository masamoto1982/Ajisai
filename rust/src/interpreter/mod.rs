// rust/src/interpreter/mod.rs

pub mod stack_ops;
pub mod arithmetic;
pub mod vector_ops;
pub mod control;
pub mod io;
pub mod error;
pub mod register_ops;

use std::collections::{HashMap, HashSet};
use crate::types::{Value, ValueType, Stack, Register, Token};
use crate::tokenizer::tokenize;
use self::error::{AjisaiError, Result};
// コンソールログ出力のために追加
use wasm_bindgen::JsValue;
use web_sys::console;

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) register: Register,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
    pub(crate) call_stack: Vec<String>,
    pub(crate) output_buffer: String,
    word_properties: HashMap<String, WordProperty>,
    // ステップ実行用のフィールド
    step_tokens: Vec<Token>,
    step_position: usize,
    step_mode: bool,
}

#[derive(Clone)]
pub struct WordDefinition {
    pub tokens: Vec<Token>,
    pub is_builtin: bool,
    pub description: Option<String>,
}

#[derive(Clone, Debug)]
pub struct WordProperty {
    pub is_value_producer: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            register: None,
            dictionary: HashMap::new(),
            dependencies: HashMap::new(),
            call_stack: Vec::new(),
            output_buffer: String::new(),
            word_properties: HashMap::new(),
            step_tokens: Vec::new(),
            step_position: 0,
            step_mode: false,
        };
        
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter.initialize_word_properties();
        interpreter
    }

    fn initialize_word_properties(&mut self) {
        // ビルトインワードの性質を定義
        let value_producers = vec![
            "R>", "R@", "DUP", "OVER", "ROT",
        ];
        
        for name in value_producers {
            self.word_properties.insert(name.to_string(), WordProperty {
                is_value_producer: true,
            });
        }
    }
    
    pub fn execute(&mut self, code: &str) -> Result<()> {
        // 改行で分割して各行を処理
        let lines: Vec<&str> = code.split('\n')
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        for line in lines {
            self.process_line(line)?;
        }
        
        Ok(())
    }

    // ステップ実行の初期化
    pub fn init_step_execution(&mut self, code: &str) -> Result<()> {
        // 改行で分割して処理すべき行を収集
        let lines: Vec<&str> = code.split('\n')
            .map(|line| line.trim())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        // 各行をトークン化して、ステップ実行用に保存
        self.step_tokens.clear();
        for line in lines {
            let tokens = tokenize(line).map_err(AjisaiError::from)?;
            if !tokens.is_empty() {
                // 行の区切りを示す特殊なトークンを追加
                if !self.step_tokens.is_empty() {
                    self.step_tokens.push(Token::Symbol("__LINE_BREAK__".to_string()));
                }
                self.step_tokens.extend(tokens);
            }
        }
        
        self.step_position = 0;
        self.step_mode = true;
        Ok(())
    }

    // ステップ実行
    pub fn execute_step(&mut self) -> Result<bool> {
        if !self.step_mode || self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            return Ok(false);
        }

        // 現在の行のトークンを収集
        let mut line_tokens = Vec::new();
        while self.step_position < self.step_tokens.len() {
            let token = &self.step_tokens[self.step_position];
            if let Token::Symbol(name) = token {
                if name == "__LINE_BREAK__" {
                    self.step_position += 1;
                    break;
                }
            }
            line_tokens.push(token.clone());
            self.step_position += 1;
        }

        // 行末に達した場合も処理
        if line_tokens.is_empty() && self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            return Ok(false);
        }

        // 収集したトークンで1行分の処理を実行
        if !line_tokens.is_empty() {
            self.process_line_from_tokens(&line_tokens)?;
        }

        Ok(self.step_position < self.step_tokens.len())
    }

    // ステップ情報の取得
    pub fn get_step_info(&self) -> Option<(usize, usize)> {
        if self.step_mode {
            // 行数をカウント
            let total_lines = self.step_tokens.iter()
                .filter(|t| matches!(t, Token::Symbol(s) if s == "__LINE_BREAK__"))
                .count() + 1;
            
            let current_line = self.step_tokens[..self.step_position.min(self.step_tokens.len())]
                .iter()
                .filter(|t| matches!(t, Token::Symbol(s) if s == "__LINE_BREAK__"))
                .count() + 1;
            
            Some((current_line, total_lines))
        } else {
            None
        }
    }

    fn process_line(&mut self, line: &str) -> Result<()> {
        let tokens = tokenize(line).map_err(AjisaiError::from)?;
        if tokens.is_empty() {
            return Ok(());
        }

        self.process_line_from_tokens(&tokens)
    }

    fn process_line_from_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        // 最後が "文字列" DEF のパターンをチェック
        if tokens.len() >= 2 {
            let last_idx = tokens.len() - 1;
            if let (Some(Token::Symbol(def_sym)), Some(Token::String(name))) = 
                (tokens.get(last_idx), tokens.get(last_idx - 1)) {
                if def_sym == "DEF" {
                    // 明示的な名前付き定義
                    let body_tokens = &tokens[..last_idx - 1];
                    if body_tokens.is_empty() {
                        return Err(AjisaiError::from("DEF requires a body"));
                    }
                    
                    // トークンをRPN順序に並び替える
                    let rpn_tokens = self.rearrange_tokens(body_tokens);
                    
                    // カスタムワードとして定義
                    return self.define_named_word(name.clone(), rpn_tokens);
                }
            }
        }
        
        // 通常の処理（自動命名または即時実行）
        let rearranged = self.rearrange_tokens(tokens);
        
        // 単一のシンボルで、既存のワードと一致する場合は実行
        if rearranged.len() == 1 {
            if let Token::Symbol(name) = &rearranged[0] {
                if self.dictionary.contains_key(name) {
                    return self.execute_tokens_with_context(&rearranged);
                }
            }
        }

        // それ以外は新しいワードとして定義（自動命名）
        // ここで元のトークン（並び替え前）を渡す
        self.define_from_tokens(tokens)?;
        
        Ok(())
    }

    fn rearrange_tokens(&self, tokens: &[Token]) -> Vec<Token> {
        console::log_1(&JsValue::from_str("--- rearrange_tokens ---"));
        console::log_1(&JsValue::from_str(&format!("Input tokens: {:?}", tokens)));

        let mut literals = Vec::new();
        let mut value_producers = Vec::new();
        let mut value_consumers = Vec::new();
        let mut others = Vec::new();

        for token in tokens {
            match token {
                Token::Number(_, _) | Token::String(_) | Token::Boolean(_) | 
                Token::Nil | Token::VectorStart | Token::VectorEnd |
                Token::BlockStart | Token::BlockEnd => {
                    literals.push(token.clone());
                },
                Token::Symbol(name) => {
                    if let Some(prop) = self.word_properties.get(name) {
                        if prop.is_value_producer {
                            value_producers.push(token.clone());
                        } else {
                            value_consumers.push(token.clone());
                        }
                    } else if self.dictionary.contains_key(name) {
                        // 未知のカスタムワードは判定する
                        if self.check_if_value_producer(name) {
                            value_producers.push(token.clone());
                        } else {
                            value_consumers.push(token.clone());
                        }
                    } else {
                        others.push(token.clone());
                    }
                },
            }
        }

        // 順序: リテラル値 → 値生産ワード → 値消費ワード → その他
        let mut result = Vec::new();
        result.extend(literals);
        result.extend(value_producers);
        result.extend(value_consumers);
        result.extend(others);
        
        console::log_1(&JsValue::from_str(&format!("Output tokens (RPN): {:?}", result)));
        console::log_1(&JsValue::from_str("--- end rearrange_tokens ---"));
        result
    }

    fn check_if_value_producer(&self, word_name: &str) -> bool {
        // ダミーのインタープリタでシミュレーション
        let mut dummy = Interpreter::new();
        dummy.dictionary = self.dictionary.clone();
        
        // 空のスタックで実行してみる
        if let Some(def) = self.dictionary.get(word_name) {
            if !def.is_builtin {
                match dummy.execute_tokens_with_context(&def.tokens) {
                    Ok(_) => !dummy.stack.is_empty(), // スタックに値が残れば値生産
                    Err(_) => false, // エラーなら値消費
                }
            } else {
                false // ビルトインは個別に定義済み
            }
        } else {
            false
        }
    }

    // 名前付きワードの定義
    fn define_named_word(&mut self, name: String, body_tokens: Vec<Token>) -> Result<()> {
        console::log_1(&JsValue::from_str("--- define_named_word ---"));
        console::log_1(&JsValue::from_str(&format!("Defining word: {}", name)));
        console::log_1(&JsValue::from_str(&format!("Body tokens (RPN): {:?}", body_tokens)));
        
        let name = name.to_uppercase();

        // ビルトインワードは再定義不可
        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
            }
        }

        // 依存関係チェック
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

        // 新しい依存関係を収集
        let mut new_dependencies = HashSet::new();
        for token in &body_tokens {
            if let Token::Symbol(s) = token {
                if self.dictionary.contains_key(s) {
                    new_dependencies.insert(s.clone());
                }
            }
        }

        // 依存関係を更新
        for dep_name in &new_dependencies {
            self.dependencies
                .entry(dep_name.clone())
                .or_insert_with(HashSet::new)
                .insert(name.clone());
        }

        // 新しいワードを登録
        self.dictionary.insert(name.clone(), WordDefinition {
            tokens: body_tokens,
            is_builtin: false,
            description: None,
        });

        // ワードの性質を判定
        let is_producer = self.check_if_value_producer(&name);
        self.word_properties.insert(name.clone(), WordProperty {
            is_value_producer: is_producer,
        });

        // 定義成功を出力
        self.append_output(&format!("Defined: {}\n", name));
        console::log_1(&JsValue::from_str("--- end define_named_word ---"));

        Ok(())
    }
    
    fn define_from_tokens(&mut self, tokens: &[Token]) -> Result<()> {
    console::log_1(&JsValue::from_str("--- define_from_tokens (auto-naming) ---"));
    console::log_1(&JsValue::from_str(&format!("Original tokens: {:?}", tokens)));

    // 内容ベースの名前を生成（元のトークンから）
    let name = self.generate_word_name(tokens);
    console::log_1(&JsValue::from_str(&format!("Generated name: {}", name)));
    
    // 既存ワードの依存関係チェック
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
        // 依存されていなければ、同じ定義の再入力として扱い、エラーにしない
        self.append_output(&format!("Word '{}' already exists.\n", name));
        console::log_1(&JsValue::from_str("--- end define_from_tokens (already exists) ---"));
        return Ok(());
    }

    // 保存用のトークンは並び替え済みのものを使用
    let storage_tokens = self.rearrange_tokens(tokens);
    console::log_1(&JsValue::from_str(&format!("Storage tokens (RPN): {:?}", storage_tokens)));

    // 新しい依存関係を収集
    let mut new_dependencies = HashSet::new();
    for token in &storage_tokens {
        if let Token::Symbol(s) = token {
            if self.dictionary.contains_key(s) {
                new_dependencies.insert(s.clone());
            }
        }
    }

    // 依存関係を更新
    for dep_name in &new_dependencies {
        self.dependencies
            .entry(dep_name.clone())
            .or_insert_with(HashSet::new)
            .insert(name.clone());
    }

    // ワードを定義（並び替え済みトークンを保存）
    self.dictionary.insert(name.clone(), WordDefinition {
        tokens: storage_tokens,
        is_builtin: false,
        description: None,
    });

    // ワードの性質を判定して記録
    let is_producer = self.check_if_value_producer(&name);
    self.word_properties.insert(name.clone(), WordProperty {
        is_value_producer: is_producer,
    });

    // 定義成功を出力
    self.append_output(&format!("Defined: {}\n", name));
    console::log_1(&JsValue::from_str("--- end define_from_tokens ---"));

    Ok(())
}

    fn generate_word_name(&self, tokens: &[Token]) -> String {
    console::log_1(&JsValue::from_str("--- generate_word_name ---"));
    console::log_1(&JsValue::from_str(&format!("Input tokens for naming: {:?}", tokens)));

    // まずカスタムワードを展開
    let expanded_tokens = self.expand_tokens_for_naming(tokens);
    console::log_1(&JsValue::from_str(&format!("Expanded tokens: {:?}", expanded_tokens)));
    
    // 展開後のトークンをRPN順序に並び替え
    let rpn_tokens = self.rearrange_tokens(&expanded_tokens);
    console::log_1(&JsValue::from_str(&format!("RPN tokens for naming: {:?}", rpn_tokens)));
    
    let final_name = rpn_tokens.iter()
        .map(|token| match token {
            Token::Number(n, d) => {
                if *d == 1 {
                    n.to_string()
                } else {
                    format!("{}_{}", n, d)  // 分数は_で表現
                }
            },
            Token::String(s) => format!("STR_{}", s.replace(" ", "_")),
            Token::Boolean(b) => b.to_string().to_uppercase(),
            Token::Symbol(s) => s.clone(),
            Token::Nil => "NIL".to_string(),
            Token::VectorStart => "VSTART".to_string(),
            Token::VectorEnd => "VEND".to_string(),
            Token::BlockStart => "BSTART".to_string(),
            Token::BlockEnd => "BEND".to_string(),
        })
        .collect::<Vec<String>>()
        .join("_")
        .trim_end_matches('_')  // 末尾の_を除去
        .to_string();

    console::log_1(&JsValue::from_str(&format!("Generated final name: {}", final_name)));
    console::log_1(&JsValue::from_str("--- end generate_word_name ---"));
    
    final_name
}

    // カスタムワードを再帰的に展開する
    fn expand_tokens_for_naming(&self, tokens: &[Token]) -> Vec<Token> {
        let mut expanded = Vec::new();
        
        for token in tokens {
            match token {
                Token::Symbol(name) => {
                    // カスタムワードの場合、その定義を展開
                    if let Some(def) = self.dictionary.get(name) {
                        if !def.is_builtin {
                            // カスタムワードの定義を再帰的に展開
                            let inner_expanded = self.expand_tokens_for_naming(&def.tokens);
                            expanded.extend(inner_expanded);
                        } else {
                            // ビルトインワードはそのまま
                            expanded.push(token.clone());
                        }
                    } else {
                        // 未知のシンボルもそのまま
                        expanded.push(token.clone());
                    }
                },
                _ => {
                    // その他のトークンはそのまま
                    expanded.push(token.clone());
                }
            }
        }
        
        expanded
    }

    pub fn get_output(&mut self) -> String {
        let output = self.output_buffer.clone();
        self.output_buffer.clear();
        output
    }
    
    pub(crate) fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
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
                Token::BlockStart => {
                    let (block_tokens, next_index) = self.collect_block_tokens(tokens, i)?;
                    self.stack.push(Value {
                        val_type: ValueType::Quotation(block_tokens),
                    });
                    i = next_index - 1;
                },
                Token::Symbol(name) => {
                    if let Some(def) = self.dictionary.get(name).cloned() {
                        if def.is_builtin {
                            self.execute_builtin(name)?;
                        } else {
                            self.execute_custom_word(name, &def.tokens)?;
                        }
                    } else {
                        return Err(AjisaiError::UnknownWord(name.clone()));
                    }
                },
                Token::VectorEnd | Token::BlockEnd => {
                    return Err(AjisaiError::from("Unexpected closing delimiter found."));
                },
            }
            i += 1;
        }
        Ok(())
    }

    fn execute_custom_word(&mut self, name: &str, tokens: &[Token]) -> Result<()> {
        self.call_stack.push(name.to_string());
        let result = self.execute_tokens_with_context(tokens);
        self.call_stack.pop();
        
        result.map_err(|e| e.with_context(&self.call_stack))
    }

    fn collect_vector_as_data(&self, tokens: &[Token]) -> Result<(Vec<Value>, usize)> {
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
                _ => {}
            }
            i += 1;
        }

        Err(AjisaiError::from("Unclosed vector"))
    }

    fn collect_block_tokens(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Token>, usize)> {
        let mut block_tokens = Vec::new();
        let mut depth = 1;
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::BlockStart => depth += 1,
                Token::BlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((block_tokens, i + 1));
                    }
                },
                _ => {}
            }
            block_tokens.push(tokens[i].clone());
            i += 1;
        }

        Err(AjisaiError::from("Unclosed block"))
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        use self::{stack_ops::*, arithmetic::*, vector_ops::*, control::*, io::*, register_ops::*};
        
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
            "IF" => op_if(self),
            "DEL" => op_del(self),
            "CALL" => op_call(self),
            "DEF" => control::op_def(self),
            
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
            "AMNESIA" => op_amnesia(self),
            
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }
    
    // アクセサメソッド群
    pub fn get_stack(&self) -> &Stack { &self.stack }
    pub fn get_register(&self) -> &Register { &self.register }
    
    pub fn get_custom_words(&self) -> Vec<String> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect()
    }
    
    pub fn get_custom_words_with_descriptions(&self) -> Vec<(String, Option<String>)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| (name.clone(), def.description.clone()))
            .collect()
    }
   
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                let is_protected = self.dependencies.get(name).map_or(false, |deps| !deps.is_empty());
                (name.clone(), def.description.clone(), is_protected)
            })
            .collect()
    }
   
    pub fn set_stack(&mut self, stack: Stack) {
        self.stack = stack;
    }
   
    pub fn set_register(&mut self, register: Register) {
        self.register = register;
    }

    pub fn restore_custom_word(&mut self, name: String, tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        let name = name.to_uppercase();
        
        // ビルトインワードは復元不可
        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(AjisaiError::from(format!("Cannot restore builtin word: {}", name)));
            }
        }

        // 新しい依存関係を収集
        let mut new_dependencies = HashSet::new();
        for token in &tokens {
            if let Token::Symbol(s) = token {
                if self.dictionary.contains_key(s) {
                    new_dependencies.insert(s.clone());
                }
            }
        }

        // 依存関係を更新
        for dep_name in &new_dependencies {
            self.dependencies
                .entry(dep_name.clone())
                .or_insert_with(HashSet::new)
                .insert(name.clone());
        }

        // ワードを定義
        self.dictionary.insert(name.clone(), WordDefinition {
            tokens,
            is_builtin: false,
            description,
        });

        // ワードの性質を判定して記録
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

    fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n, d) => if *d == 1 { n.to_string() } else { format!("{}/{}", n, d) },
            Token::String(s) => format!("\"{}\"", s),
            Token::Boolean(b) => b.to_string(),
            Token::Nil => "nil".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::BlockStart => "{".to_string(),
            Token::BlockEnd => "}".to_string(),
        }
    }
}

// AMNESIA操作の実装
fn op_amnesia(_interp: &mut Interpreter) -> Result<()> {
    if let Some(window) = web_sys::window() {
        let event = web_sys::CustomEvent::new("ajisai-amnesia")
            .map_err(|_| AjisaiError::from("Failed to create amnesia event"))?;
        window.dispatch_event(&event)
            .map_err(|_| AjisaiError::from("Failed to dispatch amnesia event"))?;
    }
    Ok(())
}
