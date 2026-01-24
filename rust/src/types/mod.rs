// rust/src/types/mod.rs
//
// Ajisai 統一Value宇宙アーキテクチャ
//
// 公理1: 全てはValueである
// 公理2: Valueは他のValueを含むことができる
// 公理3: 操作は常に「現在のコンテキスト」に対して行われる
// 公理4: コンテキストの外側は存在しない
//
// ============================================================================
// 設計思想
// ============================================================================
//
// LISP: 全てはS式である
// Ajisai: 全てはValueである
//
// LISP: (1 2 (3 4) 5)
// Ajisai: { 1 2 { 3 4 } 5 }
//
// スタックもValueである。この公理から、全ての設計が導出される。
//
// ============================================================================
// 内部表現
// ============================================================================
//
// | ユーザー入力     | 内部表現                              | 表示         |
// |------------------|---------------------------------------|--------------|
// | 42               | Scalar(42/1)                          | 42           |
// | 1/3              | Scalar(1/3)                           | 1/3          |
// | TRUE             | Scalar(1/1) with Boolean hint         | TRUE         |
// | FALSE            | Scalar(0/1) with Boolean hint         | FALSE        |
// | NIL              | Nil                                   | NIL          |
// | 'Hello'          | Vector([72, 101, 108, 108, 111])      | 'Hello'      |
// | [ 1 2 3 ]        | Vector([Scalar(1), Scalar(2), ...])   | { 1 2 3 }    |
// | [ 1 [ 2 3 ] 4 ]  | Vector([Scalar(1), Vector(...), ...]) | { 1 { 2 3 } 4 } |

pub mod fraction;
pub mod display;
pub mod tensor;  // 行列演算ユーティリティ

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
    /// NIL（空値）を示す
    Nil,
}

/// Valueのデータ本体（再帰的定義）
///
/// LISPのcons cellに対応する構造。
/// これにより、任意の深さのネスト構造を自然に表現できる。
#[derive(Debug, Clone, PartialEq)]
pub enum ValueData {
    /// スカラー値（単一の分数）
    Scalar(Fraction),
    /// ベクター値（Valueの配列）- 再帰的にValueを含む
    Vector(Vec<Value>),
    /// NIL（空）
    Nil,
}

/// Ajisai の唯一の値型（再帰的定義）
///
/// すべてのデータはこの構造体で表現される。
/// スタックもValueであり、ネストしたValueも表現できる。
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    /// データ本体
    pub data: ValueData,
    /// 表示ヒント（演算には使用しない）
    pub display_hint: DisplayHint,
}

impl Value {
    // === コンストラクタ ===

    /// NIL値を作成
    #[inline]
    pub fn nil() -> Self {
        Self {
            data: ValueData::Nil,
            display_hint: DisplayHint::Nil,
        }
    }

