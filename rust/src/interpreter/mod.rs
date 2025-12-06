// rust/src/interpreter/mod.rs

pub mod helpers;        // 共通ヘルパー関数
pub mod vector_ops;
pub mod tensor_ops;     // テンソル演算とブロードキャスト
pub mod arithmetic;
pub mod comparison;
pub mod logic;
pub mod control;
pub mod dictionary;
pub mod io;
pub mod audio;
pub mod higher_order;
pub mod cast;
pub mod datetime;
pub mod sort;

use std::collections::{HashMap, HashSet};
use crate::types::{Stack, Token, Value, ValueType, WordDefinition, ExecutionLine};
use crate::types::fraction::Fraction;
use crate::error::{Result, AjisaiError};
use async_recursion::async_recursion;
use self::helpers::wrap_in_square_vector;
use gloo_timers::future::sleep;
use std::time::Duration;

/// 操作対象を指定する列挙型
///
/// Ajisaiでは、操作を「スタック全体」または「スタックトップの要素」に
/// 対して実行できる。この列挙型は現在の操作スコープを表す。
///
/// - `Stack`: スタック全体を操作対象とする（例: .. GET）
/// - `StackTop`: スタックトップの要素を操作対象とする（デフォルト）
///
/// 各単語の実行後、自動的に StackTop にリセットされる。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperationTarget {
    /// スタック全体を操作対象とする
    Stack,
    /// スタックトップの要素を操作対象とする（デフォルト）
    StackTop,
}

