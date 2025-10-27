use crate::operator::{NativeAction, OpMode, Operator, StackEffect};
use crate::rational::Rational;
use crate::trie_store::{new_shared_store, SharedTrieStore, TrieStore};
use crate::vstack::VStack;
use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::{Arc, RwLock};
// tauri::AppHandle は削除

/// Ajisai実行時エラー
#[derive(Debug, thiserror::Error)]
pub enum AjisaiError {
    #[error("Unknown token: {0}")]
    UnknownToken(String),
    #[error("Stack underflow: '{op}' needs {need} elements, found {found}")]
    StackUnderflow { op: String, need: usize, found: usize },
    #[error("Evaluation stack underflow")]
    EvalStackUnderflow,
    #[error("'{op}' requires a target operand")]
    TargetRequired(String),
    #[error("'{op}' requires a value on eval stack")]
    ValueRequired(String),
    #[error("'{op}' requires a name for assignment")]
    NameRequired(String),
    #[error("Division by zero")]
    DivisionByZero,
}

// OperandStoreは VStack への共有ポインタを格納する
type Operand = Arc<RwLock<VStack>>;
type OperandStore = SharedTrieStore<Operand>;
type OperatorStore = SharedTrieStore<Arc<Operator>>;

// 評価スタック。リテラルやOperandへのポインタを一時的に保持
type EvalStack = VecDeque<Operand>;

pub struct Interpreter {
    operators: OperatorStore,
    operands: OperandStore,
    eval_stack: EvalStack,
    target: Option<Operand>, // 現在の暗m
    // app_handle: AppHandle, // GUI通知用 (削除)
    output: Vec<String>, // 実行結果（コンソール出力）を保持するバッファ
}

impl Interpreter {
    /// 新しいInterpreterを初期化します
    pub fn new() -> Self {
        let operators = new_shared_store::<Arc<Operator>>();
        let operands = new_shared_store::<Operand>();
        
        let mut i = Interpreter {
            operators,
            operands,
            eval_stack: VecDeque::new(),
            target: None,
            // app_handle, (削除)
            output: Vec::new(),
        };
        i.define_builtins(); // 組み込みワードを定義
        i
    }
    
    // (Wasm_api.rsから呼ばれる) OperatorStore への参照
    pub fn get_operators_store(&self) -> OperatorStore {
        self.operators.clone()
    }
    
    // (Wasm_api.rsから呼ばれる) OperandStore への参照
    pub fn get_operands_store(&self) -> OperandStore {
        self.operands.clone()
    }

    /// 出力バッファをクリアします
    fn clear_output(&mut self) {
        self.output.clear();
    }
    
    /// 出力バッファに書き込みます
    fn write_output(&mut self, s: String) {
        self.output.push(s);
    }

    /// 入力された文字列を評価し、コンソール出力を返します
    pub fn eval(&mut self, line: &str) -> Result<Vec<String>, AjisaiError> {
        self.clear_output();
        let mut tokens = line.fields().collect::<VecDeque<_>>();
        
        while let Some(token) = tokens.pop_front() {
            match token {
                // 1. Vectorリテラル (簡易実装)
                "[" => {
                    let mut data = Vec::new();
                    while let Some(lit_token) = tokens.pop_front() {
                        if lit_token == "]" { break; }
                        if let Ok(r) = Rational::from_str(lit_token) {
                            data.push(r);
                        } else {
                            // TODO: エラーハンドリング
                        }
                    }
                    let v = Arc::new(RwLock::new(VStack::new()));
                    v.write().unwrap().set(data);
                    self.eval_stack.push_back(v);
                    continue;
                }
                "]" => continue, // `[` で処理済み

                // 2. 代入演算子 `->` (特別扱い)
                "->" => {
                    let name = tokens.pop_front()
                        .ok_or(AjisaiError::NameRequired("->".to_string()))?;
                    let v_to_assign = self.pop_eval_or_target()?;
                    
                    // 新しいVStackを作成し、内容をコピーする
                    let new_v = Arc::new(RwLock::new(VStack::new()));
                    new_v.write().unwrap().set(v_to_assign.read().unwrap().get_copy());
                    
                    // OperandStoreに登録
                    self.operands.write().unwrap().insert(name, new_v);
                    continue;
                }
                _ => {} // 他のトークンは続行
            }

            // 3. 数値リテラルか？
            if let Ok(r) = Rational::from_str(token) {
                let v = Arc::new(RwLock::new(VStack::new()));
                v.write().unwrap().push(r);
                self.eval_stack.push_back(v);
                continue;
            }

            // 4. Operator (ワード) か？
            if let Some(op) = self.operators.read().unwrap().find(token) {
                self.execute_operator(op)?;
                continue;
            }

            // 5. Operand (既存の名前) か？
            if let Some(v) = self.operands.read().unwrap().find(token) {
                if self.target.is_some() || !self.eval_stack.is_empty() {
                    self.eval_stack.push_back(v);
                } else {
                    self.target = Some(v);
                }
                continue;
            }

            // 6. 不明なトークン
            return Err(AjisaiError::UnknownToken(token.to_string()));
        }
        
        // 1行評価完了
        self.target = None;
        self.eval_stack.clear(); // 一時スタックをクリア
        Ok(self.output.clone()) // コンソール出力を返す
    }

