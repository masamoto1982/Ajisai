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
                                self.execute_custom_word_with_iteration(name, &def.tokens)?;
                                // 連鎖削除
                                self.delete_temporary_word_cascade(name);
                                return Ok(());
                            } else if !def.is_builtin {
                                // 永続的なカスタムワードの場合、暗黙の反復を試みる
                                return self.execute_custom_word_with_iteration(name, &def.tokens);
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
                    if let Some(def) = self.dictionary.get(name).cloned() {
                        if def.is_builtin {
                            self.execute_builtin(name)?;
                        } else {
                            self.execute_custom_word_with_iteration(name, &def.tokens)?;
                        }
                    } else {
                        return Err(AjisaiError::UnknownWord(name.clone()));
                    }
                },
                Token::VectorEnd => {
                    return Err(AjisaiError::from("Unexpected closing delimiter found."));
                },
            }
            i += 1;
        }
        Ok(())
    }

    // 暗黙の反復機能を持つカスタムワード実行
    pub(super) fn execute_custom_word_with_iteration(&mut self, name: &str, tokens: &[Token]) -> Result<()> {
        // アリティ（引数の数）を推定
        let arity = self.estimate_word_arity(name, tokens);
        
        // スタックから必要な数の引数を確認
        if self.stack.len() < arity {
            // 引数が足りない場合は通常の実行を試みる
            return self.execute_custom_word_simple(name, tokens);
        }
        
        // 引数の中にベクトルがあるかチェック
        let mut has_vector = false;
        let mut vector_positions = Vec::new();
        let mut vector_lengths = Vec::new();
        
        for i in 0..arity {
            let idx = self.stack.len() - arity + i;
            if let ValueType::Vector(v) = &self.stack[idx].val_type {
                has_vector = true;
                vector_positions.push(i);
                vector_lengths.push(v.len());
            }
        }
        
        if !has_vector {
            // ベクトルがない場合は通常の実行
            return self.execute_custom_word_simple(name, tokens);
        }
        
        // すべてのベクトルが同じ長さかチェック
        if !vector_lengths.is_empty() {
            let first_len = vector_lengths[0];
            if !vector_lengths.iter().all(|&len| len == first_len) {
                // 長さが異なる場合はエラー
                return Err(AjisaiError::from("Vector length mismatch in implicit iteration"));
            }
        }
        
        // ベクトルがある場合は暗黙の反復を適用
        self.apply_implicit_iteration(name, tokens, arity, vector_positions)
    }
    
    fn apply_implicit_iteration(
        &mut self, 
        name: &str, 
        tokens: &[Token], 
        arity: usize,
        vector_positions: Vec<usize>
    ) -> Result<()> {
        // 引数を取得
        let mut args = Vec::new();
        for _ in 0..arity {
            args.push(self.stack.pop().unwrap());
        }
        args.reverse();
        
        // ベクトルの長さを取得（すべて同じ長さのはず）
        let vec_len = args.iter()
            .enumerate()
            .filter_map(|(idx, arg)| {
                if vector_positions.contains(&idx) {
                    if let ValueType::Vector(v) = &arg.val_type {
                        Some(v.len())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .next()
            .unwrap_or(0);
        
        // 各インデックスに対して処理
        let mut results = Vec::new();
        for i in 0..vec_len {
            // 引数を準備
            for (arg_idx, arg) in args.iter().enumerate() {
                if vector_positions.contains(&arg_idx) {
                    // ベクトル引数から要素を取得
                    if let ValueType::Vector(v) = &arg.val_type {
                        self.stack.push(v[i].clone());
                    }
                } else {
                    // スカラー引数はそのまま使用（ブロードキャスト）
                    self.stack.push(arg.clone());
                }
            }
            
            // ワードを実行
            self.call_stack.push(name.to_string());
            let exec_result = self.execute_tokens_with_context(tokens);
            self.call_stack.pop();
            
            exec_result.map_err(|e| e.with_context(&self.call_stack))?;
            
            // 結果を収集（スタックトップから取得）
            if let Some(result) = self.stack.pop() {
                results.push(result);
            } else {
                // 結果がない場合はnilを追加
                results.push(Value { val_type: ValueType::Nil });
            }
        }
        
        // 結果をスタックにプッシュ
        self.stack.push(Value {
            val_type: ValueType::Vector(results)
        });
        
        Ok(())
    }
    
    // 通常のカスタムワード実行（暗黙の反復なし）
    fn execute_custom_word_simple(&mut self, name: &str, tokens: &[Token]) -> Result<()> {
        self.call_stack.push(name.to_string());
        let result = self.execute_tokens_with_context(tokens);
        self.call_stack.pop();
        result.map_err(|e| e.with_context(&self.call_stack))
    }
    
    fn estimate_word_arity(&self, _name: &str, tokens: &[Token]) -> usize {
        // トークンをシミュレーション実行してアリティを推定
        let mut dummy_stack: Vec<()> = Vec::new();
        let mut consumed = 0usize;
        
        for token in tokens {
            match token {
                Token::Number(_, _) | Token::String(_) | Token::Boolean(_) | Token::Nil => {
                    dummy_stack.push(());
                },
                Token::Symbol(sym) => {
                    // 既知の演算子のアリティ
                    let arity = match sym.as_str() {
                        "+" | "-" | "*" | "/" | ">" | ">=" | "=" | "<" | "<=" | "AND" | "OR" => {
                            consumed = consumed.saturating_add(2);
                            if dummy_stack.len() >= 2 {
                                dummy_stack.pop();
                                dummy_stack.pop();
                                dummy_stack.push(());
                            }
                            2
                        },
                        "DUP" | "NOT" | "NIL?" | "NOT-NIL?" => {
                            consumed = consumed.saturating_add(1);
                            if !dummy_stack.is_empty() {
                                dummy_stack.push(());
                            }
                            1
                        },
                        "DROP" => {
                            consumed = consumed.saturating_add(1);
                            dummy_stack.pop();
                            1
                        },
                        "SWAP" | "OVER" => {
                            consumed = consumed.saturating_add(2);
                            2
                        },
                        "ROT" | "?" => {
                            consumed = consumed.saturating_add(3);
                            3
                        },
                        _ => {
                            // カスタムワードの場合、デフォルトで1引数と仮定
                            consumed = consumed.saturating_add(1);
                            1
                        }
                    };
                    
                    // 最大値を記録
                    consumed = consumed.max(arity);
                },
                _ => {}
            }
        }
        
        // スタックに追加された要素の数と消費された要素の数から推定
        consumed.saturating_sub(dummy_stack.len()).max(1)
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
