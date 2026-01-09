// rust/src/types/mod.rs
//
// 統一分数アーキテクチャ（Unified Fraction Architecture）
//
// すべての値は Vec<Fraction> として表現される。
// 型チェックは存在しない。表示時のみ DisplayHint を参照する。
//
// ============================================================================
// 設計思想
// ============================================================================
//
// この設計が革命的なのは：
// 「型」という概念を言語レベルから除去した
//
// 従来の言語：データ → 型 → 演算可否の判定 → 演算
// Ajisai：データ → 演算（型チェックなし）→ 表示時に解釈
//
// すべてが分数。演算は常に成功する（数学的に意味があるかはユーザー次第）。
// これは自由と責任をユーザーに委ねる設計。
// FORTHの精神を、型システムという最も根本的なレベルで実現した。
//
// ============================================================================
// 内部表現
// ============================================================================
//
// | ユーザー入力     | 内部表現                              | 表示         |
// |------------------|---------------------------------------|--------------|
// | 42               | [42/1]                                | [ 42 ]       |
// | 1/3              | [1/3]                                 | [ 1/3 ]      |
// | TRUE             | [1/1]                                 | TRUE         |
// | FALSE            | [0/1]                                 | FALSE        |
// | 'A'              | [65/1]                                | 'A'          |
// | 'Hello'          | [72/1, 101/1, 108/1, 108/1, 111/1]    | 'Hello'      |
// | [ 1 2 3 ]        | [1/1, 2/1, 3/1]                       | [ 1 2 3 ]    |
// | NIL / [ ]        | []                                    | NIL          |

pub mod fraction;
pub mod display;
pub mod tensor;  // 行列演算ユーティリティ（Vectorベースで動作）

use std::collections::HashSet;
use self::fraction::Fraction;

/// 表示ヒント
///
/// 演算には一切使用しない。表示時のみ参照される。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DisplayHint {
    /// 自動判定（デフォルト）
    #[default]
    Auto,
    /// 数値として表示
    Number,
    /// 文字列として表示
    String,
    /// 真偽値として表示
    Boolean,
    /// 日時として表示
    DateTime,
}

/// Ajisai の唯一の値型
///
/// すべてのデータはこの構造体で表現される。
/// 数値、真偽値、文字列、配列、NIL の区別は内部的には存在しない。
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    /// データ本体（純粋な分数の配列）
    pub data: Vec<Fraction>,

    /// 表示ヒント（演算には使用しない）
    pub display_hint: DisplayHint,

    /// 形状情報（多次元配列の構造を保持）
    /// 例: [2, 3] は 2x3 の行列を表す
    /// 空の場合はスカラーまたは1次元配列
    pub shape: Vec<usize>,
}

impl Value {
    /// 空の値（NIL）を作成
    #[inline]
    pub fn nil() -> Self {
        Self {
            data: Vec::new(),
            display_hint: DisplayHint::Auto,
            shape: vec![],
        }
    }

