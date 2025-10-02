// rust/src/interpreter/mod.rs

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;
pub mod audio;

use std::collections::{HashMap, HashSet};
use crate::types::{Stack, Token, Value, ValueType, BracketType, Fraction, WordDefinition, ExecutionLine};
use self::error::{Result, AjisaiError};
use num_traits::{Zero, One, ToPrimitive};
use async_recursion::async_recursion;
use gloo_timers::future::TimeoutFuture;

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependents: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) execution_state: Option<WordExecutionState>,
    pub(crate) definition_to_load: Option<String>,
}

pub struct WordExecutionState {
    pub program_counter: usize,
    pub word_name: String,
    pub continue_loop: bool,
}

async fn sleep(ms: u64) {
    TimeoutFuture::new(ms as u32).await;
}

async fn evaluate_condition(
    dictionary: &HashMap<String, WordDefinition>,
    condition_tokens: &[Token],
    value_to_test: &Value
) -> Result<bool> {
    let mut temp_interp = Interpreter {
        stack: vec![value_to_test.clone()],
        dictionary: dictionary.clone(),
        dependents: HashMap::new(),
        output_buffer: String::new(),
        execution_state: None,
        definition_to_load: None,
    };
    
    temp_interp.execute_tokens(condition_tokens).await?;
    
    if let Some(result_val) = temp_interp.stack.pop() {
        Ok(is_truthy(&result_val))
    } else {
        Ok(false)
    }
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            dictionary: HashMap::new(),
            dependents: HashMap::new(),
            output_buffer: String::new(),
            execution_state: None,
            definition_to_load: None,
        };
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    pub async fn execute(&mut self, code: &str) -> Result<()> {
        if code.contains(" DEF") {
            return control::parse_multiple_word_definitions(self, code);
        }
        
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)?;
        
        self.execute_tokens(&tokens).await
    }

    pub fn execute_tokens_sync(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            
            match token {
                Token::Number(s) => {
                    let frac = Fraction::from_str(s).map_err(AjisaiError::from)?;
                    self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(frac) }], BracketType::Square)});
                },
                Token::String(s) => {
                    self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone()) }], BracketType::Square)});
                },
                Token::Boolean(b) => self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b) }], BracketType::Square)}),
                Token::Nil => self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }], BracketType::Square)}),
                Token::VectorStart(_) => {
                    let (values, consumed) = self.collect_vector(&tokens, i, 1)?;
                    self.stack.push(Value { val_type: ValueType::Vector(values, BracketType::Square) });
                    i += consumed - 1;
                },
                Token::DefBlockStart => {
                    let (body_tokens, consumed) = self.collect_def_block(&tokens, i)?;
                    self.stack.push(Value { val_type: ValueType::DefinitionBody(body_tokens) });
                    i += consumed - 1;
                },
                Token::Symbol(name) => {
                    let upper_name = name.to_uppercase();
                    if let Some(def) = self.dictionary.get(&upper_name).cloned() {
                        if def.is_builtin {
                            self.execute_builtin(&upper_name)?;
                        } else {
                            return Err(AjisaiError::from("Custom words not supported in sync execution mode"));
                        }
                    } else {
                        return Err(AjisaiError::UnknownWord(name.clone()));
                    }
                },
                _ => {}
            }
            i += 1;
        }
        Ok(())
    }

    fn split_tokens_by_lines(&self, tokens: &[Token]) -> Vec<Vec<Token>> {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        
        for token in tokens {
            if matches!(token, Token::LineBreak) {
                if !current_line.is_empty() {
                    lines.push(std::mem::take(&mut current_line));
                }
            } else {
                current_line.push(token.clone());
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }

    #[async_recursion(?Send)]
    pub async fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        let lines = self.split_tokens_by_lines(tokens);
        
        for line_tokens in lines {
            if line_tokens.is_empty() {
                continue;
            }
            
            self.execute_line_with_guard_chain(&line_tokens).await?;
        }
        
        Ok(())
    }

    #[async_recursion(?Send)]
async fn execute_line_with_guard_chain(&mut self, tokens: &[Token]) -> Result<()> {
    let guard_positions: Vec<usize> = tokens.iter()
        .enumerate()
        .filter(|(_, t)| matches!(t, Token::GuardSeparator))
        .map(|(i, _)| i)
        .collect();
    
    if guard_positions.is_empty() {
        return self.execute_single_line_tokens(tokens).await;
    }
    
    // ガードで区切られたセグメントを処理
    let mut segments = Vec::new();
    let mut start = 0;
    
    for &guard_pos in &guard_positions {
        segments.push(&tokens[start..guard_pos]);
        start = guard_pos + 1;
    }
    if start < tokens.len() {
        segments.push(&tokens[start..]);
    }
    
    // 最初のセグメントが空（先頭が:）の場合、デフォルト節として処理
    if segments[0].is_empty() {
        if segments.len() > 1 {
            return self.execute_single_line_tokens(segments[1]).await;
        }
        return Ok(());
    }
    
    // 各条件-アクションペアを評価
    let mut i = 0;
    while i < segments.len() {
        let condition_segment = segments[i];
        
        if condition_segment.is_empty() {
            // 空のセグメント（デフォルト節）
            if i + 1 < segments.len() {
                return self.execute_single_line_tokens(segments[i + 1]).await;
            }
            return Ok(());
        }
        
        // 条件を評価（スタックトップの値を保持）
        let stack_before = self.stack.clone();
        self.execute_single_line_tokens(condition_segment).await?;
        
        if let Some(condition_result) = self.stack.pop() {
            if is_truthy(&condition_result) {
                // 条件が真：次のセグメントを実行
                if i + 1 < segments.len() {
                    // スタックを条件評価前に戻す
                    self.stack = stack_before;
                    return self.execute_single_line_tokens(segments[i + 1]).await;
                }
                return Ok(());
            } else {
                // 条件が偽：スタックを復元して次の条件へ
                self.stack = stack_before;
                i += 2; // 条件とアクションをスキップ
                
                // 次のセグメントがない、または最後のセグメントの場合
                if i >= segments.len() {
                    // 最後のセグメントが残っていればデフォルトとして実行
                    if !segments.is_empty() && segments.len() % 2 == 1 {
                        return self.execute_single_line_tokens(segments[segments.len() - 1]).await;
                    }
                    return Ok(());
                }
            }
        } else {
            return Err(AjisaiError::from("Condition evaluation produced no result"));
        }
    }
    
    Ok(())
}

    #[async_recursion(?Send)]
    async fn execute_single_line_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            
            match token {
                Token::Number(s) => {
                    let frac = Fraction::from_str(s).map_err(AjisaiError::from)?;
                    self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(frac) }], BracketType::Square)});
                },
                Token::String(s) => {
                    self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone()) }], BracketType::Square)});
                },
                Token::Boolean(b) => self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b) }], BracketType::Square)}),
                Token::Nil => self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }], BracketType::Square)}),
                Token::VectorStart(_) => {
                    let (values, consumed) = self.collect_vector(tokens, i, 1)?;
                    self.stack.push(Value { val_type: ValueType::Vector(values, BracketType::Square) });
                    i += consumed - 1;
                },
                Token::DefBlockStart => {
                    let (body_tokens, consumed) = self.collect_def_block(tokens, i)?;
                    self.stack.push(Value { val_type: ValueType::DefinitionBody(body_tokens) });
                    i += consumed - 1;
                },
                Token::Symbol(name) => {
                    self.execute_word(name).await?;
                },
                Token::GuardSeparator => {
                    // ガードセパレータは既に上位で処理済みなのでスキップ
                },
                Token::LineBreak => {
                    // 行内では無視
                },
                _ => {} 
            }
            i += 1;
        }
        Ok(())
    }

    fn collect_def_block(&self, tokens: &[Token], start: usize) -> Result<(Vec<Token>, usize)> {
        let mut depth = 1;
        let mut i = start + 1;
        while i < tokens.len() {
            match tokens[i] {
                Token::DefBlockStart => depth += 1,
                Token::DefBlockEnd => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok((tokens[start + 1..i].to_vec(), i - start + 1));
                    }
                },
                _ => {}
            }
            i += 1;
        }
        Err(AjisaiError::from("Unclosed definition block"))
    }

    #[async_recursion(?Send)]
    async fn execute_word(&mut self, name: &str) -> Result<()> {
        let def = {
            self.dictionary.get(&name.to_uppercase()).cloned()
                .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?
        };

        if def.is_builtin {
            return self.execute_builtin(name);
        }

        let has_conditional_lines = def.lines.iter().any(|line| !line.condition_tokens.is_empty());
        
        if has_conditional_lines {
            let value_to_test = self.stack.last().cloned()
                .ok_or(AjisaiError::StackUnderflow)?;
            
            let mut matched_line: Option<ExecutionLine> = None;
            let dictionary_clone = self.dictionary.clone();

            for line in &def.lines {
                if line.condition_tokens.is_empty() {
                    matched_line = Some(line.clone());
                    break;
                }
                
                if evaluate_condition(&dictionary_clone, &line.condition_tokens, &value_to_test).await? {
                    matched_line = Some(line.clone());
                    break;
                }
            }
            
            if let Some(line) = matched_line {
                let _ = self.stack.pop().unwrap();
                self.stack.push(value_to_test);
                self.execute_line(&line).await?;
            }
        } else {
            let mut i = 0;
            while i < def.lines.len() {
                self.execute_line(&def.lines[i]).await?;
                i += 1;
            }
        }
        
        Ok(())
    }

    #[async_recursion(?Send)]
    async fn execute_line(&mut self, line: &ExecutionLine) -> Result<()> {
        self.execute_tokens(&line.body_tokens).await
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name.to_uppercase().as_str() {
            "GET" => vector_ops::op_get(self), 
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self), 
            "REMOVE" => vector_ops::op_remove(self),
            "LENGTH" => vector_ops::op_length(self), 
            "TAKE" => vector_ops::op_take(self),
            "SPLIT" => vector_ops::op_split(self),
            "CONCAT" => vector_ops::op_concat(self), 
            "REVERSE" => vector_ops::op_reverse(self),
            "+" => arithmetic::op_add(self), 
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self), 
            "/" => arithmetic::op_div(self),
            "=" => arithmetic::op_eq(self), 
            "<" => arithmetic::op_lt(self),
            "<=" => arithmetic::op_le(self), 
            ">" => arithmetic::op_gt(self),
            ">=" => arithmetic::op_ge(self), 
            "AND" => arithmetic::op_and(self),
            "OR" => arithmetic::op_or(self), 
            "NOT" => arithmetic::op_not(self),
            ":" | ";" => {
                Err(AjisaiError::from("':' and ';' can only be used for conditional branching within expressions"))
            },
            "PRINT" => io::op_print(self), 
            "AUDIO" => audio::op_sound(self),
            "TIMES" => self.execute_times(),
            "WAIT" => self.execute_wait(),
            "DEF" => {
                if self.stack.len() >= 2 {
                    control::op_def(self)
                } else {
                    Err(AjisaiError::from("DEF requires definition and name on stack. Usage: : ... ; 'WORD_NAME' DEF"))
                }
            },
            "DEL" => {
                if !self.stack.is_empty() {
                    control::op_del(self)
                } else {
                    Err(AjisaiError::from("DEL requires a word name on stack. Usage: 'WORD_NAME' DEL"))
                }
            },
            "?" => control::op_lookup(self),
            "RESET" => self.execute_reset(),
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }

    fn execute_times(&mut self) -> Result<()> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::from("TIMES requires word name and count. Usage: 'WORD' [ n ] TIMES"));
        }

        let count_val = self.stack.pop().unwrap();
        let name_val = self.stack.pop().unwrap();

        let count = match &count_val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::Number(n) if n.denominator == num_bigint::BigInt::one() => {
                        n.numerator.to_i64().ok_or_else(|| AjisaiError::from("Count too large"))?
                    },
                    _ => return Err(AjisaiError::type_error("integer", "other type")),
                }
            },
            _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
        };

        let word_name = match &name_val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::String(s) => s.clone(),
                    _ => return Err(AjisaiError::type_error("string", "other type")),
                }
            },
            _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
        };

        let upper_name = word_name.to_uppercase();
        
        // カスタムワードかどうかを確認
        if let Some(def) = self.dictionary.get(&upper_name) {
            if def.is_builtin {
                return Err(AjisaiError::from("TIMES can only be used with custom words"));
            }
        } else {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        // スケジュール実行（非同期）
        self.output_buffer.push_str(&format!("[DEBUG] Scheduling {} executions of '{}'\n", count, word_name));
        
        // 注意: この実装は同期的なので、実際には即座に全て実行される
        // 本来は async で実装すべきだが、ここでは簡易実装
        for _ in 0..count {
            
            // execute_tokens は async なので、ここでは execute_tokens_sync を使うか
            // 別の方法が必要... これは問題
            // とりあえず同期実行を試みる
            self.execute_word_sync(&upper_name)?;
        }

        Ok(())
    }

    fn execute_word_sync(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(name).cloned()
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

        if def.is_builtin {
            return self.execute_builtin(name);
        }

        for line in &def.lines {
            for token in &line.body_tokens {
                match token {
                    Token::Number(s) => {
                        let frac = Fraction::from_str(s).map_err(AjisaiError::from)?;
                        self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(frac) }], BracketType::Square)});
                    },
                    Token::String(s) => {
                        self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone()) }], BracketType::Square)});
                    },
                    Token::Boolean(b) => {
                        self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b) }], BracketType::Square)});
                    },
                    Token::Nil => {
                        self.stack.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }], BracketType::Square)});
                    },
                    Token::Symbol(sym_name) => {
                        let sym_def = self.dictionary.get(&sym_name.to_uppercase()).cloned()
                            .ok_or_else(|| AjisaiError::UnknownWord(sym_name.to_string()))?;
                        if sym_def.is_builtin {
                            self.execute_builtin(&sym_name.to_uppercase())?;
                        } else {
                            self.execute_word_sync(&sym_name.to_uppercase())?;
                        }
                    },
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn execute_wait(&mut self) -> Result<()> {
        if self.stack.len() < 2 {
            return Err(AjisaiError::from("WAIT requires word name and delay. Usage: 'WORD' [ ms ] WAIT"));
        }

        let delay_val = self.stack.pop().unwrap();
        let name_val = self.stack.pop().unwrap();

        let delay_ms = match &delay_val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::Number(n) if n.denominator == num_bigint::BigInt::one() => {
                        n.numerator.to_u64().ok_or_else(|| AjisaiError::from("Delay too large"))?
                    },
                    _ => return Err(AjisaiError::type_error("integer", "other type")),
                }
            },
            _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
        };

        let word_name = match &name_val.val_type {
            ValueType::Vector(v, _) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::String(s) => s.clone(),
                    _ => return Err(AjisaiError::type_error("string", "other type")),
                }
            },
            _ => return Err(AjisaiError::type_error("single-element vector with string", "other type")),
        };

        let upper_name = word_name.to_uppercase();
        
        // カスタムワードかどうかを確認
        if let Some(def) = self.dictionary.get(&upper_name) {
            if def.is_builtin {
                return Err(AjisaiError::from("WAIT can only be used with custom words"));
            }
        } else {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        // 注意: 同期実装では待機できない
        // とりあえずメッセージだけ出力
        self.output_buffer.push_str(&format!("[DEBUG] Would wait {}ms before executing '{}'\n", delay_ms, word_name));
        
        // 実際には待機せずに実行
        self.execute_word_sync(&upper_name)?;

        Ok(())
    }
    
    fn collect_vector(&self, tokens: &[Token], start: usize, depth: usize) -> Result<(Vec<Value>, usize)> {
        let mut values = Vec::new();
        let mut d = 1;
        let mut end = 0;

        for i in (start + 1)..tokens.len() {
            match tokens[i] {
                Token::VectorStart(_) => d += 1,
                Token::VectorEnd(_) => {
                    d -= 1;
                    if d == 0 {
                        end = i;
                        break;
                    }
                },
                _ => {}
            }
        }

        if end == 0 {
            return Err(AjisaiError::from("Unclosed vector"));
        }

        let mut i = start + 1;
        while i < end {
            match &tokens[i] {
                Token::VectorStart(_) => {
                    let new_bracket_type = match depth % 3 {
                        1 => BracketType::Curly,
                        2 => BracketType::Round,
                        0 => BracketType::Square,
                        _ => unreachable!(),
                    };
                    let (nested_values, consumed) = self.collect_vector(tokens, i, depth + 1)?;
                    values.push(Value { val_type: ValueType::Vector(nested_values, new_bracket_type) });
                    i += consumed;
                },
                Token::Number(s) => {
                    values.push(Value { val_type: ValueType::Number(Fraction::from_str(s).map_err(AjisaiError::from)?) });
                    i += 1;
                },
                Token::String(s) => {
                    values.push(Value { val_type: ValueType::String(s.clone()) });
                    i += 1;
                },
                Token::Boolean(b) => {
                    values.push(Value { val_type: ValueType::Boolean(*b) });
                    i += 1;
                },
                Token::Nil => {
                    values.push(Value { val_type: ValueType::Nil });
                    i += 1;
                },
                Token::Symbol(name) => {
                    values.push(Value { val_type: ValueType::Symbol(name.clone()) });
                    i += 1;
                },
                _ => { i += 1; }
            }
        }

        Ok((values, end - start + 1))
    }

    pub fn rebuild_dependencies(&mut self) -> Result<()> {
        self.dependents.clear();
        
        let custom_words: Vec<(String, WordDefinition)> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| (name.clone(), def.clone()))
            .collect();
            
        for (word_name, word_def) in custom_words {
            let mut dependencies = HashSet::new();
            
            for line in &word_def.lines {
                for token in line.condition_tokens.iter().chain(line.body_tokens.iter()) {
                    if let Token::Symbol(s) = token {
                        let upper_s = s.to_uppercase();
                        if self.dictionary.contains_key(&upper_s) && !self.dictionary.get(&upper_s).unwrap().is_builtin {
                            dependencies.insert(upper_s.clone());
                            self.dependents.entry(upper_s).or_default().insert(word_name.clone());
                        }
                    }
                }
            }
            
            if let Some(def) = self.dictionary.get_mut(&word_name) {
                def.dependencies = dependencies;
            }
        }
        
        Ok(())
    }

    pub fn get_word_definition_tokens(&self, name: &str) -> Option<String> {
        if let Some(def) = self.dictionary.get(name) {
            if !def.is_builtin && !def.lines.is_empty() {
                let mut result = String::new();
                for (i, line) in def.lines.iter().enumerate() {
                    if i > 0 { result.push('\n'); }
                    
                    if !line.condition_tokens.is_empty() {
                        for token in &line.condition_tokens {
                            result.push_str(&self.token_to_string(token));
                            result.push(' ');
                        }
                        result.push_str(": ");
                    }
                    
                    for token in &line.body_tokens {
                        result.push_str(&self.token_to_string(token));
                        result.push(' ');
                    }
                }
                return Some(result.trim().to_string());
            }
        }
        None
    }
    
    fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(s) => s.clone(),
            Token::String(s) => format!("'{}'", s),
            Token::Boolean(true) => "TRUE".to_string(),
            Token::Boolean(false) => "FALSE".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::Nil => "NIL".to_string(),
            Token::VectorStart(BracketType::Square) => "[".to_string(),
            Token::VectorEnd(BracketType::Square) => "]".to_string(),
            Token::VectorStart(BracketType::Curly) => "{".to_string(),
            Token::VectorEnd(BracketType::Curly) => "}".to_string(),
            Token::VectorStart(BracketType::Round) => "(".to_string(),
            Token::VectorEnd(BracketType::Round) => ")".to_string(),
            Token::GuardSeparator => ":".to_string(),
            Token::DefBlockEnd => ";".to_string(),
            Token::LineBreak => "\n".to_string(),
            _ => "".to_string(),
        }
    }

    pub fn get_output(&mut self) -> String { std::mem::take(&mut self.output_buffer) }
    pub fn get_stack(&self) -> &Stack { &self.stack }
    pub fn set_stack(&mut self, stack: Stack) { self.stack = stack; }
    
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                let is_protected = self.dependents.get(name)
                    .map_or(false, |deps| !deps.is_empty());
                
                (name.clone(), def.description.clone(), is_protected)
            })
            .collect()
    }
    
    pub fn get_word_definition(&self, _name: &str) -> Option<String> { None }
    pub fn restore_custom_word(&mut self, _name: String, _tokens: Vec<Token>, _description: Option<String>) -> Result<()> { Ok(()) }
    
    pub fn execute_reset(&mut self) -> Result<()> {
        self.stack.clear(); 
        self.dictionary.clear();
        self.dependents.clear();
        self.output_buffer.clear(); 
        self.execution_state = None;
        self.definition_to_load = None;
        crate::builtins::register_builtins(&mut self.dictionary);
        Ok(())
    }
}

fn is_truthy(value: &Value) -> bool {
    if let ValueType::Vector(v, _) = &value.val_type {
        if v.len() == 1 {
            return match &v[0].val_type {
                ValueType::Boolean(b) => *b, 
                ValueType::Nil => false,
                ValueType::Number(n) => !n.numerator.is_zero(),
                ValueType::String(s) => !s.is_empty(),
                ValueType::Vector(inner_v, _) => !inner_v.is_empty(),
                _ => true,
            }
        }
    }
    false
}