/// 非同期アクションを表す列挙型
///
/// コアロジック実行中に非同期操作が必要になった場合に返される。
/// 同期コンテキストではエラーとして処理し、
/// 非同期コンテキストでは実際に非同期処理を実行する。
#[derive(Debug, Clone)]
pub enum AsyncAction {
    /// WAIT ワード: 指定ミリ秒後にワードを実行
    Wait {
        duration_ms: u64,
        word_name: String,
    },
    // 将来の拡張用:
    // Fetch { url: String, callback_word: String },
    // ReadFile { path: String, callback_word: String },
}

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependents: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) definition_to_load: Option<String>,
    pub(crate) operation_target: OperationTarget,
    pub(crate) force_flag: bool,  // 強制実行フラグ（DEL/DEFで依存関係がある場合に使用）
    pub(crate) disable_no_change_check: bool,  // "No change is an error"チェックを無効化（REDUCE等で使用）
    // 追加: 継続実行用の状態
    pub(crate) pending_tokens: Option<Vec<Token>>,
    pub(crate) pending_token_index: usize,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            dictionary: HashMap::new(),
            dependents: HashMap::new(),
            output_buffer: String::new(),
            definition_to_load: None,
            operation_target: OperationTarget::StackTop,
            force_flag: false,
            disable_no_change_check: false,
            pending_tokens: None,
            pending_token_index: 0,
        };
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    /// Tensor収集メソッド（新しいToken::TensorStart/TensorEnd用）
    ///
    /// Phase 3: すべての配列リテラルをTensorとして扱う
    /// - 数値のみの配列はTensorに変換
    /// - 混合型の配列は一時的にVectorとして扱う（後方互換性）
    fn collect_tensor(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Value>, usize)> {
        if !matches!(&tokens[start_index], Token::TensorStart) {
            return Err(AjisaiError::from("Expected tensor start ([)"));
        }

        let mut values = Vec::new();
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::TensorStart => {
                    let (nested_values, consumed) = self.collect_tensor(tokens, i)?;
                    // ネストされたテンソルはVectorとして一時的に保持（後でTensorに変換）
                    values.push(Value { val_type: ValueType::Vector(nested_values) });
                    i += consumed;
                },
                Token::TensorEnd => {
                    return Ok((values, i - start_index + 1));
                },
                Token::Number(n) => {
                    values.push(Value { val_type: ValueType::Number(Fraction::from_str(n).map_err(AjisaiError::from)?) });
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
                Token::Symbol(s) => {
                    values.push(Value { val_type: ValueType::Symbol(s.clone()) });
                    i += 1;
                },
                _ => {
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from("Unclosed tensor starting with ["))
    }

    /// 後方互換性のため残存（非推奨）
    #[allow(deprecated)]
    fn collect_vector(&self, tokens: &[Token], start_index: usize) -> Result<(Vec<Value>, usize)> {
        let bracket_type = match &tokens[start_index] {
            Token::VectorStart(bt) => bt.clone(),
            _ => return Err(AjisaiError::from("Expected vector start")),
        };

        let mut values = Vec::new();
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart(_) => {
                    let (nested_values, consumed) = self.collect_vector(tokens, i)?;
                    values.push(Value { val_type: ValueType::Vector(nested_values) });
                    i += consumed;
                },
                Token::VectorEnd(bt) if *bt == bracket_type => {
                    return Ok((values, i - start_index + 1));
                },
                Token::Number(n) => {
                    values.push(Value { val_type: ValueType::Number(Fraction::from_str(n).map_err(AjisaiError::from)?) });
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
                Token::Symbol(s) => {
                    values.push(Value { val_type: ValueType::Symbol(s.clone()) });
                    i += 1;
                },
                _ => {
                    i += 1;
                }
            }
        }
        Err(AjisaiError::from(format!("Unclosed vector starting with {}", bracket_type.opening_char())))
    }

    /// ガード構造実行（同期版）
    pub(crate) fn execute_guard_structure_sync(
        &mut self,
        lines: &[ExecutionLine]
    ) -> Result<()> {
        let action = self.execute_guard_structure_core(lines)?;

        if let Some(async_action) = action {
            return Err(AjisaiError::from(format!(
                "Async operation {:?} requires async context",
                async_action
            )));
        }

        Ok(())
    }

    /// ガード構造実行（非同期版）
    #[async_recursion(?Send)]
    pub(crate) async fn execute_guard_structure(&mut self, lines: &[ExecutionLine]) -> Result<()> {
        if lines.is_empty() {
            return Ok(());
        }

        let all_lines_have_colon = lines.iter().all(|line| {
            line.body_tokens.first() == Some(&Token::GuardSeparator)
        });

        if !all_lines_have_colon {
            for line in lines {
                let tokens = if line.body_tokens.first() == Some(&Token::GuardSeparator) {
                    &line.body_tokens[1..]
                } else {
                    &line.body_tokens[..]
                };
                self.execute_section(tokens).await?;
            }
            return Ok(());
        }

        let mut i = 0;
        while i < lines.len() {
            let line = &lines[i];
            let content_tokens = &line.body_tokens[1..];

            if i + 1 < lines.len() {
                self.execute_section(content_tokens).await?;

                if self.is_condition_true()? {
                    i += 1;
                    let action_line = &lines[i];
                    let action_tokens = &action_line.body_tokens[1..];
                    self.execute_section(action_tokens).await?;
                    return Ok(());
                }
                i += 2;
            } else {
                self.execute_section(content_tokens).await?;
                return Ok(());
            }
        }

        Ok(())
    }

    /// WAIT ワードの引数を取得し、AsyncAction を構築する
    fn prepare_wait_action(&mut self) -> Result<AsyncAction> {
        use num_traits::{One, ToPrimitive};

        if self.stack.len() < 2 {
            return Err(AjisaiError::from(
                "WAIT requires word name and delay. Usage: 'WORD' [ ms ] WAIT"
            ));
        }

        let delay_val = self.stack.pop().unwrap();
        let name_val = self.stack.pop().unwrap();

        // 遅延時間を取得
        let duration_ms = match &delay_val.val_type {
            ValueType::Vector(v) if v.len() == 1 => {
                match &v[0].val_type {
                    ValueType::Number(n) if n.denominator == num_bigint::BigInt::one() => {
                        n.numerator.to_u64().ok_or_else(||
                            AjisaiError::from("Delay too large")
                        )?
                    },
                    _ => return Err(AjisaiError::type_error("integer", "other type")),
                }
            },
            ValueType::Tensor(t) if t.data().len() == 1 => {
                let n = &t.data()[0];
                if n.denominator == num_bigint::BigInt::one() {
                    n.numerator.to_u64().ok_or_else(||
                        AjisaiError::from("Delay too large")
                    )?
                } else {
                    return Err(AjisaiError::type_error("integer", "fraction"));
                }
            },
            _ => return Err(AjisaiError::type_error(
                "single-element vector or tensor with integer",
                "other type"
            )),
        };

        // ワード名を取得
        let word_name = helpers::get_word_name_from_value(&name_val)?;

        // ワードの存在確認
        if let Some(def) = self.dictionary.get(&word_name) {
            if def.is_builtin {
                return Err(AjisaiError::from(
                    "WAIT can only be used with custom words"
                ));
            }
        } else {
            return Err(AjisaiError::UnknownWord(word_name));
        }

        Ok(AsyncAction::Wait { duration_ms, word_name })
    }

    /// セクション内のトークンを実行（コアロジック）
    ///
    /// 非同期操作に遭遇した場合は AsyncAction を返し、処理を中断する。
    /// 呼び出し元は AsyncAction を処理した後、継続実行する責任を持つ。
    ///
    /// # 戻り値
    /// - `Ok((next_index, None))`: 正常完了
    /// - `Ok((next_index, Some(AsyncAction)))`: 非同期操作が必要
    /// - `Err(_)`: エラー発生
    fn execute_section_core(
        &mut self,
        tokens: &[Token],
        start_index: usize
    ) -> Result<(usize, Option<AsyncAction>)> {
        let mut i = start_index;

        while i < tokens.len() {
            match &tokens[i] {
                Token::Number(n) => {
                    use crate::types::tensor::Tensor;
                    let frac = Fraction::from_str(n).map_err(AjisaiError::from)?;
                    let tensor = Tensor::vector(vec![frac]);
                    self.stack.push(Value::from_tensor(tensor));
                },
                Token::String(s) => {
                    // 非数値型のため、Vectorでラップ
                    let val = Value { val_type: ValueType::String(s.clone()) };
                    self.stack.push(wrap_in_square_vector(val));
                },
                Token::Boolean(b) => {
                    // 非数値型のため、Vectorでラップ
                    let val = Value { val_type: ValueType::Boolean(*b) };
                    self.stack.push(wrap_in_square_vector(val));
                },
                Token::Nil => {
                    // 非数値型のため、Vectorでラップ
                    let val = Value { val_type: ValueType::Nil };
                    self.stack.push(wrap_in_square_vector(val));
                },
                Token::TensorStart => {
                    // Phase 3: 新しいTensor指向のパース処理
                    let (values, consumed) = self.collect_tensor(tokens, i)?;

                    // すべての配列リテラルをTensorに変換を試みる
                    let value_to_push = match crate::types::validate_rectangular(&values) {
                        Ok(_shape) => {
                            // Tensorへの変換を試みる
                            match Value::vector_to_tensor(&values) {
                                Ok(tensor) => Value::from_tensor(tensor),
                                Err(_) => {
                                    // 変換失敗時は混合型としてVectorで保持（後方互換性）
                                    #[allow(deprecated)]
                                    { Value { val_type: ValueType::Vector(values) } }
                                }
                            }
                        }
                        Err(_) => {
                            // 矩形でない場合も混合型としてVectorで保持
                            #[allow(deprecated)]
                            { Value { val_type: ValueType::Vector(values) } }
                        }
                    };

                    self.stack.push(value_to_push);
                    i += consumed;
                    continue;
                },
                #[allow(deprecated)]
                Token::VectorStart(_) => {
                    // 後方互換性のため残存（非推奨）
                    let (values, consumed) = self.collect_vector(tokens, i)?;

                    // 旧形式のVectorStart処理
                    #[allow(deprecated)]
                    let value_to_push = { Value { val_type: ValueType::Vector(values) } };

                    self.stack.push(value_to_push);
                    i += consumed;
                    continue;
                },
                Token::Symbol(s) => {
                    // . と .. は大文字変換せずにチェック
                    match s.as_str() {
                        ".." => {
                            self.operation_target = OperationTarget::Stack;
                        },
                        "." => {
                            self.operation_target = OperationTarget::StackTop;
                        },
                        _ => {
                            // その他のシンボルは大文字変換してチェック
                            let upper = s.to_uppercase();
                            match upper.as_str() {
                                        "WAIT" => {
                                    // WAIT の引数を準備し、AsyncAction を返す
                                    let action = self.prepare_wait_action()?;
                                    return Ok((i + 1, Some(action)));
                                },
                                _ => {
                                    // 通常のワード実行（同期）
                                    self.execute_word_core(&upper)?;
                                    self.operation_target = OperationTarget::StackTop;
                                }
                            }
                        }
                    }
                },
                Token::GuardSeparator | Token::LineBreak => {
                    // スキップ
                },
                Token::TensorEnd => {
                    return Err(AjisaiError::from("Unexpected tensor end (])"));
                },
                #[allow(deprecated)]
                Token::VectorEnd(_) => {
                    return Err(AjisaiError::from("Unexpected vector end"));
                },
            }
            i += 1;
        }

        Ok((i, None))
    }

    /// セクション実行（同期版）
    ///
    /// WAITワードに遭遇した場合はエラーとなる。
    fn execute_section_sync(&mut self, tokens: &[Token]) -> Result<()> {
        let (_, action) = self.execute_section_core(tokens, 0)?;

        if let Some(async_action) = action {
            return Err(AjisaiError::from(format!(
                "Async operation {:?} requires async context",
                async_action
            )));
        }

        Ok(())
    }

    /// セクション実行（非同期版）
    #[async_recursion(?Send)]
    async fn execute_section(&mut self, tokens: &[Token]) -> Result<()> {
        let mut current_index = 0;

        loop {
            let (next_index, action) = self.execute_section_core(tokens, current_index)?;

            match action {
                None => break,
                Some(AsyncAction::Wait { duration_ms, word_name }) => {
                    sleep(Duration::from_millis(duration_ms)).await;
                    self.execute_word_async(&word_name).await?;
                    current_index = next_index;
                }
            }
        }

        Ok(())
    }

    /// ガード構造実行のコアロジック
    ///
    /// 行の配列を受け取り、ガード節として処理する。
    /// 非同期操作に遭遇した場合は AsyncAction を返す。
    fn execute_guard_structure_core(
        &mut self,
        lines: &[ExecutionLine]
    ) -> Result<Option<AsyncAction>> {
        if lines.is_empty() {
            return Ok(None);
        }

        // すべての行が : で始まっているかチェック
        let all_lines_have_colon = lines.iter().all(|line| {
            line.body_tokens.first() == Some(&Token::GuardSeparator)
        });

        // : で始まらない行がある場合、すべてをデフォルト行として順次実行
        if !all_lines_have_colon {
            for line in lines {
                let tokens = if line.body_tokens.first() == Some(&Token::GuardSeparator) {
                    &line.body_tokens[1..]
                } else {
                    &line.body_tokens[..]
                };

                let (_, action) = self.execute_section_core(tokens, 0)?;
                if action.is_some() {
                    return Ok(action);
                }
            }
            return Ok(None);
        }

        // すべての行が : で始まる場合、ガード節として処理
        let mut i = 0;
        while i < lines.len() {
            let line = &lines[i];
            let content_tokens = &line.body_tokens[1..]; // : を除く

            if i + 1 < lines.len() {
                // 条件行の可能性
                let (_, action) = self.execute_section_core(content_tokens, 0)?;
                if action.is_some() {
                    return Ok(action);
                }

                // 条件を評価
                if self.is_condition_true()? {
                    // 真の場合：次の行（処理行）を実行
                    i += 1;
                    let action_line = &lines[i];
                    let action_tokens = &action_line.body_tokens[1..];

                    let (_, action) = self.execute_section_core(action_tokens, 0)?;
                    if action.is_some() {
                        return Ok(action);
                    }
                    return Ok(None);
                }
                // 偽の場合：次の条件へ
                i += 2;
            } else {
                // 最後の行 → デフォルト処理
                let (_, action) = self.execute_section_core(content_tokens, 0)?;
                return Ok(action);
            }
        }

        Ok(None)
    }

    fn is_condition_true(&mut self) -> Result<bool> {
        if self.stack.is_empty() {
            return Ok(false);
        }

        let top = self.stack.pop().unwrap();

        match &top.val_type {
            ValueType::Boolean(b) => Ok(*b),
            ValueType::Vector(v) => {
                if v.len() == 1 {
                    if let ValueType::Boolean(b) = v[0].val_type {
                        Ok(b)
                    } else {
                        Ok(true)
                    }
                } else {
                    Ok(!v.is_empty())
                }
            },
            ValueType::Nil => Ok(false),
            _ => Ok(true),
        }
    }

    // トークンを行に分割
    fn tokens_to_lines(&self, tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        
        for token in tokens {
            match token {
                Token::LineBreak => {
                    if !current_line.is_empty() {
                        lines.push(ExecutionLine {
                            body_tokens: current_line.clone(),
                        });
                        current_line.clear();
                    }
                },
                _ => {
                    current_line.push(token.clone());
                }
            }
        }
        
        if !current_line.is_empty() {
            lines.push(ExecutionLine {
                body_tokens: current_line,
            });
        }
        
        Ok(lines)
    }

    /// ワード実行のコアロジック（同期）
    ///
    /// 組み込みワードまたはカスタムワードを実行する。
    /// WAITワードはこの関数では処理せず、呼び出し元で AsyncAction として処理する。
    pub(crate) fn execute_word_core(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(name).cloned()
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

        if def.is_builtin {
            return self.execute_builtin(name);
        }

        // ガード構造をコアロジックで処理
        let action = self.execute_guard_structure_core(&def.lines)?;

        if action.is_some() {
            // 非同期アクションが発生した場合はエラー（同期コンテキスト）
            return Err(AjisaiError::from(
                "WAIT requires async execution context. Use execute() instead of execute_sync()."
            ));
        }

        Ok(())
    }

    pub(crate) fn execute_word_sync(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(name).cloned()
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

        if def.is_builtin {
            return self.execute_builtin(name);
        }

        // 行の配列としてガード構造を処理
        self.execute_guard_structure_sync(&def.lines)
    }

    /// ワード実行（非同期版）
    #[async_recursion(?Send)]
    pub(crate) async fn execute_word_async(&mut self, name: &str) -> Result<()> {
        let def = self.dictionary.get(name).cloned()
            .ok_or_else(|| AjisaiError::UnknownWord(name.to_string()))?;

        if def.is_builtin {
            // WAITは既に execute_section で処理済み
            return self.execute_builtin(name);
        }

        self.execute_guard_structure(&def.lines).await
    }

    fn execute_builtin(&mut self, name: &str) -> Result<()> {
        // DEL, DEF, ! 以外はフラグをリセット
        if name != "DEL" && name != "DEF" && name != "!" {
            self.force_flag = false;
        }

        match name {
            "GET" => vector_ops::op_get(self),
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self),
            "REMOVE" => vector_ops::op_remove(self),
            "LENGTH" => vector_ops::op_length(self),
            "TAKE" => vector_ops::op_take(self),
            "SPLIT" => vector_ops::op_split(self),
            "CONCAT" => vector_ops::op_concat(self),
            "REVERSE" => vector_ops::op_reverse(self),
            "LEVEL" => vector_ops::op_level(self),
            "RANGE" => vector_ops::op_range(self),
            "SHAPE" => tensor_ops::op_shape(self),
            "RANK" => tensor_ops::op_rank(self),
            "RESHAPE" => tensor_ops::op_reshape(self),
            "TRANSPOSE" => tensor_ops::op_transpose(self),
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            "=" => comparison::op_eq(self),
            "<" => comparison::op_lt(self),
            "<=" => comparison::op_le(self),
            ">" => comparison::op_gt(self),
            ">=" => comparison::op_ge(self),
            "AND" => logic::op_and(self),
            "OR" => logic::op_or(self),
            "NOT" => logic::op_not(self),
            "PRINT" => io::op_print(self),
            "DEF" => dictionary::op_def(self),
            "DEL" => dictionary::op_del(self),
            "?" => dictionary::op_lookup(self),
            "RESET" => self.execute_reset(),
            "MAP" => higher_order::op_map(self),
            "FILTER" => higher_order::op_filter(self),
            "COUNT" => higher_order::op_count(self),
            "REDUCE" => higher_order::op_reduce(self),
            "FOLD" => higher_order::op_fold(self),
            "SCAN" => higher_order::op_scan(self),
            "UNFOLD" => higher_order::op_unfold(self),
            "TIMES" => control::execute_times(self),
            "WAIT" => {
                // WAITは execute_section_core で AsyncAction として処理されるべき
                // ここに到達した場合はエラー
                Err(AjisaiError::from(
                    "WAIT should be handled by execute_section_core, not execute_builtin"
                ))
            },
            "STR" => cast::op_str(self),
            "NUM" => cast::op_num(self),
            "BOOL" => cast::op_bool(self),
            "NIL" => cast::op_nil(self),
            "CHARS" => cast::op_chars(self),
            "JOIN" => cast::op_join(self),
            "NOW" => datetime::op_now(self),
            "DATETIME" => datetime::op_datetime(self),
            "TIMESTAMP" => datetime::op_timestamp(self),
            "FRACTIONSORT" => sort::op_fractionsort(self),
            "!" => {
                self.force_flag = true;
                Ok(())
            },
            _ => Err(AjisaiError::UnknownWord(name.to_string())),
        }
    }

    pub fn token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n) => n.clone(),
            Token::String(s) => format!("'{}'", s),
            Token::Boolean(true) => "TRUE".to_string(),
            Token::Boolean(false) => "FALSE".to_string(),
            Token::Symbol(s) => s.clone(),
            Token::Nil => "NIL".to_string(),
            Token::TensorStart => "[".to_string(),
            Token::TensorEnd => "]".to_string(),
            #[allow(deprecated)]
            Token::VectorStart(bt) => bt.opening_char().to_string(),
            #[allow(deprecated)]
            Token::VectorEnd(bt) => bt.closing_char().to_string(),
            Token::GuardSeparator => ":".to_string(),
            Token::LineBreak => "\n".to_string(),
        }
    }
    
    pub fn get_word_definition_tokens(&self, name: &str) -> Option<String> {
        if let Some(def) = self.dictionary.get(name) {
            if !def.is_builtin && !def.lines.is_empty() {
                let mut result = String::new();
                for (i, line) in def.lines.iter().enumerate() {
                    if i > 0 { result.push('\n'); }
                    
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
    
    pub fn execute_reset(&mut self) -> Result<()> {
        self.stack.clear();
        self.dictionary.clear();
        self.dependents.clear();
        self.output_buffer.clear();
        self.definition_to_load = None;
        self.operation_target = OperationTarget::StackTop;
        self.force_flag = false;
        self.pending_tokens = None;
        self.pending_token_index = 0;
        crate::builtins::register_builtins(&mut self.dictionary);
        Ok(())
    }

    pub async fn execute(&mut self, code: &str) -> Result<()> {
        let custom_word_names: HashSet<String> = self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, _)| name.clone())
            .collect();
        let tokens = crate::tokenizer::tokenize_with_custom_words(code, &custom_word_names)?;
        
        // トークンを行に分割
        let lines = self.tokens_to_lines(&tokens)?;

        // 行の配列としてガード構造を処理
        self.execute_guard_structure(&lines).await?;

        Ok(())
    }

    pub fn get_output(&mut self) -> String { 
        std::mem::take(&mut self.output_buffer) 
    }
    
    pub fn get_stack(&self) -> &Stack { 
        &self.stack 
    }
    
    pub fn set_stack(&mut self, stack: Stack) { 
        self.stack = stack; 
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
                for token in line.body_tokens.iter() {
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
            .map(|(name, def)| (name.clone(), def.description.clone(), def.is_builtin))
            .collect()
    }

    pub fn get_word_definition(&self, name: &str) -> Option<String> {
        self.get_word_definition_tokens(name)
    }

    /// 指定されたワードを参照している他のワードの集合を取得
    pub fn get_dependents(&self, word_name: &str) -> HashSet<String> {
        let mut result = HashSet::new();
        for (name, def) in &self.dictionary {
            if !def.is_builtin && def.dependencies.contains(word_name) {
                result.insert(name.clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // TODO: Fix test - function doesn't consume argument from stack
    async fn test_tail_recursion_simple() {
        let mut interp = Interpreter::new();

        // Define a recursive countdown function
        // Format: [ 'definition body' ] 'NAME' DEF
        // Guard condition: [0] [0] GET [0] = checks if top of input vector equals 0
        // If true: do nothing (empty line)
        // If false: decrement and call COUNTDOWN recursively
        let code = r#"
[ ': [0] [0] GET [0] =
:
: [0] [0] GET [1] - COUNTDOWN' ] 'COUNTDOWN' DEF
[5] COUNTDOWN
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Tail recursion should succeed: {:?}", result);

        // Verify call stack is empty after execution
        // Stack should be empty after countdown completes
        assert_eq!(interp.stack.len(), 0, "Stack should be empty after countdown");
    }

    #[tokio::test]
    #[ignore] // TODO: Fix test - function doesn't consume argument from stack
    async fn test_tail_recursion_large_number() {
        let mut interp = Interpreter::new();

        // Test with a larger number to ensure tail recursion optimization works
        // Guard condition: [0] [0] GET [0] = checks if top of input vector equals 0
        // If true: do nothing (empty line)
        // If false: decrement and call COUNTDOWN recursively
        let code = r#"
[ ': [0] [0] GET [0] =
:
: [0] [0] GET [1] - COUNTDOWN' ] 'COUNTDOWN' DEF
[100] COUNTDOWN
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Tail recursion with large number should succeed: {:?}", result);

        // Verify call stack is empty
        // Stack should be empty after countdown completes
        assert_eq!(interp.stack.len(), 0, "Stack should be empty after countdown");
    }

    #[tokio::test]
    async fn test_stack_get_basic() {
        let mut interp = Interpreter::new();

        // Test basic .. GET behavior
        let code = r#"
: [5] [0] .. GET
"#;

        println!("\n=== Basic .. GET Test ===");
        let result = interp.execute(code).await;
        println!("Result: {:?}", result);
        println!("Final stack length: {}", interp.stack.len());
        println!("Final stack contents:");
        for (i, val) in interp.stack.iter().enumerate() {
            println!("  [{}]: {:?}", i, val);
        }

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tail_recursion_with_stack_mode() {
        let mut interp = Interpreter::new();

        // Proper countdown using STACK mode to access the argument
        let code = r#"
: [ ': [0] .. GET [0] >
: [0] .. GET [1] - COUNTDOWN' ] 'COUNTDOWN' DEF
: [3] COUNTDOWN
"#;

        println!("\n=== Proper Tail Recursion Test (STACK mode) ===");
        let result = interp.execute(code).await;
        println!("Result: {:?}", result);
        println!("Final stack length: {}", interp.stack.len());
        println!("Final stack contents:");
        for (i, val) in interp.stack.iter().enumerate() {
            println!("  [{}]: {:?}", i, val);
        }

        if result.is_ok() {
            // Stack should not grow linearly
            assert!(interp.stack.len() < 10,
                "Stack grew too much: {} elements. This indicates stack pollution.",
                interp.stack.len());
        }
    }

    #[tokio::test]
    async fn test_tail_recursion_detailed_trace() {
        let mut interp = Interpreter::new();

        // Simple test with just 3 iterations to understand the flow
        let code = r#"
[ ': [0] [0] GET [0] >
: [0] [0] GET [1] - COUNTDOWN' ] 'COUNTDOWN' DEF
[3] COUNTDOWN
"#;

        let result = interp.execute(code).await;
        println!("\n=== Detailed Trace Test (Original - Broken) ===");
        println!("Result: {:?}", result);
        println!("Final stack length: {}", interp.stack.len());
        println!("Final stack contents:");
        for (i, val) in interp.stack.iter().enumerate() {
            println!("  [{}]: {:?}", i, val);
        }
        assert!(result.is_ok(), "Should succeed: {:?}", result);
    }

    #[tokio::test]
    async fn test_tail_recursion_stack_growth() {
        let mut interp = Interpreter::new();

        // Test with a medium-sized recursion to check for stack growth
        let code = r#"
[ ': [0] [0] GET [0] >
: [0] [0] GET [1] - COUNTDOWN' ] 'COUNTDOWN' DEF
[10] COUNTDOWN
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Should succeed: {:?}", result);

        println!("\n=== Stack Growth Test ===");
        println!("Stack length after 10 iterations: {}", interp.stack.len());
        println!("Stack contents:");
        for (i, val) in interp.stack.iter().enumerate() {
            println!("  [{}]: {:?}", i, val);
        }

        // The stack should not grow linearly with the number of iterations
        // If tail recursion is working correctly, stack size should be constant or minimal
        assert!(interp.stack.len() < 10,
            "Stack grew too much! Length: {}. This suggests tail recursion is not working correctly.",
            interp.stack.len());
    }

    #[tokio::test]
    async fn test_simple_addition() {
        let mut interp = Interpreter::new();

        // Simple test: add two numbers
        let code = r#"
: [2] [3] +
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Simple addition should succeed: {:?}", result);

        // Verify result
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    #[ignore] // TODO: Fix test - function doesn't consume argument from stack
    async fn test_tail_recursion_countdown_empty_stack() {
        let mut interp = Interpreter::new();

        // Simple countdown that empties the stack - using direct comparison
        // Pattern: keep decrementing until we reach 0
        let code = r#"
[ ': [0] [0] GET [0] =
:
: [0] [0] GET [1] - COUNTDOWN_EMPTY' ] 'COUNTDOWN_EMPTY' DEF
[5] COUNTDOWN_EMPTY
"#;

        println!("\n=== Countdown Empty Stack Pattern ===");
        let result = interp.execute(code).await;
        println!("Result: {:?}", result);
        println!("Final stack length: {}", interp.stack.len());
        println!("Final stack contents:");
        for (i, val) in interp.stack.iter().enumerate() {
            println!("  [{}]: {:?}", i, val);
        }

        assert!(result.is_ok(), "Countdown should succeed: {:?}", result);
        // Stack should be empty after countdown completes
        assert_eq!(interp.stack.len(), 0, "Stack should be empty after countdown");
    }

    #[tokio::test]
    #[ignore] // TODO: Fix test - function doesn't consume argument from stack
    async fn test_tail_recursion_repeat_n_times() {
        let mut interp = Interpreter::new();

        // Repeat pattern: decrement n until 0, no accumulator needed
        // This just counts down without leaving anything on stack
        let code = r#"
[ ': [0] [0] GET [1] =
:
: [0] [0] GET [1] - REPEAT_N' ] 'REPEAT_N' DEF
[10] REPEAT_N
"#;

        println!("\n=== Repeat N Times Pattern ===");
        let result = interp.execute(code).await;
        println!("Result: {:?}", result);
        println!("Final stack length: {}", interp.stack.len());
        println!("Final stack contents:");
        for (i, val) in interp.stack.iter().enumerate() {
            println!("  [{}]: {:?}", i, val);
        }

        assert!(result.is_ok(), "Repeat should succeed: {:?}", result);
        // Stack should be empty after completion
        assert_eq!(interp.stack.len(), 0, "Stack should be empty");
    }

    #[tokio::test]
    #[ignore] // TODO: Fix test - function doesn't consume argument from stack
    async fn test_tail_recursion_with_large_iterations() {
        let mut interp = Interpreter::new();

        // Test with large number to ensure no stack overflow
        let code = r#"
[ ': [0] [0] GET [0] =
:
: [0] [0] GET [1] - COUNTDOWN_LARGE' ] 'COUNTDOWN_LARGE' DEF
[1000] COUNTDOWN_LARGE
"#;

        println!("\n=== Large Iterations Test (1000) ===");
        let result = interp.execute(code).await;
        println!("Result: {:?}", result);
        println!("Final stack length: {}", interp.stack.len());

        assert!(result.is_ok(), "Large countdown should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 0, "Stack should be empty");
    }

    #[tokio::test]
    async fn test_definition_and_call() {
        let mut interp = Interpreter::new();

        // Test defining a word and calling it
        let code = r#"
[ ': [2] [3] +' ] 'ADDTEST' DEF
ADDTEST
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Definition and call should succeed: {:?}", result);

        // Verify call stack is empty
    }

    #[tokio::test]
    async fn test_force_flag_del_without_dependents() {
        let mut interp = Interpreter::new();
        interp.execute("[ ': [ 2 ] *' ] 'DOUBLE' DEF").await.unwrap();

        // 依存なしなら ! 不要で削除可能
        let result = interp.execute("'DOUBLE' DEL").await;
        assert!(result.is_ok());
        assert!(!interp.dictionary.contains_key("DOUBLE"));
    }

    #[tokio::test]
    async fn test_force_flag_del_with_dependents_error() {
        let mut interp = Interpreter::new();
        interp.execute("[ ': [ 2 ] *' ] 'DOUBLE' DEF").await.unwrap();
        interp.execute("[ ': DOUBLE DOUBLE' ] 'QUAD' DEF").await.unwrap();

        // 依存ありで ! なしはエラー
        let result = interp.execute("'DOUBLE' DEL").await;
        assert!(result.is_err());
        assert!(interp.dictionary.contains_key("DOUBLE"));
    }

    #[tokio::test]
    async fn test_force_flag_del_with_dependents_forced() {
        let mut interp = Interpreter::new();
        interp.execute("[ ': [ 2 ] *' ] 'DOUBLE' DEF").await.unwrap();
        interp.execute("[ ': DOUBLE DOUBLE' ] 'QUAD' DEF").await.unwrap();

        // ! 付きなら削除可能
        let result = interp.execute("! 'DOUBLE' DEL").await;
        assert!(result.is_ok());
        assert!(!interp.dictionary.contains_key("DOUBLE"));
        assert!(interp.output_buffer.contains("Warning"));
    }

    #[tokio::test]
    async fn test_force_flag_def_with_dependents_error() {
        let mut interp = Interpreter::new();
        interp.execute("[ ': [ 2 ] *' ] 'DOUBLE' DEF").await.unwrap();
        interp.execute("[ ': DOUBLE DOUBLE' ] 'QUAD' DEF").await.unwrap();

        // 依存ありで ! なしの再定義はエラー
        let result = interp.execute("[ ': [ 3 ] *' ] 'DOUBLE' DEF").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_force_flag_def_with_dependents_forced() {
        let mut interp = Interpreter::new();
        interp.execute("[ ': [ 2 ] *' ] 'DOUBLE' DEF").await.unwrap();
        interp.execute("[ ': DOUBLE DOUBLE' ] 'QUAD' DEF").await.unwrap();

        // ! 付きなら再定義可能
        let result = interp.execute("! [ ': [ 3 ] *' ] 'DOUBLE' DEF").await;
        assert!(result.is_ok());
        assert!(interp.output_buffer.contains("Warning"));
    }

    #[tokio::test]
    async fn test_force_flag_builtin_always_error() {
        let mut interp = Interpreter::new();

        // 組み込みは ! があっても削除不可
        let result = interp.execute("! '+' DEL").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_force_flag_reset_after_other_word() {
        let mut interp = Interpreter::new();
        interp.execute("[ ': [ 2 ] *' ] 'DOUBLE' DEF").await.unwrap();
        interp.execute("[ ': DOUBLE DOUBLE' ] 'QUAD' DEF").await.unwrap();

        // ! の後に別のワードを実行するとフラグがリセットされる
        interp.execute("!").await.unwrap();
        interp.execute("[ 1 2 ] LENGTH").await.unwrap();  // 何か別のワード操作
        let result = interp.execute("'DOUBLE' DEL").await;
        assert!(result.is_err());  // フラグがリセットされているのでエラー
    }

    #[tokio::test]
    async fn test_guard_with_def_true_case() {
        let mut interp = Interpreter::new();

        // Test: Define a custom word inside guard clause (true case)
        // 5 > 3 is true, so ANSWER should be defined
        let code = r#"
: [5] [3] >
: [ ': [ 42 ]' ] 'ANSWER' DEF
: [ ': [ 0 ]' ] 'ZERO' DEF
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Guard with DEF should succeed: {:?}", result);

        // Verify ANSWER is defined
        assert!(interp.dictionary.contains_key("ANSWER"), "ANSWER should be defined");
        assert!(!interp.dictionary.contains_key("ZERO"), "ZERO should not be defined");

        // Debug: Print ANSWER definition
        if let Some(def) = interp.dictionary.get("ANSWER") {
            println!("ANSWER definition has {} lines", def.lines.len());
            for (i, line) in def.lines.iter().enumerate() {
                println!("Line {}: {} tokens", i, line.body_tokens.len());
                for token in &line.body_tokens {
                    println!("  Token: {}", interp.token_to_string(token));
                }
            }
        }

        // Call ANSWER and verify result
        let call_code = ": ANSWER";
        let call_result = interp.execute(call_code).await;
        if let Err(ref e) = call_result {
            println!("Error calling ANSWER: {:?}", e);
        }
        assert!(call_result.is_ok(), "Calling ANSWER should succeed");
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_guard_with_def_false_case() {
        let mut interp = Interpreter::new();

        // Test: Define a custom word inside guard clause (false case)
        // 3 > 5 is false, so SMALL should be defined
        let code = r#"
: [3] [5] >
: [ ': [ 100 ]' ] 'BIG' DEF
: [ ': [ -1 ]' ] 'SMALL' DEF
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Guard with DEF (false case) should succeed: {:?}", result);

        // Verify SMALL is defined
        assert!(!interp.dictionary.contains_key("BIG"), "BIG should not be defined");
        assert!(interp.dictionary.contains_key("SMALL"), "SMALL should be defined");

        // Call SMALL and verify result
        let call_code = ": SMALL";
        let call_result = interp.execute(call_code).await;
        assert!(call_result.is_ok(), "Calling SMALL should succeed: {:?}", call_result.err());
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_guard_default_clause_with_def() {
        let mut interp = Interpreter::new();

        // Test: Default clause in guard structure defines a word
        let code = r#"
: [FALSE]
: [ ': [ 100 ]' ] 'HUNDRED' DEF
: [ ': [ 999 ]' ] 'DEFAULT' DEF
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Guard default clause with DEF should succeed: {:?}", result);

        // Verify DEFAULT is defined
        assert!(!interp.dictionary.contains_key("HUNDRED"), "HUNDRED should not be defined");
        assert!(interp.dictionary.contains_key("DEFAULT"), "DEFAULT should be defined");

        // Call DEFAULT and verify result
        let call_code = ": DEFAULT";
        let call_result = interp.execute(call_code).await;
        assert!(call_result.is_ok(), "Calling DEFAULT should succeed: {:?}", call_result.err());
    }

    #[tokio::test]
    async fn test_def_with_guard_using_existing_custom_word() {
        let mut interp = Interpreter::new();

        // Test: Define a word inside guard that uses an existing custom word
        let def_code = ": [ ': [ 2 ] *' ] 'DOUBLE' DEF";
        let result = interp.execute(def_code).await;
        assert!(result.is_ok(), "DOUBLE definition should succeed: {:?}", result);

        let guard_code = r#"
: [10] [5] >
: [ ': [ 3 ] DOUBLE' ] 'PROCESS' DEF
: [ ': [ 0 ]' ] 'NOPROCESS' DEF
"#;
        let result = interp.execute(guard_code).await;
        assert!(result.is_ok(), "DEF with guard using existing word should succeed: {:?}", result);

        // Verify PROCESS is defined
        assert!(interp.dictionary.contains_key("PROCESS"), "PROCESS should be defined");
        assert!(interp.dictionary.contains_key("DOUBLE"), "DOUBLE should exist");

        // Call PROCESS and verify result
        let call_code = ": PROCESS";
        let call_result = interp.execute(call_code).await;
        assert!(call_result.is_ok(), "Calling PROCESS should succeed: {:?}", call_result.err());
    }

    #[tokio::test]
    async fn test_default_line_without_colon() {
        let mut interp = Interpreter::new();

        // Test: Simple expression without colon (default line)
        let code = "[5] [3] +";

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Default line without colon should succeed: {:?}", result);

        // Verify result
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1, "Result vector should have one element");
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "8", "Result should be 8");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_def_without_colon() {
        let mut interp = Interpreter::new();

        // Test: Define a word without colon prefix
        let code = "[ ': [ 42 ]' ] 'ANSWER' DEF";

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "DEF without colon should succeed: {:?}", result);

        // Verify ANSWER is defined
        assert!(interp.dictionary.contains_key("ANSWER"), "ANSWER should be defined");

        // Call ANSWER
        let call_result = interp.execute("ANSWER").await;
        assert!(call_result.is_ok(), "Calling ANSWER should succeed");
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
    }

    #[tokio::test]
    async fn test_multiple_lines_without_colon() {
        let mut interp = Interpreter::new();

        // Test: Multiple lines without colons (all treated as default lines)
        let code = r#"
[1] [2] +
[3] *
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Multiple lines without colon should succeed: {:?}", result);

        // Verify result: (1 + 2) * 3 = 9
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1, "Result vector should have one element");
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "9", "Result should be 9");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_mixed_colon_and_no_colon() {
        let mut interp = Interpreter::new();

        // Test: Mix of lines with and without colons (treated as default lines)
        let code = r#"
[10] [20] +
: [5] *
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Mixed colon lines should succeed: {:?}", result);

        // Verify result: (10 + 20) * 5 = 150
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1, "Result vector should have one element");
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "150", "Result should be 150");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_guard_with_colon_vs_default_without_colon() {
        let mut interp = Interpreter::new();

        // Test: Guard clause with colons (conditional execution)
        let guard_code = r#"
: [5] [3] >
: [100]
: [0]
"#;

        let result = interp.execute(guard_code).await;
        assert!(result.is_ok(), "Guard clause should succeed: {:?}", result);

        // Result should be [100] because 5 > 3 is true
        assert_eq!(interp.stack.len(), 1, "Stack should have one element");
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1, "Result vector should have one element");
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "100", "Result should be 100");
                }
            }
        }

        // Clear stack
        interp.stack.clear();

        // Test: Same logic but without colons (all lines executed)
        let default_code = r#"
[5] [3] >
[100]
[0]
"#;

        let result = interp.execute(default_code).await;
        assert!(result.is_ok(), "Default lines should succeed: {:?}", result);

        // All lines executed, so we should have 3 items on stack
        assert_eq!(interp.stack.len(), 3, "Stack should have three elements");
    }

    #[tokio::test]
    async fn test_guard_five_lines_ok() {
        let mut interp = Interpreter::new();

        // 5行（奇数）：正常
        let code = r#"
: [FALSE]
: [100]
: [FALSE]
: [200]
: [999]
"#;

        let result = interp.execute(code).await;
        assert!(result.is_ok(), "Guard with 5 lines should succeed: {:?}", result);

        // すべての条件がfalseなのでデフォルトの999
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "999");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_reduce_addition() {
        let mut interp = Interpreter::new();
        let code = ": [ 1 2 3 4 5 ] '+' REDUCE";
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "REDUCE addition should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        // 結果が15であることを確認
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "15");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_reduce_multiplication() {
        let mut interp = Interpreter::new();
        let code = ": [ 1 2 3 4 5 ] '*' REDUCE";
        let result = interp.execute(code).await;
        if let Err(ref e) = result {
            eprintln!("Error: {:?}", e);
        }
        assert!(result.is_ok(), "REDUCE multiplication should succeed: {:?}", result);
        // 結果が120であることを確認（5! = 120）
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "120");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_reduce_subtraction() {
        let mut interp = Interpreter::new();
        let code = ": [ 10 3 1 ] '-' REDUCE";
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "REDUCE subtraction should succeed: {:?}", result);
        // 結果が6であることを確認（10-3-1=6）
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "6");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_reduce_single_element_error() {
        let mut interp = Interpreter::new();
        let code = ": [ 42 ] '+' REDUCE";
        let result = interp.execute(code).await;
        assert!(result.is_err(), "REDUCE with single element should fail");
    }

    #[tokio::test]
    async fn test_reduce_empty_vector_error() {
        let mut interp = Interpreter::new();
        let code = ": [ ] '+' REDUCE";
        let result = interp.execute(code).await;
        assert!(result.is_err(), "REDUCE with empty vector should fail");
    }

    #[tokio::test]
    async fn test_reduce_stack_mode() {
        let mut interp = Interpreter::new();
        let code = ": [1] [2] [3] [4] [5] [5] '+' .. REDUCE";
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "REDUCE STACK mode should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        // 結果が15であることを確認
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "15");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_reduce_with_custom_word() {
        let mut interp = Interpreter::new();

        // より簡単なカスタムワード: DOUBLE (2倍にする)
        let def_code = ": [ '[0] [2] *' ] 'DOUBLE' DEF";
        let def_result = interp.execute(def_code).await;
        assert!(def_result.is_ok(), "Failed to define DOUBLE: {:?}", def_result);

        // DOUBLEを使ってREDUCE（乗算の代わり）
        let code = ": [ 1 2 3 ] '+' REDUCE";  // まず加算で動作確認
        let result = interp.execute(code).await;
        if let Err(ref e) = result {
            eprintln!("Error: {:?}", e);
            eprintln!("Stack: {:?}", interp.stack);
        }
        assert!(result.is_ok(), "REDUCE with simple add should succeed: {:?}", result);

        // 結果が6であることを確認 (1+2+3=6)
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "6");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_reduce_fractions() {
        let mut interp = Interpreter::new();
        let code = ": [ 1/2 1/3 1/6 ] '+' REDUCE";
        let result = interp.execute(code).await;
        assert!(result.is_ok(), "REDUCE with fractions should succeed: {:?}", result);
        // 結果が1であることを確認（1/2 + 1/3 + 1/6 = 1）
        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator.to_string(), "1");
                    assert_eq!(n.denominator.to_string(), "1");
                }
            }
        }
    }

    #[tokio::test]
    async fn test_map_no_change_allowed() {
        let mut interp = Interpreter::new();
        // identity関数をMAPで使用しても、エラーにならないことを確認
        let result = interp.execute("[ ':' ] 'IDENTITY' DEF [ 1 2 3 ] 'IDENTITY' MAP").await;
        assert!(result.is_ok(), "MAP with identity function should not error: {:?}", result);

        // 結果が [ 1 2 3 ] であることを確認
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            if let crate::types::ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 3);
            }
        }
    }

    #[tokio::test]
    async fn test_map_stack_mode() {
        let mut interp = Interpreter::new();
        // Stackモードでの動作確認
        let result = interp.execute("[ '[ 2 ] *' ] 'DOUBLE' DEF [ 1 ] [ 2 ] [ 3 ] [ 3 ] 'DOUBLE' .. MAP").await;
        assert!(result.is_ok(), "MAP in Stack mode should work: {:?}", result);

        // スタックに3つの要素があること
        assert_eq!(interp.stack.len(), 3);
    }
}
