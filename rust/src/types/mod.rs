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
/// 唯一の例外はNil: これはNIL値であることを示す特別なマーカー。
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
    /// NIL（空値）を示す
    Nil,
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

// 将来のワード実装で使用されるユーティリティメソッド群
#[allow(dead_code)]
impl Value {
    /// NIL値を作成
    ///
    /// NILは空のVectorとは異なる概念。
    /// DisplayHint::Nilでマークされる特別な値。
    #[inline]
    pub fn nil() -> Self {
        Self {
            data: Vec::new(),
            display_hint: DisplayHint::Nil,
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

    /// 数値（単一の分数）から値を作成
    #[inline]
    pub fn from_number(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    /// Value のベクタから値を作成
    ///
    /// 統一分数アーキテクチャ:
    /// - スカラー要素のみ: 1Dベクタを作成（shape = [n]）
    /// - ベクタ要素: 次元を追加（shape = [n, inner_shape...]）
    ///
    /// スカラーは shape = [] として表現される。
    /// これにより `[ 1 ]` → shape [1]、`[ [ 1 ] ]` → shape [1, 1] と区別できる。
    ///
    /// # 空のベクタの扱い
    /// 空のベクタは自動的にNIL値として返される。
    /// これは「空のスタック = NIL」という設計思想に基づく。
    pub fn from_vector(values: Vec<Value>) -> Self {
        // 空のベクタはNILとして扱う（Stack -> Value変換などで安全になる）
        if values.is_empty() {
            return Self::nil();
        }

        // 要素からデータを収集（NILは空のデータを持つ）
        let inner_shape = values[0].shape.clone();
        let data: Vec<Fraction> = values.iter()
            .flat_map(|v| v.data.iter().cloned())
            .collect();

        // 新しい形状を計算: [要素数, 内部形状...]
        let mut new_shape = vec![values.len()];
        new_shape.extend(inner_shape);

        // 表示ヒントを継承（同種の要素からなるベクタの場合）
        // 全ての要素が同じDisplayHintを持つ場合、そのヒントを継承
        // ただし、Nil/Autoは継承対象外
        let hint = {
            let first_hint = values[0].display_hint;

            // Nil や Auto は継承しない
            if first_hint == DisplayHint::Nil || first_hint == DisplayHint::Auto {
                DisplayHint::Auto
            } else {
                // 全ての要素が同じヒントを持つかチェック
                let all_same_hint = values.iter().all(|v| v.display_hint == first_hint);
                if all_same_hint {
                    first_hint
                } else {
                    DisplayHint::Auto
                }
            }
        };

        Self {
            data,
            display_hint: hint,
            shape: new_shape,
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
    ///
    /// DisplayHint::Nilでマークされた値のみがNIL。
    /// 空のVectorは許容されない（作成時にエラー）。
    #[inline]
    pub fn is_nil(&self) -> bool {
        self.display_hint == DisplayHint::Nil
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

    // ========================================================================
    // スタック操作用メソッド
    // 「スタック = Value（Vector）」という統一モデルのためのヘルパー
    // ========================================================================

    /// スタックとしての要素数を取得
    ///
    /// NILの場合は0、それ以外はshapeの最初の要素（または1）を返す
    #[inline]
    pub fn stack_len(&self) -> usize {
        if self.is_nil() {
            0
        } else if self.shape.is_empty() {
            // スカラーは1要素
            1
        } else {
            self.shape[0]
        }
    }

    /// スタックの要素をVec<Value>として再構築
    ///
    /// 内部のデータを個別のValueとして取り出す。
    /// NILの場合は空のVecを返す。
    pub fn to_stack_elements(&self) -> Vec<Value> {
        if self.is_nil() {
            return Vec::new();
        }

        if self.shape.is_empty() {
            // スカラーは1要素のベクタとして扱う
            vec![self.clone()]
        } else if self.shape.len() == 1 {
            let outer_size = self.shape[0];

            // 1次元でdataが空の場合、outer_size個のNILを返す
            // （NILを含むベクタを正しく再構築）
            if self.data.is_empty() {
                return (0..outer_size).map(|_| Value::nil()).collect();
            }

            // 1次元: 各分数を個別のValueとして返す
            self.data.iter().map(|f| Value::from_fraction(f.clone())).collect()
        } else {
            // 多次元: 最外層の要素を再構築
            let outer_size = self.shape[0];
            let inner_size: usize = self.shape[1..].iter().product();
            let inner_shape = self.shape[1..].to_vec();

            // 内部要素がNIL（inner_size = 0またはdata不足）の場合
            if inner_size == 0 || self.data.is_empty() {
                return (0..outer_size).map(|_| Value::nil()).collect();
            }

            (0..outer_size).map(|i| {
                let start = i * inner_size;
                let end = start + inner_size;
                // データ範囲外の場合はNILを返す
                if end > self.data.len() {
                    return Value::nil();
                }
                let data = self.data[start..end].to_vec();
                Value {
                    data,
                    display_hint: self.display_hint,
                    shape: inner_shape.clone(),
                }
            }).collect()
        }
    }

    /// スタックの最後の要素を取得（参照）
    ///
    /// NILまたは空の場合はNoneを返す
    pub fn stack_last(&self) -> Option<Value> {
        if self.is_nil() {
            return None;
        }

        let elements = self.to_stack_elements();
        elements.last().cloned()
    }

    /// スタックに要素を追加した新しいValueを作成
    ///
    /// 現在のスタック（self）に新しい要素を追加した結果を返す。
    /// selfは変更されない（イミュータブル操作）。
    pub fn stack_with_push(&self, value: Value) -> Value {
        if self.is_nil() {
            // 空スタックへの追加 → 新しい値を含む1要素のVector
            Value::from_vector(vec![value])
        } else {
            // 既存の要素と新しい値を結合
            let mut elements = self.to_stack_elements();
            elements.push(value);
            Value::from_vector(elements)
        }
    }

    /// スタックから最後の要素を取り除いた新しいValueと、取り除いた要素を返す
    ///
    /// NILまたは空の場合は(self.clone(), None)を返す。
    /// selfは変更されない（イミュータブル操作）。
    pub fn stack_with_pop(&self) -> (Value, Option<Value>) {
        if self.is_nil() {
            return (self.clone(), None);
        }

        let mut elements = self.to_stack_elements();
        let popped = elements.pop();
        let new_stack = Value::from_vector(elements); // 空ならNILになる
        (new_stack, popped)
    }
}

// ============================================================================
// トークンとパーサー関連の型定義
// ============================================================================

/// トークン定義
///
/// パーサーが生成するトークンの種類を定義
///
/// TRUE/FALSE/NILは組み込みワードとして実装されるため、
/// Symbolとして扱われ、インタープリタで処理される
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(String),
    String(String),
    Symbol(String),  // TRUE, FALSE, NIL もここに含まれる
    /// ベクタ開始 - [], {}, () 全てをこれで表現
    VectorStart,
    /// ベクタ終了
    VectorEnd,
    GuardSeparator,  // : または ;
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

/// 可視次元の最大値
pub const MAX_VISIBLE_DIMENSIONS: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_vector_preserves_number_hint() {
        // 全てNumber要素からなるベクタはNumberヒントを継承
        let values: Vec<Value> = vec![
            Value::from_int(65),
            Value::from_int(66),
        ];
        let result = Value::from_vector(values);
        assert_eq!(result.display_hint, DisplayHint::Number);
        assert_eq!(result.data.len(), 2);
    }

    #[test]
    fn test_from_vector_preserves_boolean_hint() {
        // 全てBoolean要素からなるベクタはBooleanヒントを継承
        let values: Vec<Value> = vec![
            Value::from_bool(true),
            Value::from_bool(false),
        ];
        let result = Value::from_vector(values);
        assert_eq!(result.display_hint, DisplayHint::Boolean);
    }

    #[test]
    fn test_from_vector_preserves_string_hint() {
        // 全てString要素からなるベクタはStringヒントを継承
        let values: Vec<Value> = vec![
            Value::from_string("a"),
            Value::from_string("b"),
        ];
        let result = Value::from_vector(values);
        assert_eq!(result.display_hint, DisplayHint::String);
    }

    #[test]
    fn test_from_vector_mixed_hints_uses_auto() {
        // 異なるヒントの要素が混在する場合はAutoになる
        let values: Vec<Value> = vec![
            Value::from_int(42),
            Value::from_string("hello"),
        ];
        let result = Value::from_vector(values);
        assert_eq!(result.display_hint, DisplayHint::Auto);
    }

    #[test]
    fn test_from_vector_single_element_preserves_hint() {
        // 単一要素の場合もヒントを継承
        let values: Vec<Value> = vec![Value::from_int(42)];
        let result = Value::from_vector(values);
        assert_eq!(result.display_hint, DisplayHint::Number);
    }

    #[test]
    fn test_from_vector_nil_elements_use_auto() {
        // Nil要素を含む場合はAutoになる
        let values: Vec<Value> = vec![Value::nil()];
        let result = Value::from_vector(values);
        assert_eq!(result.display_hint, DisplayHint::Auto);
    }

    #[test]
    fn test_number_vector_displays_as_numbers() {
        // 数値配列は数値として表示される（文字列化されない）
        let values: Vec<Value> = vec![
            Value::from_int(65),
            Value::from_int(66),
        ];
        let result = Value::from_vector(values);
        let display = format!("{}", result);
        // 65, 66 は 'AB' ではなく数値として表示される
        assert!(display.contains("65"));
        assert!(display.contains("66"));
        assert!(!display.contains("'"));  // 文字列クオートがないことを確認
    }

    #[test]
    fn test_from_vector_empty_returns_nil() {
        // 空のベクタはNILを返す（「空のスタック = NIL」）
        let values: Vec<Value> = vec![];
        let result = Value::from_vector(values);
        assert!(result.is_nil(), "Empty vector should return NIL");
        assert_eq!(result.display_hint, DisplayHint::Nil);
        assert!(result.data.is_empty());
    }
}
