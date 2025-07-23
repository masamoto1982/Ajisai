use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rug::Rational;

use crate::interpreter::Interpreter;

// --- 意味記憶 (Semantic Memory) ---
// 辞書に登録される実行可能なエンティティ
#[derive(Clone)]
pub enum Word {
    Builtin(WordFunc),
    UserDefined(Rc<Vec<Token>>),
}

impl std::fmt::Debug for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Word::Builtin(_) => write!(f, "Builtin"),
            Word::UserDefined(tokens) => write!(f, "UserDefined({:?})", tokens),
        }
    }
}

// WordがPartialEqを実装するため、手動で比較ロジックを定義
impl PartialEq for Word {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Word::Builtin(f1), Word::Builtin(f2)) => (f1 as *const ()) == (f2 as *const ()),
            (Word::UserDefined(v1), Word::UserDefined(v2)) => Rc::ptr_eq(v1, v2),
            _ => false,
        }
    }
}

pub type WordFunc = fn(&mut Interpreter) -> Result<(), String>;
pub type Dictionary = HashMap<String, Rc<Word>>;

// --- トークン ---
// ソースコードを解析した結果の最小単位
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Number(Rational),
    String(String),
    VectorStart, // `[`
    VectorEnd,   // `]`
    BlockStart,  // `{`
    BlockEnd,    // `}`
}

// --- スタック上の型 ---
// スタックに積まれる情報の種類
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    // --- データ型 (エピソード記憶) ---
    // これらは純粋なデータとしてスタックに積まれる
    Number(Rc<Rational>),
    String(Rc<String>),
    Bool(bool),
    Symbol(Rc<String>), // ワード名そのものをデータとして扱う
    Vector(Rc<RefCell<Vec<Type>>>),

    // --- 手続き記憶 (実行可能な計画) ---
    // IFやCALLなどの制御構造で使われる、即時実行可能なコードブロック
    Quotation(Rc<Vec<Token>>),

    // --- 意味記憶への参照 ---
    // 辞書に登録されたワードそのもの
    Word(Rc<Word>),
}
