use crate::interpreter::{AjisaiError, Interpreter};
use std::fmt;

/// Operatorの動作モード（スタック/ベクター）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpMode {
    Stack,  // S: スタック・モード (末尾を操作)
    Vector, // V: ベクター・モード (全体を操作)
}

/// スタック・モード時のスタック消費/生成数
#[derive(Debug, Clone, Copy)]
pub struct StackEffect {
    pub input: usize,  // 消費する数
    pub output: usize, // 生成する数
}

/// Operatorは辞書に格納されるワード（操作）です。
/// Rustのネイティブ関数として実装されます。
pub type NativeAction = Box<dyn Fn(&mut Interpreter) -> Result<(), AjisaiError> + Send + Sync>;

pub struct Operator {
    pub name: String,
    pub mode: OpMode,
    pub effect: StackEffect, // ModeStack の場合のみ意味を持つ
    action: NativeAction,
    // UserDef: Vec<String>, // 将来的なAjisaiコードによる定義
}

impl Operator {
    /// 新しいネイティブOperatorを定義します
    pub fn new_native(
        name: &str,
        mode: OpMode,
        effect: StackEffect,
        action: NativeAction,
    ) -> Self {
        Operator {
            name: name.to_string(),
            mode,
            effect,
            action,
        }
    }

    /// GUIに表示するシグネチャ文字列を生成します
    pub fn signature(&self) -> String {
        match self.mode {
            OpMode::Stack => format!("( S: {} -- {} )", self.effect.input, self.effect.output),
            OpMode::Vector => "( V: v -- ... )".to_string(),
        }
    }

    /// Operatorの動作を実行します
    pub fn execute(&self, interpreter: &mut Interpreter) -> Result<(), AjisaiError> {
        (self.action)(interpreter)
    }
}

/// OperatorはTrieに格納するため、名前でPartialEqを実装
impl PartialEq for Operator {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl fmt::Debug for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Operator")
            .field("name", &self.name)
            .field("mode", &self.mode)
            .field("effect", &self.effect)
            .finish()
    }
}