    /// eval_stack または target から Operand を取得します
    fn pop_eval_or_target(&mut self) -> Result<Operand, AjisaiError> {
        if let Some(v) = self.eval_stack.pop_back() {
            Ok(v)
        } else if let Some(v) = self.target.take() {
            Ok(v)
        } else {
            Err(AjisaiError::EvalStackUnderflow)
        }
    }

    /// Operatorを実行します
    fn execute_operator(&mut self, op: Arc<Operator>) -> Result<(), AjisaiError> {
        if self.target.is_none() {
            self.target = self.eval_stack.pop_back();
        }
        
        if self.target.is_none() && op.name != "->" { // `->` は特別
             return Err(AjisaiError::TargetRequired(op.name.clone()));
        }

        match op.mode {
            OpMode::Stack => {
                let target_len = self.target.as_ref().map_or(0, |v| v.read().unwrap().len());
                if target_len < op.effect.input {
                    return Err(AjisaiError::StackUnderflow {
                        op: op.name.clone(),
                        need: op.effect.input,
                        found: target_len,
                    });
                }
                op.execute(self)?;
            }
            OpMode::Vector => {
                op.execute(self)?;
            }
        }
        Ok(())
    }

    // --- 組み込みワードの定義 ---
    fn define_builtins(&mut self) {
        let mut store = self.operators.write().unwrap();

        // + ( S: 2 -- 1 )
        store.insert("+", Arc::new(Operator::new_native(
            "+", OpMode::Stack, StackEffect { input: 2, output: 1 },
            Box::new(|i: &mut Interpreter| {
                let mut v = i.target.as_ref().unwrap().write().unwrap();
                let b = v.pop().unwrap();
                let a = v.pop().unwrap();
                v.push(a.add(&b));
                Ok(())
            })
        )));

        // . ( S: 1 -- 0 ) ... スタックトップを表示
        store.insert(".", Arc::new(Operator::new_native(
            ".", OpMode::Stack, StackEffect { input: 1, output: 0 },
            Box::new(|i: &mut Interpreter| {
                let r = i.target.as_ref().unwrap().write().unwrap().pop().unwrap();
                i.write_output(r.to_string());
                Ok(())
            })
        )));

        // push ( S: 1 -- 1 ) ... evalStackから取る
        store.insert("push", Arc::new(Operator::new_native(
            "push", OpMode::Stack, StackEffect { input: 0, output: 1 },
            Box::new(|i: &mut Interpreter| {
                let val_v = i.eval_stack.pop_back()
                    .ok_or(AjisaiError::ValueRequired("push".to_string()))?;
                let r = val_v.write().unwrap().pop()
                    .ok_or(AjisaiError::ValueRequired("push".to_string()))?;
                
                i.target.as_ref().unwrap().write().unwrap().push(r);
                Ok(())
            })
        )));
        
        // --- ベクター・モード (V:) ---

        // sum ( V: v -- n )
        store.insert("sum", Arc::new(Operator::new_native(
            "sum", OpMode::Vector, StackEffect { input: 0, output: 0 },
            Box::new(|i: &mut Interpreter| {
                let data = i.target.as_ref().unwrap().read().unwrap().get_copy();
                let total = data.iter().fold(Rational::zero(), |acc, r| acc.add(r));
                
                // 結果をevalStackにプッシュ
                let res_v = Arc::new(RwLock::new(VStack::new()));
                res_v.write().unwrap().push(total);
                i.eval_stack.push_back(res_v);
                i.target = None; // ターゲットを消費
                Ok(())
            })
        )));

        // sort ( V: v -- v )
        store.insert("sort", Arc::new(Operator::new_native(
            "sort", OpMode::Vector, StackEffect { input: 0, output: 0 },
            Box::new(|i: &mut Interpreter| {
                i.target.as_ref().unwrap().write().unwrap().sort();
                i.target = None; // ターゲットを消費
                Ok(())
            })
        )));

        // v. ( V: v -- 0 ) ... ベクター全体を表示
        store.insert("v.", Arc::new(Operator::new_native(
            "v.", OpMode::Vector, StackEffect { input: 0, output: 0 },
            Box::new(|i: &mut Interpreter| {
                let s = i.target.as_ref().unwrap().read().unwrap().to_string();
                i.write_output(s);
                i.target = None; // ターゲットを消費
                Ok(())
            })
        )));
    }
}
