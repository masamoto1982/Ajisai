// rust/src/interpreter/execute.rs

use crate::types::{Value, ValueType, Token};
use crate::tokenizer::tokenize;
use super::{Interpreter, error::{AjisaiError, Result}};
use super::{stack_ops::*, arithmetic::*, vector_ops::*, control::*, io::*, register_ops::*};

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
            match &tokens[0] {
                // リテラル値は直接実行
                Token::Number(_, _) | Token::String(_) | Token::Boolean(_) | Token::Nil => {
                    return self.execute_tokens_with_context(tokens);
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
                        return self.execute_tokens_with_context(tokens);
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
        
        // 複数トークンの式は必ず自動定義（Ajisaiのコンセプト）
        self.define_from_tokens(tokens)
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
