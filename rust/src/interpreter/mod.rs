pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;
pub mod flow_control;
pub mod audio;  // 新規追加

use std::collections::{HashMap, HashSet};
use crate::types::{Workspace, Token, Value, ValueType, BracketType, Fraction, WordDefinition, ExecutionLine};
use self::error::{Result, AjisaiError};
use num_traits::Zero;
use std::str::FromStr;

pub struct Interpreter {
    pub(crate) workspace: Workspace,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependents: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) execution_state: Option<WordExecutionState>,
    pub(crate) definition_to_load: Option<String>,
}

pub struct WordExecutionState {
    pub program_counter: usize,
    pub repeat_counters: Vec<i64>,
    pub word_name: String,
    pub continue_loop: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            workspace: Vec::new(),
            dictionary: HashMap::new(),
            dependents: HashMap::new(),
            output_buffer: String::new(),
            execution_state: None,
            definition_to_load: None,
        };
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    pub fn execute(&mut self, code: &str) -> Result<()> {
        if code.contains(" DEF") {
            return control::parse_multiple_word_definitions(self, code);
        }
        
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)?;
        
        self.execute_tokens(&tokens)
    }

    pub fn execute_tokens(&mut self, tokens: &[Token]) -> Result<()> {
        // 条件実行（:）の検出と処理
        if let Some(guard_pos) = tokens.iter().position(|t| matches!(t, Token::GuardSeparator)) {
            return self.execute_conditional_statement(tokens, guard_pos);
        }
        
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            
            match token {
                Token::Number(s) => {
                    let frac = Fraction::from_str(s).map_err(AjisaiError::from)?;
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(frac) }], BracketType::Square)});
                },
                Token::String(s) => {
                    self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone()) }], BracketType::Square)});
                },
                Token::Boolean(b) => self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b) }], BracketType::Square)}),
                Token::Nil => self.workspace.push(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }], BracketType::Square)}),
                Token::VectorStart(_) => {
                    let (values, consumed) = self.collect_vector(tokens, i, 1)?;
                    self.workspace.push(Value { val_type: ValueType::Vector(values, BracketType::Square) });
                    i += consumed - 1;
                },
                Token::DefBlockStart => {
                    let (body_tokens, consumed) = self.collect_def_block(tokens, i)?;
                    self.workspace.push(Value { val_type: ValueType::DefinitionBody(body_tokens) });
                    i += consumed - 1;
                },
                Token::Symbol(name) => {
                    self.execute_word(name)?;
                },
                Token::LineBreak => {},
                _ => {} 
            }
            i += 1;
        }
        Ok(())
    }

    fn execute_conditional_statement(&mut self, tokens: &[Token], guard_pos: usize) -> Result<()> {
    let condition_tokens = &tokens[..guard_pos];
    let body_tokens = &tokens[guard_pos + 1..];
    
    // 修飾子を分離
    let (execution_tokens, repeat_count, delay_ms) = self.parse_modifiers(body_tokens);
    
    // 条件トークンを実行して結果を取得
    self.execute_tokens(condition_tokens)?;
    
    // 条件評価の結果をワークスペースから取得
    let condition_result = self.workspace.pop()
        .ok_or(AjisaiError::from("Condition evaluation produced no result"))?;
    
    // 条件が真の場合のみ実行
    if is_truthy(&condition_result) {
        // 指定回数実行
        for iteration in 0..repeat_count {
            if iteration > 0 && delay_ms > 0 {
                let sleep_result = crate::wasm_sleep(delay_ms);
                self.output_buffer.push_str(&format!("{}\n", sleep_result));
            }
            self.execute_tokens(&execution_tokens)?;
        }
    }
    
    Ok(())
}

    fn parse_modifiers(&self, tokens: &[Token]) -> (Vec<Token>, i64, u64) {
        let mut execution_tokens = Vec::new();
        let mut repeat_count = 1i64;
        let mut delay_ms = 0u64;
        
        for token in tokens {
            match token {
                Token::Modifier(m_str) => {
                    if m_str.ends_with('x') {
                        if let Ok(count) = m_str[..m_str.len()-1].parse::<i64>() {
                            repeat_count = count;
                        }
                    } else if m_str.ends_with("ms") {
                        if let Ok(ms) = m_str[..m_str.len()-2].parse::<u64>() {
                            delay_ms = ms;
                        }
                    } else if m_str.ends_with('s') {
                        if let Ok(s) = m_str[..m_str.len()-1].parse::<u64>() {
                            delay_ms = s * 1000;
                        }
                    }
                },
                _ => execution_tokens.push(token.clone()),
            }
        }
        
        (execution_tokens, repeat_count, delay_ms)
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

    fn execute_word(&mut self, name: &str) -> Result<()> {
    let def = self.dictionary.get(&name.to_uppercase()).cloned()
        .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

    if def.is_builtin {
        return self.execute_builtin(name);
    }

    // 条件付き行があるかチェック
    let has_conditional_lines = def.lines.iter().any(|line| !line.condition_tokens.is_empty());
    
    if has_conditional_lines {
        // 条件分岐モード: 最初に真になった条件の行のみ実行
        let value_to_test = self.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
        
        for line in &def.lines {
            if !line.condition_tokens.is_empty() {
                // 条件を評価
                if self.evaluate_condition(&line.condition_tokens, &value_to_test)? {
                    // 元の値をワークスペースに戻す
                    self.workspace.push(value_to_test);
                    // この行を実行して終了
                    return self.execute_line(line);
                }
            } else {
                // デフォルト行: 元の値をワークスペースに戻して実行
                self.workspace.push(value_to_test);
                return self.execute_line(line);
            }
        }
        
        // どの条件にもマッチしなかった場合、値を戻す
        self.workspace.push(value_to_test);
    } else {
        // 条件なしモード: 全ての行を順次実行
        for line in &def.lines {
            self.execute_line(line)?;
        }
    }
    
    Ok(())
}

fn evaluate_condition(&mut self, condition_tokens: &[Token], value_to_test: &Value) -> Result<bool> {
    // 一時的なインタープリターで条件を評価
    let mut temp_interp = Interpreter {
        workspace: vec![value_to_test.clone()],
        dictionary: self.dictionary.clone(),
        dependents: HashMap::new(),
        output_buffer: String::new(),
        execution_state: None,
        definition_to_load: None,
    };
    
    temp_interp.execute_tokens(condition_tokens)?;
    
    if let Some(result_val) = temp_interp.workspace.pop() {
        Ok(is_truthy(&result_val))
    } else {
        Ok(false)
    }
}

fn execute_line(&mut self, line: &ExecutionLine) -> Result<()> {
    for iteration in 0..line.repeat_count {
        if iteration > 0 && line.delay_ms > 0 {
            let sleep_result = crate::wasm_sleep(line.delay_ms);
            self.output_buffer.push_str(&format!("{}\n", sleep_result));
        }
        self.execute_tokens(&line.body_tokens)?;
    }
    Ok(())
}

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        match name.to_uppercase().as_str() {
            "GET" => vector_ops::op_get(self), "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self), "REMOVE" => vector_ops::op_remove(self),
            "LENGTH" => vector_ops::op_length(self), "TAKE" => vector_ops::op_take(self),
            "DROP" => vector_ops::op_drop_vector(self), "SPLIT" => vector_ops::op_split(self),
            "DUP" => vector_ops::op_dup_workspace(self), "SWAP" => vector_ops::op_swap_workspace(self),
            "ROT" => vector_ops::op_rot_workspace(self), "CONCAT" => vector_ops::op_concat(self),
            "REVERSE" => vector_ops::op_reverse(self),
            "+" => arithmetic::op_add(self), "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self), "/" => arithmetic::op_div(self),
            "=" => arithmetic::op_eq(self), "<" => arithmetic::op_lt(self),
            "<=" => arithmetic::op_le(self), ">" => arithmetic::op_gt(self),
            ">=" => arithmetic::op_ge(self), "AND" => arithmetic::op_and(self),
            "OR" => arithmetic::op_or(self), "NOT" => arithmetic::op_not(self),
            ":" => {
            Err(AjisaiError::from("':' can only be used in conditional expressions. Usage: condition : action"))
        },
            "PRINT" => io::op_print(self), 
            "SOUND" => audio::op_sound(self),  // 新規追加
            "DEF" => {
                if self.workspace.len() >= 2 {
                    control::op_def(self)
                } else {
                    Err(AjisaiError::from("DEF requires definition and name on workspace. Usage: : ... ; 'WORD_NAME' DEF"))
                }
            },
            "DEL" => {
                if !self.workspace.is_empty() {
                    control::op_del(self)
                } else {
                    Err(AjisaiError::from("DEL requires a word name on workspace. Usage: 'WORD_NAME' DEL"))
                }
            },
            "GOTO" => flow_control::op_goto(self),
            "?" => control::op_lookup(self),
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
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
        if let Some(def) = self.dictionary.get(&name.to_uppercase()) {
            if let Some(original_source) = &def.original_source {
                return Some(original_source.clone());
            }
            
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
                    
                    if line.repeat_count != 1 {
                        result.push_str(&format!("{}x ", line.repeat_count));
                    }
                    if line.delay_ms > 0 {
                        if line.delay_ms >= 1000 && line.delay_ms % 1000 == 0 {
                            result.push_str(&format!("{}s ", line.delay_ms / 1000));
                        } else {
                            result.push_str(&format!("{}ms ", line.delay_ms));
                        }
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
            Token::Modifier(s) => s.clone(),
            Token::LineBreak => "\n".to_string(),
            _ => "".to_string(),
        }
    }

    pub fn get_output(&mut self) -> String { std::mem::take(&mut self.output_buffer) }
    pub fn get_workspace(&self) -> &Workspace { &self.workspace }
    pub fn set_workspace(&mut self, workspace: Workspace) { self.workspace = workspace; }
    
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
        self.workspace.clear(); 
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
                ValueType::Boolean(b) => *b, ValueType::Nil => false,
                ValueType::Number(n) => !n.numerator.is_zero(),
                ValueType::String(s) => !s.is_empty(),
                ValueType::Vector(inner_v, _) => !inner_v.is_empty(),
                _ => true,
            }
        }
    }
    false
}