    /// 単一の分数から値を作成（スカラー）
    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
            display_hint: DisplayHint::Number,
        }
    }

    /// 整数から値を作成（スカラー）
    #[inline]
    pub fn from_int(n: i64) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(n)),
            display_hint: DisplayHint::Number,
        }
    }

    /// 真偽値から値を作成（スカラー）
    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(if b { 1 } else { 0 })),
            display_hint: DisplayHint::Boolean,
        }
    }

    /// 文字列から値を作成（各文字をスカラーとして持つVector）
    pub fn from_string(s: &str) -> Self {
        let children: Vec<Value> = s.bytes()
            .map(|b| Value::from_int(b as i64))
            .collect();

        if children.is_empty() {
            return Self {
                data: ValueData::Nil,
                display_hint: DisplayHint::String,
            };
        }

        Self {
            data: ValueData::Vector(children),
            display_hint: DisplayHint::String,
        }
    }

    /// シンボルから値を作成（文字列として格納）
    pub fn from_symbol(s: &str) -> Self {
        Self::from_string(s)
    }

    /// 空のベクターを作成
    #[inline]
    pub fn empty_vector() -> Self {
        Self {
            data: ValueData::Vector(Vec::new()),
            display_hint: DisplayHint::Auto,
        }
    }

    /// 子Valueの配列からValueを作成
    #[inline]
    pub fn from_children(children: Vec<Value>) -> Self {
        Self {
            data: ValueData::Vector(children),
            display_hint: DisplayHint::Auto,
        }
    }

    /// Value のベクタから値を作成（from_vectorのエイリアス）
    pub fn from_vector(values: Vec<Value>) -> Self {
        if values.is_empty() {
            return Self::nil();
        }

        // ベクターは常に Auto として扱う
        // 内部要素の display_hint は保持されるため、
        // 表示時に適切に処理される
        Self {
            data: ValueData::Vector(values),
            display_hint: DisplayHint::Auto,
        }
    }

    /// 数値（単一の分数）から値を作成（from_fractionのエイリアス）
    #[inline]
    pub fn from_number(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    /// DateTimeとして値を作成（Unixタイムスタンプ、スカラー）
    #[inline]
    pub fn from_datetime(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
            display_hint: DisplayHint::DateTime,
        }
    }

    /// 表示ヒントを設定
    #[inline]
    pub fn with_hint(mut self, hint: DisplayHint) -> Self {
        self.display_hint = hint;
        self
    }

    // === 判定メソッド ===

    /// NIL かどうか
    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    /// スカラー値かどうか
    #[inline]
    pub fn is_scalar(&self) -> bool {
        matches!(self.data, ValueData::Scalar(_))
    }

    /// ベクター値かどうか
    #[inline]
    pub fn is_vector(&self) -> bool {
        matches!(self.data, ValueData::Vector(_))
    }

    /// 単一要素の値かどうか（スカラーの場合true）
    #[inline]
    pub fn is_single(&self) -> bool {
        self.is_scalar()
    }

    /// 真偽値として評価
    /// NIL = false、ゼロ = false、空Vector = false、それ以外 = true
    #[inline]
    pub fn is_truthy(&self) -> bool {
        match &self.data {
            ValueData::Nil => false,
            ValueData::Scalar(f) => !f.is_zero() && !f.is_nil(),
            ValueData::Vector(v) => !v.is_empty() && !v.iter().all(|c| !c.is_truthy()),
        }
    }

    // === 長さ・アクセス ===

    /// 長さを取得
    /// - Nil: 0
    /// - Scalar: 1
    /// - Vector: 子の数
    #[inline]
    pub fn len(&self) -> usize {
        match &self.data {
            ValueData::Nil => 0,
            ValueData::Scalar(_) => 1,
            ValueData::Vector(v) => v.len(),
        }
    }

    /// 空かどうか
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 子Valueを取得
    pub fn get_child(&self, index: usize) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) => v.get(index),
            ValueData::Scalar(_) if index == 0 => Some(self),
            _ => None,
        }
    }

    /// 子Valueを可変で取得
    pub fn get_child_mut(&mut self, index: usize) -> Option<&mut Value> {
        match &mut self.data {
            ValueData::Vector(v) => v.get_mut(index),
            _ => None,
        }
    }

    /// 最初の子を取得
    #[inline]
    pub fn first(&self) -> Option<&Value> {
        self.get_child(0)
    }

    /// 最後の子を取得
    #[inline]
    pub fn last(&self) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) => v.last(),
            ValueData::Scalar(_) => Some(self),
            ValueData::Nil => None,
        }
    }

    // === 操作メソッド ===

    /// 子を末尾に追加
    pub fn push_child(&mut self, child: Value) {
        match &mut self.data {
            ValueData::Vector(v) => v.push(child),
            ValueData::Nil => {
                // NIL を Vector に昇格
                self.data = ValueData::Vector(vec![child]);
                self.display_hint = DisplayHint::Auto;
            }
            ValueData::Scalar(f) => {
                // Scalar を Vector に昇格
                let old = Value::from_fraction(f.clone());
                self.data = ValueData::Vector(vec![old, child]);
                self.display_hint = DisplayHint::Auto;
            }
        }
    }

    /// 末尾の子を取り出し
    pub fn pop_child(&mut self) -> Option<Value> {
        match &mut self.data {
            ValueData::Vector(v) => v.pop(),
            _ => None,
        }
    }

    /// 指定位置に子を挿入
    pub fn insert_child(&mut self, index: usize, child: Value) {
        if let ValueData::Vector(v) = &mut self.data {
            if index <= v.len() {
                v.insert(index, child);
            }
        }
    }

    /// 指定位置の子を削除
    pub fn remove_child(&mut self, index: usize) -> Option<Value> {
        if let ValueData::Vector(v) = &mut self.data {
            if index < v.len() {
                return Some(v.remove(index));
            }
        }
        None
    }

    /// 指定位置の子を置換
    pub fn replace_child(&mut self, index: usize, child: Value) -> Option<Value> {
        if let ValueData::Vector(v) = &mut self.data {
            if index < v.len() {
                return Some(std::mem::replace(&mut v[index], child));
            }
        }
        None
    }

    // === スカラー値へのアクセス ===

    /// スカラー値を取得
    #[inline]
    pub fn as_scalar(&self) -> Option<&Fraction> {
        match &self.data {
            ValueData::Scalar(f) => Some(f),
            _ => None,
        }
    }

    /// スカラー値を可変で取得
    #[inline]
    pub fn as_scalar_mut(&mut self) -> Option<&mut Fraction> {
        match &mut self.data {
            ValueData::Scalar(f) => Some(f),
            _ => None,
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

    // === ベクターへのアクセス ===

    /// 子のベクターを取得
    #[inline]
    pub fn as_vector(&self) -> Option<&Vec<Value>> {
        match &self.data {
            ValueData::Vector(v) => Some(v),
            _ => None,
        }
    }

    /// 子のベクターを可変で取得
    #[inline]
    pub fn as_vector_mut(&mut self) -> Option<&mut Vec<Value>> {
        match &mut self.data {
            ValueData::Vector(v) => Some(v),
            _ => None,
        }
    }

    /// イテレータを取得（子Valueを走査）
    pub fn iter(&self) -> ValueIter<'_> {
        ValueIter {
            value: self,
            index: 0,
        }
    }

    // === 互換性のためのメソッド ===

    /// 全ての分数を平坦化して取得（互換性のため）
    pub fn flatten_fractions(&self) -> Vec<Fraction> {
        match &self.data {
            ValueData::Nil => vec![Fraction::nil()],
            ValueData::Scalar(f) => vec![f.clone()],
            ValueData::Vector(v) => {
                v.iter().flat_map(|c| c.flatten_fractions()).collect()
            }
        }
    }

    /// 形状情報を取得（互換性のため）
    pub fn shape(&self) -> Vec<usize> {
        match &self.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(_) => vec![],
            ValueData::Vector(v) => {
                if v.is_empty() {
                    vec![0]
                } else {
                    // 同質なベクターの場合のみ形状を計算
                    let first_shape = v[0].shape();
                    let all_same = v.iter().skip(1).all(|c| c.shape() == first_shape);
                    if all_same && !first_shape.is_empty() {
                        let mut shape = vec![v.len()];
                        shape.extend(first_shape);
                        shape
                    } else {
                        vec![v.len()]
                    }
                }
            }
        }
    }

    /// 分数配列とヒントから作成（互換性のため）
    #[allow(dead_code)]
    pub fn from_fractions_with_shape(data: Vec<Fraction>, _shape: Vec<usize>, hint: DisplayHint) -> Self {
        if data.is_empty() {
            return Self::nil();
        }
        if data.len() == 1 {
            if data[0].is_nil() && hint == DisplayHint::Nil {
                return Self::nil();
            }
            return Self {
                data: ValueData::Scalar(data[0].clone()),
                display_hint: hint,
            };
        }
        Self {
            data: ValueData::Vector(data.into_iter().map(Value::from_fraction).collect()),
            display_hint: hint,
        }
    }

    /// 分数の配列から値を作成（数値ヒント付き）
    #[inline]
    pub fn from_numbers(v: Vec<Fraction>) -> Self {
        if v.is_empty() {
            return Self::nil();
        }
        if v.len() == 1 {
            return Self {
                data: ValueData::Scalar(v[0].clone()),
                display_hint: DisplayHint::Number,
            };
        }
        Self {
            data: ValueData::Vector(v.into_iter().map(Value::from_fraction).collect()),
            display_hint: DisplayHint::Number,
        }
    }

    /// 分数の配列から値を作成
    #[inline]
    pub fn from_vec(v: Vec<Fraction>) -> Self {
        if v.is_empty() {
            return Self::nil();
        }
        if v.len() == 1 {
            return Self {
                data: ValueData::Scalar(v[0].clone()),
                display_hint: DisplayHint::Auto,
            };
        }
        Self {
            data: ValueData::Vector(v.into_iter().map(Value::from_fraction).collect()),
            display_hint: DisplayHint::Auto,
        }
    }
}

/// Value のイテレータ
pub struct ValueIter<'a> {
    value: &'a Value,
    index: usize,
}

impl<'a> Iterator for ValueIter<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.value.get_child(self.index);
        if result.is_some() {
            self.index += 1;
        }
        result
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

/// スタック型（旧型との互換性のため残す）
pub type Stack = Vec<Value>;

/// 可視次元の最大値
pub const MAX_VISIBLE_DIMENSIONS: usize = 3;
