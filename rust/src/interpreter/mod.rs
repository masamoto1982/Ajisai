// rust/src/interpreter/mod.rs - 統一構文対応版

pub mod vector_ops;
pub mod arithmetic;
pub mod control;
pub mod io;
pub mod error;

use std::collections::{HashMap, HashSet};
use crate::types::{Workspace, Token, Value, ValueType, ExecutionLine, RepeatControl, TimeControl, Fraction};
use crate::parser::{Parser, Expression, RepeatSpec, TimeSpec};
use self::error::{Result, AjisaiError};
use web_sys::console;
use wasm_bindgen::JsValue;
use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive};

pub struct Interpreter {
    pub(crate) workspace: Workspace,
    pub(crate) dictionary: HashMap<String, InterpreterWordDefinition>,
    pub(crate) dependencies: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) debug_buffer: String,
    pub(crate) call_stack: Vec<String>,
    loop_index_stack: Vec<usize>,  // LOOP_INDEX管理用
}

#[derive(Clone)]
pub struct InterpreterWordDefinition {
    pub lines: Vec<ExecutionLine>,
    pub is_builtin: bool,
    pub description: Option<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        console::log_1(&JsValue::from_str("=== INTERPRETER NEW (unified) ==="));
        let mut interpreter = Interpreter {
            workspace: Vec::new(),
            dictionary: HashMap::new(),
            dependencies: HashMap::new(),
            output_buffer: String::new(),
            debug_buffer: String::new(),
            call_stack: Vec::new(),
            loop_index_stack: Vec::new(),
        };
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        console::log_1(&JsValue::from_str(&format!("Interpreter created with {} builtin words", interpreter.dictionary.len())));
        interpreter
    }

    pub fn execute(&mut self, code: &str) -> Result<()> {
        console::log_1(&JsValue::from_str(&format!("=== EXECUTE (unified) ===\nCode: '{}'", code)));
        
        // トークナイズ
        let custom_words: HashSet<String> = self.dictionary.keys().cloned().collect();
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_words)?;
        
        if tokens.is_empty() {
            return Ok(());
        }
        
        // S式としてパース
        let mut parser = Parser::new(tokens);
        let expressions = parser.parse()?;
        
        // 各式を評価
        for expr in &expressions {
            self.eval_expression(expr)?;
        }
        
        Ok(())
    }
    
    fn eval_expression(&mut self, expr: &Expression) -> Result<Value> {
        console::log_1(&JsValue::from_str(&format!("=== eval_expression ===\n{:?}", expr)));
        
        match expr {
            // 基本値
            Expression::Number(n) => {
                let val = Value { val_type: ValueType::Number(n.clone()) };
                let wrapped = Value { val_type: ValueType::Vector(vec![val]) };
                self.workspace.push(wrapped.clone());
                Ok(wrapped)
            },
            Expression::String(s) => {
                let val = Value { val_type: ValueType::String(s.clone()) };
                let wrapped = Value { val_type: ValueType::Vector(vec![val]) };
                self.workspace.push(wrapped.clone());
                Ok(wrapped)
            },
            Expression::Boolean(b) => {
                let val = Value { val_type: ValueType::Boolean(*b) };
                let wrapped = Value { val_type: ValueType::Vector(vec![val]) };
                self.workspace.push(wrapped.clone());
                Ok(wrapped)
            },
            Expression::Nil => {
                let val = Value { val_type: ValueType::Nil };
                let wrapped = Value { val_type: ValueType::Vector(vec![val]) };
                self.workspace.push(wrapped.clone());
                Ok(wrapped)
            },
            
            // S式（アクションファースト）
            Expression::SExpression { action, args } => {
                self.eval_s_expression(action, args)
            },
            
            // データVector
            Expression::Vector(elements) => {
                let mut values = Vec::new();
                for elem in elements {
                    let value = self.expression_to_value(elem)?;
                    values.push(value);
                }
                let wrapped = Value { val_type: ValueType::Vector(values) };
                self.workspace.push(wrapped.clone());
                Ok(wrapped)
            },
            
            // 制御構造
            Expression::Repeat { spec, body } => {
                self.eval_repeat(spec, body)
            },
            Expression::Delay { spec, body } => {
                self.eval_delay(spec, body)
            },
            Expression::If { condition, then_branch, else_branch } => {
                self.eval_if(condition, then_branch, else_branch.as_deref())
            },
            
            // シンボル（カスタムワード呼び出し）
            Expression::Symbol(name) => {
                if self.dictionary.contains_key(name) {
                    self.execute_word(name)?;
                } else {
                    return Err(AjisaiError::UnknownWord(name.clone()));
                }
                Ok(Value { val_type: ValueType::Nil })
            },
            
            // LINE構文は直接評価できない
            Expression::Line { .. } => {
                Err(AjisaiError::from("LINE should only appear in word definitions"))
            },
        }
    }
    
    fn eval_s_expression(&mut self, action: &Expression, args: &[Expression]) -> Result<Value> {
        console::log_1(&JsValue::from_str(&format!("=== eval_s_expression ===\nAction: {:?}", action)));
        
        match action {
            Expression::Symbol(name) => match name.as_str() {
                // 算術演算
                "+" => self.eval_add(args),
                "-" => self.eval_sub(args),
                "*" => self.eval_mul(args),
                "/" => self.eval_div(args),
                
                // 比較演算
                ">" => self.eval_gt(args),
                ">=" => self.eval_ge(args),
                "=" => self.eval_eq(args),
                "<" => self.eval_lt(args),
                "<=" => self.eval_le(args),
                
                // 論理演算
                "AND" => self.eval_and(args),
                "OR" => self.eval_or(args),
                "NOT" => self.eval_not(args),
                
                // Vector操作（位置指定）
                "GET" => self.eval_get(args),
                "INSERT" => self.eval_insert(args),
                "REPLACE" => self.eval_replace(args),
                "REMOVE" => self.eval_remove(args),
                
                // Vector操作（量指定）
                "LENGTH" => self.eval_length(args),
                "TAKE" => self.eval_take(args),
                "DROP" => self.eval_drop(args),
                "CONCAT" => self.eval_concat(args),
                "REVERSE" => self.eval_reverse(args),
                
                // ワークスペース操作
                "DUP" => {
                    vector_ops::op_dup_workspace(self)?;
                    Ok(Value { val_type: ValueType::Nil })
                },
                "SWAP" => {
                    vector_ops::op_swap_workspace(self)?;
                    Ok(Value { val_type: ValueType::Nil })
                },
                "ROT" => {
                    vector_ops::op_rot_workspace(self)?;
                    Ok(Value { val_type: ValueType::Nil })
                },
                
                // 特殊操作
                "HEAD" => {
                    let top = self.workspace.last()
                        .ok_or(AjisaiError::WorkspaceUnderflow)?
                        .clone();
                    Ok(top)
                },
                "WORKSPACE_SIZE" => {
                    let size = self.workspace.len();
                    let val = Value {
                        val_type: ValueType::Number(Fraction::new(
                            BigInt::from(size),
                            BigInt::one()
                        ))
                    };
                    Ok(Value { val_type: ValueType::Vector(vec![val]) })
                },
                "LOOP_INDEX" => {
                    let index = self.loop_index_stack.last().copied().unwrap_or(0);
                    let val = Value {
                        val_type: ValueType::Number(Fraction::new(
                            BigInt::from(index),
                            BigInt::one()
                        ))
                    };
                    Ok(Value { val_type: ValueType::Vector(vec![val]) })
                },
                
                // I/O
                "PRINT" => self.eval_print(args),
                
                // ワード管理
                "DEF" => self.eval_def(args),
                "DEL" => self.eval_del(args),
                "RESET" => {
                    self.execute_reset()?;
                    Ok(Value { val_type: ValueType::Nil })
                },
                
                // カスタムワード呼び出し
                _ => {
                    if self.dictionary.contains_key(name) {
                        // 引数を評価してワークスペースに配置
                        for arg in args {
                            self.eval_expression(arg)?;
                        }
                        // カスタムワードを実行
                        self.execute_word(name)?;
                        Ok(Value { val_type: ValueType::Nil })
                    } else {
                        Err(AjisaiError::UnknownWord(name.clone()))
                    }
                }
            },
            _ => Err(AjisaiError::from("Action must be a symbol")),
        }
    }
    
    // 算術演算の評価
    fn eval_add(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("+ requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.add_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    fn eval_sub(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("- requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.sub_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    fn eval_mul(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("* requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.mul_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    fn eval_div(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("/ requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.div_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    // 比較演算
    fn eval_gt(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("> requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.gt_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    fn eval_lt(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("< requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.lt_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    fn eval_ge(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from(">= requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.ge_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    fn eval_le(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("<= requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.le_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    fn eval_eq(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("= requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = Value {
            val_type: ValueType::Vector(vec![Value {
                val_type: ValueType::Boolean(a == b)
            }])
        };
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    // 論理演算
    fn eval_and(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("AND requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.and_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    fn eval_or(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("OR requires exactly 2 arguments"));
        }
        let a = self.expression_to_value(&args[0])?;
        let b = self.expression_to_value(&args[1])?;
        let result = self.or_values(&a, &b)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    fn eval_not(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 1 {
            return Err(AjisaiError::from("NOT requires exactly 1 argument"));
        }
        let a = self.expression_to_value(&args[0])?;
        let result = self.not_value(&a)?;
        self.workspace.push(result.clone());
        Ok(result)
    }
    
    // Vector操作
    fn eval_get(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("GET requires vector and index"));
        }
        let vector = self.expression_to_value(&args[0])?;
        let index = self.expression_to_value(&args[1])?;
        
        // ワークスペースに一時的にpush
        self.workspace.push(vector);
        self.workspace.push(index);
        vector_ops::op_get(self)?;
        
        self.workspace.last().cloned().ok_or(AjisaiError::from("GET failed"))
    }
    
    fn eval_length(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 1 {
            return Err(AjisaiError::from("LENGTH requires exactly 1 argument"));
        }
        let vector = self.expression_to_value(&args[0])?;
        self.workspace.push(vector);
        vector_ops::op_length(self)?;
        self.workspace.last().cloned().ok_or(AjisaiError::from("LENGTH failed"))
    }
    
    fn eval_insert(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 3 {
            return Err(AjisaiError::from("INSERT requires vector, index, and element"));
        }
        let vector = self.expression_to_value(&args[0])?;
        let index = self.expression_to_value(&args[1])?;
        let element = self.expression_to_value(&args[2])?;
        
        self.workspace.push(vector);
        self.workspace.push(index);
        self.workspace.push(element);
        vector_ops::op_insert(self)?;
        
        self.workspace.last().cloned().ok_or(AjisaiError::from("INSERT failed"))
    }
    
    fn eval_replace(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 3 {
            return Err(AjisaiError::from("REPLACE requires vector, index, and element"));
        }
        let vector = self.expression_to_value(&args[0])?;
        let index = self.expression_to_value(&args[1])?;
        let element = self.expression_to_value(&args[2])?;
        
        self.workspace.push(vector);
        self.workspace.push(index);
        self.workspace.push(element);
        vector_ops::op_replace(self)?;
        
        self.workspace.last().cloned().ok_or(AjisaiError::from("REPLACE failed"))
    }
    
    fn eval_remove(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("REMOVE requires vector and index"));
        }
        let vector = self.expression_to_value(&args[0])?;
        let index = self.expression_to_value(&args[1])?;
        
        self.workspace.push(vector);
        self.workspace.push(index);
        vector_ops::op_remove(self)?;
        
        self.workspace.last().cloned().ok_or(AjisaiError::from("REMOVE failed"))
    }
    
    fn eval_take(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("TAKE requires vector and count"));
        }
        let vector = self.expression_to_value(&args[0])?;
        let count = self.expression_to_value(&args[1])?;
        
        self.workspace.push(vector);
        self.workspace.push(count);
        vector_ops::op_take(self)?;
        
        self.workspace.last().cloned().ok_or(AjisaiError::from("TAKE failed"))
    }
    
    fn eval_drop(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("DROP requires vector and count"));
        }
        let vector = self.expression_to_value(&args[0])?;
        let count = self.expression_to_value(&args[1])?;
        
        self.workspace.push(vector);
        self.workspace.push(count);
        vector_ops::op_drop_vector(self)?;
        
        self.workspace.last().cloned().ok_or(AjisaiError::from("DROP failed"))
    }
    
    fn eval_concat(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 2 {
            return Err(AjisaiError::from("CONCAT requires exactly 2 vectors"));
        }
        let v1 = self.expression_to_value(&args[0])?;
        let v2 = self.expression_to_value(&args[1])?;
        
        self.workspace.push(v1);
        self.workspace.push(v2);
        vector_ops::op_concat(self)?;
        
        self.workspace.last().cloned().ok_or(AjisaiError::from("CONCAT failed"))
    }
    
    fn eval_reverse(&mut self, args: &[Expression]) -> Result<Value> {
        if args.len() != 1 {
            return Err(AjisaiError::from("REVERSE requires exactly 1 vector"));
        }
        let vector = self.expression_to_value(&args[0])?;
        
        self.workspace.push(vector);
        vector_ops::op_reverse(self)?;
        
        self.workspace.last().cloned().ok_or(AjisaiError::from("REVERSE failed"))
    }
    
    // I/O
    fn eval_print(&mut self, args: &[Expression]) -> Result<Value> {
        for arg in args {
            let value = self.expression_to_value(arg)?;
            self.output_buffer.push_str(&format!("{} ", value));
        }
        Ok(Value { val_type: ValueType::Nil })
    }
    
    // 制御構造
    fn eval_repeat(&mut self, spec: &RepeatSpec, body: &Expression) -> Result<Value> {
        console::log_1(&JsValue::from_str(&format!("=== eval_repeat: {:?} ===", spec)));
        
        match spec {
            RepeatSpec::Times(n) => {
                for i in 0..*n {
                    self.loop_index_stack.push(i as usize);
                    self.eval_expression(body)?;
                    self.loop_index_stack.pop();
                }
            },
            RepeatSpec::Forever => {
                // 簡略実装（実際には非同期が必要）
                return Err(AjisaiError::from("FOREVER not yet implemented"));
            },
            RepeatSpec::Once => {
                self.eval_expression(body)?;
            },
            RepeatSpec::While(_) => {
                return Err(AjisaiError::from("WHILE not yet implemented"));
            },
        }
        
        Ok(Value { val_type: ValueType::Nil })
    }
    
    fn eval_delay(&mut self, spec: &TimeSpec, body: &Expression) -> Result<Value> {
        console::log_1(&JsValue::from_str(&format!("=== eval_delay: {:?} ===", spec)));
        
        // 簡略実装（実際にはタイマー実装が必要）
        match spec {
            TimeSpec::Immediate => {
                self.eval_expression(body)?;
            },
            _ => {
                console::log_1(&JsValue::from_str("Delay not fully implemented, executing immediately"));
                self.eval_expression(body)?;
            }
        }
        
        Ok(Value { val_type: ValueType::Nil })
    }
    
    fn eval_if(&mut self, condition: &Expression, then_branch: &Expression, else_branch: Option<&Expression>) -> Result<Value> {
        console::log_1(&JsValue::from_str("=== eval_if ==="));
        
        let cond_value = self.expression_to_value(condition)?;
        let is_true = self.is_truthy(&cond_value);
        
        if is_true {
            self.eval_expression(then_branch)
        } else if let Some(else_expr) = else_branch {
            self.eval_expression(else_expr)
        } else {
            Ok(Value { val_type: ValueType::Nil })
        }
    }
    
    // ワード管理
    fn eval_def(&mut self, args: &[Expression]) -> Result<Value> {
        console::log_1(&JsValue::from_str("=== eval_def (GOTO-style) ==="));
        
        if args.len() != 2 {
            return Err(AjisaiError::from("DEF requires word name and lines"));
        }
        
        let word_name = match &args[0] {
            Expression::Symbol(name) => name.clone(),
            _ => return Err(AjisaiError::from("Word name must be a symbol")),
        };
        
        let lines = match &args[1] {
            Expression::Vector(lines) => {
                let mut execution_lines = Vec::new();
                for line_expr in lines {
                    if let Expression::Line { repeat, timing, condition, action } = line_expr {
                        let exec_line = self.expression_line_to_execution_line(repeat, timing, condition.as_deref(), action)?;
                        execution_lines.push(exec_line);
                    } else {
                        return Err(AjisaiError::from("Invalid LINE in word definition"));
                    }
                }
                execution_lines
            },
            _ => return Err(AjisaiError::from("Lines must be a vector")),
        };
        
        // 依存関係の記録
        let mut deps = HashSet::new();
        for line in &lines {
            for action in &line.action {
                if let ValueType::Symbol(s) = &action.val_type {
                    if self.dictionary.contains_key(s) && !self.dictionary[s].is_builtin {
                        deps.insert(s.clone());
                    }
                }
            }
        }
        
        for dep in &deps {
            self.dependencies.entry(dep.clone())
                .or_insert_with(HashSet::new)
                .insert(word_name.clone());
        }
        
        // ワード登録
        self.dictionary.insert(word_name.clone(), InterpreterWordDefinition {
            lines,
            is_builtin: false,
            description: None,
        });
        
        self.output_buffer.push_str(&format!("Defined word: {}\n", word_name));
        Ok(Value { val_type: ValueType::Nil })
    }
    
    fn eval_del(&mut self, args: &[Expression]) -> Result<Value> {
        console::log_1(&JsValue::from_str("=== eval_del ==="));
        
        if args.len() != 1 {
            return Err(AjisaiError::from("DEL requires exactly one word name"));
        }
        
        let word_name = match &args[0] {
            Expression::Symbol(name) => name.clone(),
            _ => return Err(AjisaiError::from("Word name must be a symbol")),
        };
        
        control::op_del_word(self, &word_name)?;
        Ok(Value { val_type: ValueType::Nil })
    }
    
    // ヘルパー関数
    fn expression_to_value(&mut self, expr: &Expression) -> Result<Value> {
        match expr {
            Expression::Number(n) => {
                Ok(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Number(n.clone()) }]) })
            },
            Expression::String(s) => {
                Ok(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::String(s.clone()) }]) })
            },
            Expression::Boolean(b) => {
                Ok(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Boolean(*b) }]) })
            },
            Expression::Nil => {
                Ok(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Nil }]) })
            },
            Expression::Vector(elements) => {
                let mut values = Vec::new();
                for elem in elements {
                    let value = self.expression_to_value(elem)?;
                    // ネストを解除
                    if let ValueType::Vector(v) = value.val_type {
                        if v.len() == 1 {
                            values.push(v[0].clone());
                        } else {
                            values.push(Value { val_type: ValueType::Vector(v) });
                        }
                    }
                }
                Ok(Value { val_type: ValueType::Vector(values) })
            },
            Expression::Symbol(s) => {
                // HEADやWORKSPACE_SIZEなどの特殊シンボル
                match s.as_str() {
                    "HEAD" => {
                        self.workspace.last().cloned()
                            .ok_or(AjisaiError::WorkspaceUnderflow)
                    },
                    "WORKSPACE_SIZE" => {
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Number(Fraction::new(
                                    BigInt::from(self.workspace.len()),
                                    BigInt::one()
                                ))
                            }])
                        })
                    },
                    "LOOP_INDEX" => {
                        let index = self.loop_index_stack.last().copied().unwrap_or(0);
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Number(Fraction::new(
                                    BigInt::from(index),
                                    BigInt::one()
                                ))
                            }])
                        })
                    },
                    _ => {
                        // 通常のシンボル
                        Ok(Value { val_type: ValueType::Vector(vec![Value { val_type: ValueType::Symbol(s.clone()) }]) })
                    }
                }
            },
            _ => {
                // その他の式は評価してから値を取得
                self.eval_expression(expr)?;
                self.workspace.pop().ok_or(AjisaiError::from("Failed to get expression value"))
            }
        }
    }
    
    fn expression_line_to_execution_line(&mut self, repeat: &RepeatSpec, timing: &TimeSpec, 
                                         condition: Option<&Expression>, action: &Expression) -> Result<ExecutionLine> {
        let repeat_ctrl = match repeat {
            RepeatSpec::Times(n) => RepeatControl::Times(*n),
            RepeatSpec::Forever => RepeatControl::Forever,
            RepeatSpec::Once => RepeatControl::Once,
            _ => RepeatControl::Once,
        };
        
        let time_ctrl = match timing {
            TimeSpec::Seconds(s) => TimeControl::Seconds(*s),
            TimeSpec::Milliseconds(ms) => TimeControl::Milliseconds(*ms),
            TimeSpec::Immediate => TimeControl::Immediate,
        };
        
        let cond = if let Some(c) = condition {
            Some(vec![self.expression_to_value(c)?])
        } else {
            None
        };
        
        let action_value = self.expression_to_value(action)?;
        let action_vec = if let ValueType::Vector(v) = action_value.val_type {
            v
        } else {
            vec![action_value]
        };
        
        Ok(ExecutionLine {
            repeat: repeat_ctrl,
            timing: time_ctrl,
            condition: cond,
            action: action_vec,
        })
    }
    
    fn is_truthy(&self, value: &Value) -> bool {
        match &value.val_type {
            ValueType::Vector(v) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::Boolean(b) => *b,
                    ValueType::Nil => false,
                    ValueType::Number(n) => !n.numerator.is_zero(),
                    _ => true,
                }
            },
            ValueType::Vector(v) => !v.is_empty(),
            _ => false,
        }
    }
    
    // 値演算のヘルパー
    fn add_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (&a.val_type, &b.val_type) {
            (ValueType::Vector(av), ValueType::Vector(bv)) 
                if av.len() == 1 && bv.len() == 1 => {
                match (&av[0].val_type, &bv[0].val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Number(n1.add(n2))
                            }])
                        })
                    },
                    _ => Err(AjisaiError::type_error("number", "other type")),
                }
            },
            _ => Err(AjisaiError::from("Arguments must be single-element vectors")),
        }
    }
    
    fn sub_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (&a.val_type, &b.val_type) {
            (ValueType::Vector(av), ValueType::Vector(bv)) 
                if av.len() == 1 && bv.len() == 1 => {
                match (&av[0].val_type, &bv[0].val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Number(n1.sub(n2))
                            }])
                        })
                    },
                    _ => Err(AjisaiError::type_error("number", "other type")),
                }
            },
            _ => Err(AjisaiError::from("Arguments must be single-element vectors")),
        }
    }
    
    fn mul_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (&a.val_type, &b.val_type) {
            (ValueType::Vector(av), ValueType::Vector(bv)) 
                if av.len() == 1 && bv.len() == 1 => {
                match (&av[0].val_type, &bv[0].val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Number(n1.mul(n2))
                            }])
                        })
                    },
                    _ => Err(AjisaiError::type_error("number", "other type")),
                }
            },
            _ => Err(AjisaiError::from("Arguments must be single-element vectors")),
        }
    }
    
    fn div_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (&a.val_type, &b.val_type) {
            (ValueType::Vector(av), ValueType::Vector(bv)) 
                if av.len() == 1 && bv.len() == 1 => {
                match (&av[0].val_type, &bv[0].val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        if n2.numerator.is_zero() {
                            return Err(AjisaiError::DivisionByZero);
                        }
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Number(n1.div(n2))
                            }])
                        })
                    },
                    _ => Err(AjisaiError::type_error("number", "other type")),
                }
            },
            _ => Err(AjisaiError::from("Arguments must be single-element vectors")),
        }
    }
    
    fn gt_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (&a.val_type, &b.val_type) {
            (ValueType::Vector(av), ValueType::Vector(bv)) 
                if av.len() == 1 && bv.len() == 1 => {
                match (&av[0].val_type, &bv[0].val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Boolean(n1.gt(n2))
                            }])
                        })
                    },
                    _ => Err(AjisaiError::type_error("number", "other type")),
                }
            },
            _ => Err(AjisaiError::from("Arguments must be single-element vectors")),
        }
    }
    
    fn lt_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (&a.val_type, &b.val_type) {
            (ValueType::Vector(av), ValueType::Vector(bv)) 
                if av.len() == 1 && bv.len() == 1 => {
                match (&av[0].val_type, &bv[0].val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Boolean(n1.lt(n2))
                            }])
                        })
                    },
                    _ => Err(AjisaiError::type_error("number", "other type")),
                }
            },
            _ => Err(AjisaiError::from("Arguments must be single-element vectors")),
        }
    }
    
    fn ge_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (&a.val_type, &b.val_type) {
            (ValueType::Vector(av), ValueType::Vector(bv)) 
                if av.len() == 1 && bv.len() == 1 => {
                match (&av[0].val_type, &bv[0].val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Boolean(n1.ge(n2))
                            }])
                        })
                    },
                    _ => Err(AjisaiError::type_error("number", "other type")),
                }
            },
            _ => Err(AjisaiError::from("Arguments must be single-element vectors")),
        }
    }
    
    fn le_values(&self, a: &Value, b: &Value) -> Result<Value> {
        match (&a.val_type, &b.val_type) {
            (ValueType::Vector(av), ValueType::Vector(bv)) 
                if av.len() == 1 && bv.len() == 1 => {
                match (&av[0].val_type, &bv[0].val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value {
                            val_type: ValueType::Vector(vec![Value {
                                val_type: ValueType::Boolean(n1.le(n2))
                            }])
                        })
                    },
                    _ => Err(AjisaiError::type_error("number", "other type")),
                }
            },
            _ => Err(AjisaiError::from("Arguments must be single-element vectors")),
        }
    }
    
    fn and_values(&self, a: &Value, b: &Value) -> Result<Value> {
        let a_truthy = self.is_truthy(a);
        let b_truthy = self.is_truthy(b);
        Ok(Value {
            val_type: ValueType::Vector(vec![Value {
                val_type: ValueType::Boolean(a_truthy && b_truthy)
            }])
        })
    }
    
    fn or_values(&self, a: &Value, b: &Value) -> Result<Value> {
        let a_truthy = self.is_truthy(a);
        let b_truthy = self.is_truthy(b);
        Ok(Value {
            val_type: ValueType::Vector(vec![Value {
                val_type: ValueType::Boolean(a_truthy || b_truthy)
            }])
        })
    }
    
    fn not_value(&self, a: &Value) -> Result<Value> {
        let truthy = self.is_truthy(a);
        Ok(Value {
            val_type: ValueType::Vector(vec![Value {
                val_type: ValueType::Boolean(!truthy)
            }])
        })
    }
    
    // カスタムワード実行
    fn execute_word(&mut self, name: &str) -> Result<()> {
        console::log_1(&JsValue::from_str(&format!("execute_word: {}", name)));
        
        if let Some(def) = self.dictionary.get(name).cloned() {
            if def.is_builtin {
                self.execute_builtin(name)
            } else {
                self.call_stack.push(name.to_string());
                let result = self.execute_custom_word(name, &def);
                self.call_stack.pop();
                result.map_err(|e| e.with_context(&self.call_stack))
            }
        } else {
            Err(AjisaiError::UnknownWord(name.to_string()))
        }
    }
    
    fn execute_custom_word(&mut self, name: &str, def: &InterpreterWordDefinition) -> Result<()> {
        console::log_1(&JsValue::from_str(&format!("=== execute_custom_word: {} ===", name)));
        
        for (line_idx, line) in def.lines.iter().enumerate() {
            console::log_1(&JsValue::from_str(&format!("Executing line {}", line_idx)));
            
            // 反復制御
            let repeat_count = match &line.repeat {
                RepeatControl::Times(n) => *n,
                RepeatControl::Once => 1,
                _ => 1,
            };
            
            for i in 0..repeat_count {
                self.loop_index_stack.push(i as usize);
                
                // 条件チェック
                let should_execute = if let Some(ref condition) = line.condition {
                    let cond_val = condition.first().unwrap_or(&Value { val_type: ValueType::Nil });
                    self.is_truthy(cond_val)
                } else {
                    true
                };
                
                if should_execute {
                    // アクション実行
                    for action in &line.action {
                        match &action.val_type {
                            ValueType::Symbol(sym_name) => {
                                self.execute_word(sym_name)?;
                            },
                            _ => {
                                self.workspace.push(action.clone());
                            }
                        }
                    }
                }
                
                self.loop_index_stack.pop();
            }
        }
        
        Ok(())
    }
    
    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        console::log_1(&JsValue::from_str(&format!("execute_builtin: {}", name)));
        
        match name {
            "GET" => vector_ops::op_get(self),
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self),
            "REMOVE" => vector_ops::op_remove(self),
            "LENGTH" => vector_ops::op_length(self),
            "TAKE" => vector_ops::op_take(self),
            "DROP" => vector_ops::op_drop_vector(self),
            "REPEAT" => vector_ops::op_repeat(self),
            "SPLIT" => vector_ops::op_split(self),
            "DUP" => vector_ops::op_dup_workspace(self),
            "SWAP" => vector_ops::op_swap_workspace(self),
            "ROT" => vector_ops::op_rot_workspace(self),
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
            "PRINT" => io::op_print(self),
            "CR" => io::op_cr(self),
            "SPACE" => io::op_space(self),
            "SPACES" => io::op_spaces(self),
            "EMIT" => io::op_emit(self),
            "RESET" => self.execute_reset(),
            _ => Err(AjisaiError::UnknownBuiltin(name.to_string())),
        }
    }

    pub fn execute_reset(&mut self) -> Result<()> {
        if let Some(window) = web_sys::window() {
            let event = web_sys::CustomEvent::new("ajisai-reset").map_err(|_| AjisaiError::from("Failed to create reset event"))?;
            window.dispatch_event(&event).map_err(|_| AjisaiError::from("Failed to dispatch reset event"))?;
        }
        self.workspace.clear();
        self.dictionary.clear();
        self.dependencies.clear();
        self.output_buffer.clear();
        self.call_stack.clear();
        crate::builtins::register_builtins(&mut self.dictionary);
        Ok(())
    }

    // 旧API互換性のため
    pub fn execute_tokens(&mut self, _tokens: &[Token]) -> Result<()> {
        Err(AjisaiError::from("Legacy execute_tokens not supported in unified mode"))
    }

    pub fn execute_single_token(&mut self, token: &Token) -> Result<String> {
        // 単一トークンをS式として実行
        let expr = match token {
            Token::Number(s) => Expression::Number(Fraction::from_str(s)?),
            Token::String(s) => Expression::String(s.clone()),
            Token::Boolean(b) => Expression::Boolean(*b),
            Token::Nil => Expression::Nil,
            Token::Symbol(s) => Expression::Symbol(s.clone()),
            _ => return Err(AjisaiError::from("Cannot execute this token")),
        };
        
        self.eval_expression(&expr)?;
        Ok(format!("Executed: {:?}", token))
    }

    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        self.dictionary.get(name).and_then(|def| {
            if def.is_builtin { return None; }
            Some(format!("Word definition for {}", name))
        })
    }

    pub fn restore_custom_word(&mut self, name: String, _tokens: Vec<Token>, description: Option<String>) -> Result<()> {
        self.dictionary.insert(name.to_uppercase(), InterpreterWordDefinition {
            lines: vec![],
            is_builtin: false,
            description,
        });
        Ok(())
    }

    pub fn get_output(&mut self) -> String { std::mem::take(&mut self.output_buffer) }
    pub fn get_debug_output(&mut self) -> String { std::mem::take(&mut self.debug_buffer) }
    pub fn get_worksp