    /// 単一の分数から値を作成（スカラー）
    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self {
            data: vec![f],
            display_hint: DisplayHint::Number,
            shape: vec![],  // スカラーは空の形状
        }
    }

    /// 整数から値を作成（スカラー）
    #[inline]
    pub fn from_int(n: i64) -> Self {
        Self {
            data: vec![Fraction::from(n)],
            display_hint: DisplayHint::Number,
            shape: vec![],  // スカラーは空の形状
        }
    }

    /// 真偽値から値を作成（スカラー）
    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self {
            data: vec![Fraction::from(if b { 1 } else { 0 })],
            display_hint: DisplayHint::Boolean,
            shape: vec![],  // スカラーは空の形状
        }
    }

    /// 文字列から値を作成
    pub fn from_string(s: &str) -> Self {
        let data: Vec<Fraction> = s.bytes().map(|b| Fraction::from(b as i64)).collect();
        let len = data.len();
        Self {
            data,
            display_hint: DisplayHint::String,
            shape: vec![len],
        }
    }

    /// シンボルから値を作成（文字列として格納）
    pub fn from_symbol(s: &str) -> Self {
        let data: Vec<Fraction> = s.bytes().map(|b| Fraction::from(b as i64)).collect();
        let len = data.len();
        Self {
            data,
            display_hint: DisplayHint::String,
            shape: vec![len],
        }
    }

    /// 分数の配列から値を作成
    #[inline]
    pub fn from_vec(v: Vec<Fraction>) -> Self {
        let len = v.len();
        Self {
            data: v,
            display_hint: DisplayHint::Auto,
            shape: vec![len],
        }
    }

    /// 分数の配列から値を作成（数値ヒント付き）
    #[inline]
    pub fn from_numbers(v: Vec<Fraction>) -> Self {
        let len = v.len();
        Self {
            data: v,
            display_hint: DisplayHint::Number,
            shape: vec![len],
        }
    }

    /// DateTimeとして値を作成（Unixタイムスタンプ、スカラー）
    #[inline]
    pub fn from_datetime(f: Fraction) -> Self {
        Self {
            data: vec![f],
            display_hint: DisplayHint::DateTime,
            shape: vec![],  // スカラーは空の形状
        }
    }

    /// 表示ヒントを設定
    #[inline]
    pub fn with_hint(mut self, hint: DisplayHint) -> Self {
        self.display_hint = hint;
        self
    }

    /// 形状を設定
    #[inline]
    pub fn with_shape(mut self, shape: Vec<usize>) -> Self {
        self.shape = shape;
        self
    }

    /// NIL かどうか
    #[inline]
    pub fn is_nil(&self) -> bool {
        self.data.is_empty()
    }

    /// 真偽値として評価（空 = false、全てゼロ = false、それ以外 = true）
    #[inline]
    pub fn is_truthy(&self) -> bool {
        !self.data.is_empty() && !self.data.iter().all(|f| f.is_zero())
    }

    /// 長さを取得
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 空かどうか
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 最初の要素を取得
    #[inline]
    pub fn first(&self) -> Option<&Fraction> {
        self.data.first()
    }

    /// 単一要素の値かどうか
    #[inline]
    pub fn is_scalar(&self) -> bool {
        self.data.len() == 1
    }

    /// 単一の分数として取得（単一要素の場合のみ）
    #[inline]
    pub fn as_scalar(&self) -> Option<&Fraction> {
        if self.data.len() == 1 {
            Some(&self.data[0])
        } else {
            None
        }
    }

    /// i64として取得（単一の整数値の場合のみ）
    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        self.as_scalar().and_then(|f| f.to_i64())
    }

    /// usizeとして取得（単一の非負整数値の場合のみ）
    #[inline]
    pub fn as_usize(&self) -> Option<usize> {
        self.as_scalar().and_then(|f| f.as_usize())
    }
}

// ============================================================================
// 後方互換性のための型定義
// ============================================================================

/// トークン定義
///
/// パーサーが生成するトークンの種類を定義
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(String),
    String(String),
    Boolean(bool),
    Symbol(String),
    /// ベクタ開始 - [], {}, () 全てをこれで表現
    VectorStart,
    /// ベクタ終了
    VectorEnd,
    GuardSeparator,  // : または ;
    Nil,
    LineBreak,
}

/// ブラケットタイプ（表示専用）
///
/// 入力時はブラケットの種類を区別せず、表示時に深さに基づいて決定する
#[derive(Debug, Clone, PartialEq)]
pub enum BracketType {
    Square,  // []
    Curly,   // {}
    Round,   // ()
}

impl BracketType {
    pub fn opening_char(&self) -> char {
        match self {
            BracketType::Square => '[',
            BracketType::Curly => '{',
            BracketType::Round => '(',
        }
    }
    pub fn closing_char(&self) -> char {
        match self {
            BracketType::Square => ']',
            BracketType::Curly => '}',
            BracketType::Round => ')',
        }
    }

