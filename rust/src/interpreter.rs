// in: masamoto1982/ajisai/Ajisai-e4b1951ad8cf96ca24706236c945342f04c7cf22/rust/src/interpreter.rs

use std::collections::{HashMap, HashSet};
use crate::types::{Value, ValueType, Fraction, Stack, Register, TableData, Token};
use crate::tokenizer::*;
use crate::builtins;

pub struct Interpreter {
    stack: Stack,
    register: Register,
    dictionary: HashMap<String, WordDefinition>,
    dependencies: HashMap<String, HashSet<String>>, // word -> それを使用しているワードのセット
    // ステップ実行用の状態
    step_tokens: Vec<Token>,
    step_position: usize,
    step_mode: bool,
    // 出力バッファ
    output_buffer: String,
    // データベース関連
    current_table: Option<String>,
    tables: HashMap<String, TableData>,
}

#[derive(Clone)]
pub struct WordDefinition {
    pub tokens: Vec<Token>,
    pub is_builtin: bool,
    pub description: Option<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            register: None,
            dictionary: HashMap::new(),
            dependencies: HashMap::new(),
            step_tokens: Vec::new(),
            step_position: 0,
            step_mode: false,
            output_buffer: String::new(),
            current_table: None,
            tables: HashMap::new(),
        };
        
        builtins::register_builtins(&mut interpreter.dictionary);
        
        interpreter
    }
    
    pub fn execute(&mut self, code: &str) -> Result<(), String> {
        let tokens = tokenize(code)?;
        self.execute_tokens_with_context(&tokens)?;
        Ok(())
    }

    pub fn get_output(&mut self) -> String {
        let output = self.output_buffer.clone();
        self.output_buffer.clear();
        output
    }
    
    fn append_output(&mut self, text: &str) {
        self.output_buffer.push_str(text);
    }

    pub fn init_step_execution(&mut self, code: &str) -> Result<(), String> {
        self.step_tokens = tokenize(code)?;
        self.step_position = 0;
        self.step_mode = true;
        Ok(())
    }

    pub fn execute_step(&mut self) -> Result<bool, String> {
        if !self.step_mode || self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            return Ok(false);
        }

        let token = self.step_tokens[self.step_position].clone();
        self.step_position += 1;

        match self.execute_single_token(&token) {
            Ok(_) => Ok(self.step_position < self.step_tokens.len()),
            Err(e) => {
                self.step_mode = false;
                Err(e)
            }
        }
    }

    pub fn get_step_info(&self) -> Option<(usize, usize)> {
        if self.step_mode {
            Some((self.step_position, self.step_tokens.len()))
        } else {
            None
        }
    }

    fn execute_single_token(&mut self, token: &Token) -> Result<(), String> {
        match token {
            Token::Description(_) => Ok(()),
            Token::Number(num, den) => {
                self.stack.push(Value {
                    val_type: ValueType::Number(Fraction::new(*num, *den)),
                });
                Ok(())
            },
            Token::String(s) => {
                self.stack.push(Value {
                    val_type: ValueType::String(s.clone()),
                });
                Ok(())
            },
            Token::Boolean(b) => {
                self.stack.push(Value {
                    val_type: ValueType::Boolean(*b),
                });
                Ok(())
            },
            Token::Nil => {
                self.stack.push(Value {
                    val_type: ValueType::Nil,
                });
                Ok(())
            },
            Token::VectorStart => {
                let (vector_values, _) = self.collect_vector_as_data(&self.step_tokens[self.step_position - 1..])?;
                self.stack.push(Value {
                    val_type: ValueType::Vector(vector_values),
                });
                Ok(())
            },
            Token::BlockStart => {
                let (block_tokens, _) = self.collect_block_tokens(&self.step_tokens, self.step_position - 1)?;
                self.stack.push(Value {
                    val_type: ValueType::Quotation(block_tokens),
                });
                Ok(())
            }
            Token::Symbol(name) => {
                if let Some(def) = self.dictionary.get(name).cloned() {
                    if def.is_builtin {
                        self.execute_builtin(name)?;
                    } else {
                        self.execute_tokens_with_context(&def.tokens)?;
                    }
                } else {
                    return Err(format!("Unknown word: {}", name));
                }
                Ok(())
            },
            Token::VectorEnd | Token::BlockEnd => Err("Unexpected closing delimiter found.".to_string()),
        }
    }

    fn collect_vector_as_data(&self, tokens: &[Token]) -> Result<(Vec<Value>, usize), String> {
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
                Token::Number(num, den) => values.push(Value { val_type: ValueType::Number(Fraction::new(*num, *den)) }),
                Token::String(s) => values.push(Value { val_type: ValueType::String(s.clone()) }),
                Token::Boolean(b) => values.push(Value { val_type: ValueType::Boolean(*b) }),
                Token::Nil => values.push(Value { val_type: ValueType::Nil }),
                Token::Symbol(s) => values.push(Value { val_type: ValueType::Symbol(s.clone()) }),
                _ => {}
            }
            i += 1;
        }

        Err("Unclosed vector".to_string())
    }

    fn collect_block_tokens(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Token>, usize), String> {
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

        Err("Unclosed block".to_string())
    }
    
    fn execute_tokens_with_context(&mut self, tokens: &[Token]) -> Result<(), String> {
        let mut i = 0;
        let mut pending_description: Option<String> = None;

        while i < tokens.len() {
            let token = &tokens[i];
            match token {
                Token::Description(text) => {
                    pending_description = Some(text.clone());
                },
                Token::Number(num, den) => {
                    self.stack.push(Value {
                        val_type: ValueType::Number(Fraction::new(*num, *den)),
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
                    i = next_index -1;
                },
                Token::Symbol(name) => {
                    if name == "DEF" {
                        self.op_def(pending_description.take())?;
                    } else if let Some(def) = self.dictionary.get(name).cloned() {
                        if def.is_builtin {
                            self.execute_builtin(name)?;
                        } else {
                            self.execute_custom_word(name, &def.tokens)?;
                        }
                    } else {
                        return Err(format!("Unknown word: {}", name));
                    }
                },
                Token::VectorEnd | Token::BlockEnd => return Err("Unexpected closing delimiter found.".to_string()),
            }
            i += 1;
        }
        Ok(())
    }

    fn execute_custom_word(&mut self, _name: &str, tokens: &[Token]) -> Result<(), String> {
    self.execute_tokens_with_context(tokens)
}

    fn op_def(&mut self, description: Option<String>) -> Result<(), String> {
        if self.stack.len() < 2 {
            return Err("Stack underflow for DEF".to_string());
        }
    
        let name_val = self.stack.pop().unwrap();
        let body_val = self.stack.pop().unwrap();
    
        match (&name_val.val_type, &body_val.val_type) {
            (ValueType::String(name), ValueType::Quotation(body_tokens)) => {
                let name = name.to_uppercase();
    
                if let Some(existing) = self.dictionary.get(&name) {
                    if existing.is_builtin {
                        return Err(format!("Cannot redefine builtin word: {}", name));
                    }
                }
    
                if self.dictionary.contains_key(&name) {
                    if let Some(dependents) = self.dependencies.get(&name) {
                        if !dependents.is_empty() {
                            let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                            return Err(format!(
                                "Cannot redefine '{}' because it is used by: {}",
                                name,
                                dependent_list.join(", ")
                            ));
                        }
                    }
                }

                let mut new_dependencies = HashSet::new();
                for token in body_tokens {
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
                    tokens: body_tokens.clone(),
                    is_builtin: false,
                    description,
                });
    
                Ok(())
            }
            _ => Err("Type error: DEF requires a quotation { ... } and a string".to_string()),
        }
    }
        
    fn execute_builtin(&mut self, name: &str) -> Result<(), String> {
        match name {
            "DUP" => self.op_dup(),
            "DROP" => self.op_drop(),
            "SWAP" => self.op_swap(),
            "OVER" => self.op_over(),
            "ROT" => self.op_rot(),
            "NIP" => self.op_nip(),
            ">R" => self.op_to_r(),
            "R>" => self.op_from_r(),
            "R@" => self.op_r_fetch(),
            "DEF" => self.op_def(None),
            "IF" => self.op_if(),
            "LENGTH" => self.op_length(),
            "HEAD" => self.op_head(),
            "TAIL" => self.op_tail(),
            "CONS" => self.op_cons(),
            "APPEND" => self.op_append(),
            "REVERSE" => self.op_reverse(),
            "NTH" => self.op_nth(),
            "UNCONS" => self.op_uncons(),
            "EMPTY?" => self.op_empty(),
            "DEL" => self.op_del(),
            "NOT" => self.op_not(),
            "AND" => self.op_and(),
            "OR" => self.op_or(),
            "+" => self.op_add(),
            "-" => self.op_sub(),
            "*" => self.op_mul(),
            "/" => self.op_div(),
            ">" => self.op_gt(),
            ">=" => self.op_ge(),
            "=" => self.op_eq(),
            "<" => self.op_lt(),
            "<=" => self.op_le(),
            "NIL?" => self.op_nil_check(),
            "NOT-NIL?" => self.op_not_nil_check(),
            "KNOWN?" => self.op_not_nil_check(),
            "DEFAULT" => self.op_default(),
            "TABLE" => self.op_table(),
            "TABLE-CREATE" => self.op_table_create(),
            "FILTER" => self.op_filter(),
            "PROJECT" => self.op_project(),
            "INSERT" => self.op_insert(),
            "UPDATE" => self.op_update(),
            "DELETE" => self.op_delete(),
            "TABLES" => self.op_tables(),
            "TABLES-INFO" => self.op_tables_info(),
            "TABLE-INFO" => self.op_table_info(),
            "TABLE-SIZE" => self.op_table_size(),
            "SAVE-DB" => self.op_save_db(),
            "LOAD-DB" => self.op_load_db(),
            "MATCH?" => self.op_match(),
            "WILDCARD" => self.op_wildcard(),
            "." => self.op_dot(),
            "PRINT" => self.op_print(),
            "CR" => self.op_cr(),
            "SPACE" => self.op_space(),
            "SPACES" => self.op_spaces(),
            "EMIT" => self.op_emit(),
            _ => Err(format!("Unknown builtin: {}", name)),
        }
    }
    
    fn op_dup(&mut self) -> Result<(), String> {
        if let Some(top) = self.stack.last() {
            self.stack.push(top.clone());
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_drop(&mut self) -> Result<(), String> {
        if self.stack.pop().is_none() {
            Err("Stack underflow".to_string())
        } else {
            Ok(())
        }
    }
    
    fn op_swap(&mut self) -> Result<(), String> {
        let len = self.stack.len();
        if len < 2 {
            Err("Stack underflow".to_string())
        } else {
            self.stack.swap(len - 1, len - 2);
            Ok(())
        }
    }
    
    fn op_over(&mut self) -> Result<(), String> {
        let len = self.stack.len();
        if len < 2 {
            Err("Stack underflow".to_string())
        } else {
            let item = self.stack[len - 2].clone();
            self.stack.push(item);
            Ok(())
        }
    }
    
    fn op_rot(&mut self) -> Result<(), String> {
        let len = self.stack.len();
        if len < 3 {
            Err("Stack underflow".to_string())
        } else {
            let third = self.stack.remove(len - 3);
            self.stack.push(third);
            Ok(())
        }
    }
    
    fn op_nip(&mut self) -> Result<(), String> {
        let len = self.stack.len();
        if len < 2 {
            Err("Stack underflow".to_string())
        } else {
            self.stack.remove(len - 2);
            Ok(())
        }
    }
    
    fn op_to_r(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            self.register = Some(val);
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_from_r(&mut self) -> Result<(), String> {
        if let Some(val) = self.register.take() {
            self.stack.push(val);
            Ok(())
        } else {
            Err("Register is empty".to_string())
        }
    }
    
    fn op_r_fetch(&mut self) -> Result<(), String> {
        if let Some(val) = &self.register {
            self.stack.push(val.clone());
            Ok(())
        } else {
            Err("Register is empty".to_string())
        }
    }

    fn op_add(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Number(n1.add(n2)) });
                Ok(())
            },
            _ => Err("Type error in +".to_string()),
        }
    }
    
    fn op_sub(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Number(n1.sub(n2)) });
                Ok(())
            },
            _ => Err("Type error in -".to_string()),
        }
    }
    
    fn op_mul(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Number(n1.mul(n2)) });
                Ok(())
            },
            _ => Err("Type error in *".to_string()),
        }
    }
    
    fn op_div(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Number(n1.div(n2)) });
                Ok(())
            },
            _ => Err("Type error in /".to_string()),
        }
    }
    
    fn op_gt(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(n1.gt(n2)) });
                Ok(())
            },
            _ => Err("Type error in >".to_string()),
        }
    }
    
    fn op_ge(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(n1.ge(n2)) });
                Ok(())
            },
            _ => Err("Type error in >=".to_string()),
        }
    }
    
    fn op_eq(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        self.stack.push(Value { val_type: ValueType::Boolean(a == b) });
        Ok(())
    }
    
    fn op_lt(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(n1.lt(n2)) });
                Ok(())
            },
            _ => Err("Type error in <".to_string()),
        }
    }
    
    fn op_le(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();
        
        match (&a.val_type, &b.val_type) {
            (ValueType::Number(n1), ValueType::Number(n2)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(n1.le(n2)) });
                Ok(())
            },
            _ => Err("Type error in <=".to_string()),
        }
    }

    fn op_and(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b_val = self.stack.pop().unwrap();
        let a_val = self.stack.pop().unwrap();
        
        match (a_val.val_type, b_val.val_type) {
            (ValueType::Boolean(a), ValueType::Boolean(b)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(a && b) });
            },
            (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(false) });
            },
            (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) | (ValueType::Nil, ValueType::Nil) => {
                self.stack.push(Value { val_type: ValueType::Nil });
            }
            _ => return Err("Type error in AND".to_string()),
        }
        Ok(())
    }

    fn op_or(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let b_val = self.stack.pop().unwrap();
        let a_val = self.stack.pop().unwrap();
        
        match (a_val.val_type, b_val.val_type) {
            (ValueType::Boolean(a), ValueType::Boolean(b)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(a || b) });
            },
            (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) => {
                self.stack.push(Value { val_type: ValueType::Boolean(true) });
            },
            (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) | (ValueType::Nil, ValueType::Nil) => {
                self.stack.push(Value { val_type: ValueType::Nil });
            }
            _ => return Err("Type error in OR".to_string()),
        }
        Ok(())
    }

    fn op_nil_check(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            self.stack.push(Value { val_type: ValueType::Boolean(matches!(val.val_type, ValueType::Nil)) });
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_not_nil_check(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            self.stack.push(Value { val_type: ValueType::Boolean(!matches!(val.val_type, ValueType::Nil)) });
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_default(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let default_val = self.stack.pop().unwrap();
        let val = self.stack.pop().unwrap();
        
        if matches!(val.val_type, ValueType::Nil) {
            self.stack.push(default_val)
        } else {
            self.stack.push(val)
        }
        Ok(())
    }

    fn op_table(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::String(name) = val.val_type {
                if let Some(table) = self.tables.get(&name) {
                    let table_vec = Value {
                        val_type: ValueType::Vector(table.records.iter().map(|rec| Value { val_type: ValueType::Vector(rec.clone()) }).collect())
                    };
                    self.stack.push(table_vec);
                    self.current_table = Some(name);
                    Ok(())
                } else {
                    Err(format!("Table '{}' not found", name))
                }
            } else {
                Err("TABLE requires a string".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_table_create(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let name_val = self.stack.pop().unwrap();
        let schema_val = self.stack.pop().unwrap();

        if let (ValueType::String(name), ValueType::Vector(schema_vec)) = (name_val.val_type, schema_val.val_type) {
            let schema: Vec<String> = schema_vec.into_iter().filter_map(|v| {
                if let ValueType::String(s) = v.val_type { Some(s) } else { None }
            }).collect();
            
            let table_data = TableData { schema, records: Vec::new() };
            self.tables.insert(name, table_data);
            Ok(())
        } else {
            Err("TABLE-CREATE requires a schema vector and a table name string".to_string())
        }
    }

    fn op_filter(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let filter_quotation = self.stack.pop().unwrap();
        let table_val = self.stack.pop().unwrap();

        if let (ValueType::Vector(records), ValueType::Quotation(filter_tokens)) = (table_val.val_type, filter_quotation.val_type) {
            let mut filtered_records = Vec::new();
            for record_val in records {
                if let ValueType::Vector(_) = &record_val.val_type {
                    self.stack.push(record_val.clone());
                    self.execute_tokens_with_context(&filter_tokens)?;
                    if let Some(result) = self.stack.pop() {
                        if let ValueType::Boolean(true) = result.val_type {
                            filtered_records.push(record_val);
                        }
                    }
                }
            }
            self.stack.push(Value { val_type: ValueType::Vector(filtered_records) });
            Ok(())
        } else {
            Err("FILTER requires a table (vector of vectors) and a filter quotation".to_string())
        }
    }

    fn op_project(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let columns_val = self.stack.pop().unwrap();
        let table_val = self.stack.pop().unwrap();
        
        // This is a simplified implementation. A real one would need schema info.
        if let (ValueType::Vector(_), ValueType::Vector(_)) = (table_val.val_type, columns_val.val_type) {
            // For now, just push back the table.
            self.stack.push(table_val);
            Ok(())
        } else {
            Err("PROJECT requires a table and a columns vector".to_string())
        }
    }

    fn op_insert(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let table_name_val = self.stack.pop().unwrap();
        let record_val = self.stack.pop().unwrap();

        if let (ValueType::String(name), ValueType::Vector(fields)) = (table_name_val.val_type, record_val.val_type) {
            if let Some(table) = self.tables.get_mut(&name) {
                table.records.push(fields);
                Ok(())
            } else {
                Err(format!("Table '{}' not found", name))
            }
        } else {
            Err("INSERT requires a record vector and a table name string".to_string())
        }
    }

    fn op_update(&mut self) -> Result<(), String> { Ok(()) }
    fn op_delete(&mut self) -> Result<(), String> { Ok(()) }

    fn op_tables(&mut self) -> Result<(), String> {
        if let Some(pattern_val) = self.stack.pop() {
            if let ValueType::String(pattern) = pattern_val.val_type {
                let table_names: Vec<Value> = self.tables.keys()
                    .filter(|name| self.wildcard_match(name, &pattern))
                    .map(|name| Value { val_type: ValueType::String(name.clone()) })
                    .collect();
                self.stack.push(Value { val_type: ValueType::Vector(table_names) });
                Ok(())
            } else {
                Err("TABLES requires a pattern string".to_string())
            }
        } else {
            Err("Stack underflow for TABLES".to_string())
        }
    }

    fn op_tables_info(&mut self) -> Result<(), String> {
        let mut output = String::new();
        if self.tables.is_empty() {
            output.push_str("No tables found.\n");
        } else {
            output.push_str(&format!("Total tables: {}\n", self.tables.len()));
            for (name, table) in &self.tables {
                output.push_str(&format!(
                    "Table '{}': {} records, schema: {:?}\n",
                    name, table.records.len(), table.schema
                ));
            }
        }
        self.append_output(&output);
        Ok(())
    }

    fn op_table_info(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::String(name) = val.val_type {
                let mut output = String::new();
                if let Some(table) = self.tables.get(&name) {
                    output.push_str(&format!(
                        "Table '{}'\n  Schema: {:?}\n  Records: {}\n",
                        name, table.schema, table.records.len()
                    ));
                    for (i, record) in table.records.iter().take(3).enumerate() {
                        output.push_str(&format!("  Record {}: {:?}\n", i, record));
                    }
                } else {
                    output.push_str(&format!("Table '{}' not found\n", name));
                }
                self.append_output(&output);
                Ok(())
            } else {
                Err("TABLE-INFO requires a string".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_table_size(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::String(name) = val.val_type {
                let size = self.tables.get(&name).map_or(0, |t| t.records.len() as i64);
                self.stack.push(Value { val_type: ValueType::Number(Fraction::new(size, 1)) });
                Ok(())
            } else {
                Err("TABLE-SIZE requires a string".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_save_db(&mut self) -> Result<(), String> {
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-save-db").map_err(|_| "Failed to create save event")?;
            window.dispatch_event(&event).map_err(|_| "Failed to dispatch save event")?;
        }
        Ok(())
    }

    fn op_load_db(&mut self) -> Result<(), String> {
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-load-db").map_err(|_| "Failed to create load event")?;
            window.dispatch_event(&event).map_err(|_| "Failed to dispatch load event")?;
        }
        Ok(())
    }

    fn op_match(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let pattern = self.stack.pop().unwrap();
        let value = self.stack.pop().unwrap();
        
        if let (ValueType::String(s), ValueType::String(p)) = (value.val_type, pattern.val_type) {
            self.stack.push(Value { val_type: ValueType::Boolean(self.wildcard_match(&s, &p)) });
            Ok(())
        } else {
            Err("Type error in MATCH?".to_string())
        }
    }

    fn op_wildcard(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn wildcard_match(&self, text: &str, pattern: &str) -> bool {
        // A simple wildcard implementation
        let mut p = pattern.replace("*", ".*").replace("?", ".");
        p = format!("^{}$", p);
        // This is not ideal as it doesn't exist in std, would need a regex crate
        text.contains(&pattern.replace("*", "").replace("?",""))
    }
    
    fn op_length(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Vector(v) => {
                    self.stack.push(Value { val_type: ValueType::Number(Fraction::new(v.len() as i64, 1)) });
                    Ok(())
                },
                _ => Err("Type error: LENGTH requires a vector".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_head(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::Vector(v) = val.val_type {
                if let Some(first) = v.first() {
                    self.stack.push(first.clone());
                    Ok(())
                } else {
                    Err("HEAD on empty vector".to_string())
                }
            } else {
                Err("Type error: HEAD requires a vector".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_tail(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::Vector(v) = val.val_type {
                if v.is_empty() {
                    Err("TAIL on empty vector".to_string())
                } else {
                    self.stack.push(Value { val_type: ValueType::Vector(v[1..].to_vec()) });
                    Ok(())
                }
            } else {
                Err("Type error: TAIL requires a vector".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_cons(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let vec_val = self.stack.pop().unwrap();
        let elem = self.stack.pop().unwrap();
        if let ValueType::Vector(mut v) = vec_val.val_type {
            v.insert(0, elem);
            self.stack.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        } else {
            Err("Type error: CONS requires an element and a vector".to_string())
        }
    }

    fn op_append(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let elem = self.stack.pop().unwrap();
        let vec_val = self.stack.pop().unwrap();
        if let ValueType::Vector(mut v) = vec_val.val_type {
            v.push(elem);
            self.stack.push(Value { val_type: ValueType::Vector(v) });
            Ok(())
        } else {
            Err("Type error: APPEND requires a vector and an element".to_string())
        }
    }
    
    fn op_reverse(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::Vector(mut v) = val.val_type {
                v.reverse();
                self.stack.push(Value { val_type: ValueType::Vector(v) });
                Ok(())
            } else {
                Err("Type error: REVERSE requires a vector".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }

    fn op_nth(&mut self) -> Result<(), String> {
        if self.stack.len() < 2 { return Err("Stack underflow".to_string()); }
        let vec_val = self.stack.pop().unwrap();
        let index_val = self.stack.pop().unwrap();
        if let (ValueType::Number(n), ValueType::Vector(v)) = (index_val.val_type, vec_val.val_type) {
            if n.denominator != 1 { return Err("NTH requires an integer index".to_string()); }
            let index = if n.numerator < 0 { v.len() as i64 + n.numerator } else { n.numerator };
            if index >= 0 && (index as usize) < v.len() {
                self.stack.push(v[index as usize].clone());
                Ok(())
            } else {
                Err("Index out of bounds".to_string())
            }
        } else {
            Err("Type error: NTH requires a number and a vector".to_string())
        }
    }
    
    fn op_uncons(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::Vector(v) = val.val_type {
                if v.is_empty() { return Err("UNCONS on empty vector".to_string()); }
                self.stack.push(v[0].clone());
                self.stack.push(Value { val_type: ValueType::Vector(v[1..].to_vec()) });
                Ok(())
            } else {
                Err("Type error: UNCONS requires a vector".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
        
    fn op_empty(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::Vector(v) = val.val_type {
                self.stack.push(Value { val_type: ValueType::Boolean(v.is_empty()) });
                Ok(())
            } else {
                Err("Type error: EMPTY? requires a vector".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_if(&mut self) -> Result<(), String> {
        if self.stack.len() < 3 {
            return Err("Stack underflow for IF".to_string());
        }
        
        let else_branch = self.stack.pop().unwrap();
        let then_branch = self.stack.pop().unwrap();
        let condition = self.stack.pop().unwrap();

        if let (ValueType::Quotation(then_tokens), ValueType::Quotation(else_tokens)) = (then_branch.val_type, else_branch.val_type) {
            let tokens_to_execute = match condition.val_type {
                ValueType::Boolean(true) => then_tokens,
                ValueType::Boolean(false) => else_tokens,
                ValueType::Nil => else_tokens,
                _ => return Err("IF condition must be a boolean or nil".to_string()),
            };
            self.execute_tokens_with_context(&tokens_to_execute)?;
            Ok(())
        } else {
            Err("IF requires two quotations".to_string())
        }
    }

    fn op_not(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            match val.val_type {
                ValueType::Boolean(b) => {
                    self.stack.push(Value { val_type: ValueType::Boolean(!b) });
                    Ok(())
                },
                ValueType::Nil => {
                    self.stack.push(Value { val_type: ValueType::Nil });
                    Ok(())
                },
                _ => Err("Type error: NOT requires a boolean or nil".to_string()),
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_del(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::String(name) = val.val_type {
                self.dictionary.remove(&name.to_uppercase());
                Ok(())
            } else {
                Err("Type error: DEL requires a string".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_dot(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            self.append_output(&format!("{} ", val));
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_print(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.last() {
            self.append_output(&format!("{} ", val));
            Ok(())
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_cr(&mut self) -> Result<(), String> {
        self.append_output("\n");
        Ok(())
    }
    
    fn op_space(&mut self) -> Result<(), String> {
        self.append_output(" ");
        Ok(())
    }
    
    fn op_spaces(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::Number(n) = val.val_type {
                if n.denominator == 1 && n.numerator >= 0 {
                    self.append_output(&" ".repeat(n.numerator as usize));
                    Ok(())
                } else {
                    Err("SPACES requires a non-negative integer".to_string())
                }
            } else {
                Err("Type error: SPACES requires a number".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
    fn op_emit(&mut self) -> Result<(), String> {
        if let Some(val) = self.stack.pop() {
            if let ValueType::Number(n) = val.val_type {
                if n.denominator == 1 && n.numerator >= 0 && n.numerator <= 255 {
                    self.append_output(&(n.numerator as u8 as char).to_string());
                    Ok(())
                } else {
                    Err("EMIT requires an integer between 0 and 255".to_string())
                }
            } else {
                Err("Type error: EMIT requires a number".to_string())
            }
        } else {
            Err("Stack underflow".to_string())
        }
    }
    
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
   
    pub fn save_table(&mut self, name: String, schema: Vec<String>, records: Vec<Vec<Value>>) {
        self.tables.insert(name, TableData { schema, records });
    }
   
    pub fn load_table(&self, name: &str) -> Option<(Vec<String>, Vec<Vec<Value>>)> {
        self.tables.get(name).map(|t| (t.schema.clone(), t.records.clone()))
    }
   
    pub fn get_all_tables(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }
   
    pub fn set_stack(&mut self, stack: Stack) {
        self.stack = stack;
    }
   
    pub fn set_register(&mut self, register: Register) {
        self.register = register;
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
            Token::Description(d) => format!("({})", d),
        }
    }
}