    /// 深さからブラケットタイプを決定
    /// depth 0 (1次元/可視最外殻): { }
    /// depth 1 (2次元): ( )
    /// depth 2 (3次元/可視限界): [ ]
    /// depth 3〜: サイクル
    pub fn from_depth(depth: usize) -> Self {
        match depth % 3 {
            0 => BracketType::Curly,   // 1次元: { }
            1 => BracketType::Round,   // 2次元: ( )
            2 => BracketType::Square,  // 3次元: [ ]
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionLine {
    pub body_tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct WordDefinition {
    pub lines: Vec<ExecutionLine>,
    pub is_builtin: bool,
    pub description: Option<String>,
    pub dependencies: HashSet<String>,
    pub original_source: Option<String>,
}

/// スタック型
pub type Stack = Vec<Value>;

/// 可視次元の最大値（後方互換性のため保持）
pub const MAX_VISIBLE_DIMENSIONS: usize = 3;

// ============================================================================
// 後方互換性のための ValueType（移行期間中のみ使用）
// ============================================================================

/// 後方互換性のためのValueType
///
/// 新しいコードではこの型を使用しないでください。
/// すべての値は Vec<Fraction> として表現されます。
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    Number(Fraction),
    Vector(Vec<Value>),
    String(String),
    Boolean(bool),
    Symbol(String),
    Nil,
    DateTime(Fraction),
}

impl Value {
    // ============================================================================
    // 後方互換性のためのメソッド（移行期間中のみ使用）
    // ============================================================================

    /// 後方互換性: val_type フィールドへのアクセス
    ///
    /// 新しいアーキテクチャでは data と display_hint を直接使用してください。
    pub fn val_type(&self) -> ValueType {
        if self.data.is_empty() {
            return ValueType::Nil;
        }

        match self.display_hint {
            DisplayHint::Boolean => {
                if self.data.len() == 1 {
                    ValueType::Boolean(!self.data[0].is_zero())
                } else {
                    // 複数要素の場合はベクタとして扱う
                    ValueType::Vector(self.data.iter().map(|f| Value::from_fraction(f.clone())).collect())
                }
            }
            DisplayHint::String => {
                // 分数のベクタを文字列に変換
                let chars: String = self.data.iter()
                    .filter_map(|f| {
                        f.to_i64().and_then(|n| {
                            if n >= 0 && n <= 0x10FFFF {
                                char::from_u32(n as u32)
                            } else {
                                None
                            }
                        })
                    })
                    .collect();
                ValueType::String(chars)
            }
            DisplayHint::DateTime => {
                if self.data.len() == 1 {
                    ValueType::DateTime(self.data[0].clone())
                } else {
                    ValueType::Vector(self.data.iter().map(|f| Value::from_fraction(f.clone())).collect())
                }
            }
            DisplayHint::Number | DisplayHint::Auto => {
                if self.data.len() == 1 {
                    ValueType::Number(self.data[0].clone())
                } else {
                    ValueType::Vector(self.data.iter().map(|f| Value::from_fraction(f.clone())).collect())
                }
            }
        }
    }

    /// 後方互換性: from_number
    #[inline]
    pub fn from_number(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    /// 後方互換性: from_vector
    ///
    /// 古いAPIとの互換性のため、Value のベクタを受け取ります。
    ///
    /// 統一分数アーキテクチャ:
    /// - 空ベクタ: NILを返す
    /// - スカラー要素のみ: 1Dベクタを作成（shape = [n]）
    /// - ベクタ要素: 次元を追加（shape = [n, inner_shape...]）
    ///
    /// スカラーは shape = [] として表現される。
    /// これにより `[ 1 ]` → shape [1]、`[ [ 1 ] ]` → shape [1, 1] と区別できる。
    pub fn from_vector(values: Vec<Value>) -> Self {
        if values.is_empty() {
            // 空ベクタ = NIL
            return Self::nil();
        }

        // すべての要素をフラット化しつつ形状情報を計算
        let inner_shape = values[0].shape.clone();
        let data: Vec<Fraction> = values.iter()
            .flat_map(|v| v.data.iter().cloned())
            .collect();

        // 空になった場合はNIL
        if data.is_empty() {
            return Self::nil();
        }

        // 新しい形状を計算: [要素数, 内部形状...]
        let mut new_shape = vec![values.len()];
        new_shape.extend(inner_shape);

        // 表示ヒントを継承（単一要素の場合のみ）
        let hint = if values.len() == 1 {
            values[0].display_hint
        } else {
            DisplayHint::Auto
        };

        Self {
            data,
            display_hint: hint,
            shape: new_shape,
        }
    }

    /// 後方互換性: from_datetime (Fraction版)
    #[inline]
    pub fn from_datetime_frac(f: Fraction) -> Self {
        Self::from_datetime(f)
    }
}
