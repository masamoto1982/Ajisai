// rust/src/types.rs
//
// Vector指向型システム
// 全てのコンテナデータはVectorで表現し、Fractionによる正確な有理数演算を維持する
//
// ============================================================================
// 層構造の設計思想（フラクタル構造としてのVector）
// ============================================================================
//
// Ajisaiでは「全てがVector」というフラクタル構造を採用している。
// スタック自体もVectorであり、その中にVectorが積まれる。
// これはLISPのリスト構造に通ずる美学を表現している。
//
// 次元構造（0次元を含めて4次元、可視は3次元まで）:
//   0次元: スタック（GUIの枠そのもの、不可視）
//   1次元: { } で表示（可視の最外殻）
//   2次元: ( ) で表示
//   3次元: [ ] で表示（可視の限界）
//   4次元〜: エラー
//
// Tensorとの違い:
//   Tensor: 数値専用、同種データのみ許容
//   Vector: 異種データ混在可能 [1 'hello' TRUE [2 3]]
//
// この異種混在の許容が、VectorをLISP的なリスト構造として機能させる鍵である。
// ============================================================================

/// 可視次元の最大数（0次元のスタックを除く）
/// 0次元: スタック（不可視、GUIの枠）
/// 1次元: { } - 最外殻
/// 2次元: ( )
/// 3次元: [ ] - 可視限界
pub const MAX_VISIBLE_DIMENSIONS: usize = 3;

pub mod fraction;
pub mod tensor;  // 行列演算ユーティリティ（Vectorベースで動作）

use std::collections::HashSet;
use std::fmt;
use num_bigint::BigInt;
use num_traits::One;
use self::fraction::Fraction;

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

/// 値の型定義
///
/// Ajisaiの全ての値はこの型で表現される
#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub val_type: ValueType,
}

/// 値の種類
///
/// - Number: 有理数（Fraction）
/// - Vector: 値の配列（再帰的にネスト可能、異種型混合可能）
/// - String: 文字列
/// - Boolean: 真偽値
/// - Symbol: シンボル
/// - Nil: 空値
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    /// 数値（有理数）
    Number(Fraction),
    /// ベクタ（Valueの配列、再帰的にネスト可能）
    Vector(Vec<Value>),
    /// 文字列
    String(String),
    /// 真偽値
    Boolean(bool),
    /// シンボル
    Symbol(String),
    /// 空値
    Nil,
    /// 日時（Unixタイムスタンプ、内部的にはFraction）
    /// 表示時は年月日時刻形式でフォーマットされる
    DateTime(Fraction),
}

// Display トレイトの実装
impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::Number(_) => write!(f, "number"),
            ValueType::String(_) => write!(f, "string"),
            ValueType::Boolean(_) => write!(f, "boolean"),
            ValueType::Symbol(_) => write!(f, "symbol"),
            ValueType::Vector(_) => write!(f, "vector"),
            ValueType::Nil => write!(f, "nil"),
            ValueType::DateTime(_) => write!(f, "datetime"),
        }
    }
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

impl Value {
    /// 深さに基づいてブラケットタイプを決定してフォーマット
    fn fmt_with_depth(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        match &self.val_type {
            ValueType::Number(n) => {
                if n.denominator == BigInt::one() {
                    write!(f, "{}", n.numerator)
                } else {
                    write!(f, "{}/{}", n.numerator, n.denominator)
                }
            }
            ValueType::String(s) => write!(f, "'{}'", s),
            ValueType::Boolean(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            ValueType::Symbol(s) => write!(f, "{}", s),
            ValueType::Vector(v) => {
                // 深さに基づいてブラケットタイプを決定
                let bracket = BracketType::from_depth(depth);
                let (open, close) = (bracket.opening_char(), bracket.closing_char());
                write!(f, "{}", open)?;
                for (i, item) in v.iter().enumerate() {
                    if i > 0 { write!(f, " ")?; }
                    item.fmt_with_depth(f, depth + 1)?;
                }
                write!(f, "{}", close)
            }
            ValueType::Nil => write!(f, "NIL"),
            ValueType::DateTime(n) => {
                // DateTime型は@プレフィックスで表示
                // JavaScript側で詳細な日時フォーマットを行う
                if n.denominator == BigInt::one() {
                    write!(f, "@{}", n.numerator)
                } else {
                    write!(f, "@{}/{}", n.numerator, n.denominator)
                }
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_depth(f, 0)
    }
}

// Valueの便利メソッド
impl Value {
    /// Numberバリアントから Value を作成
    pub fn from_number(fraction: Fraction) -> Self {
        Value {
            val_type: ValueType::Number(fraction),
        }
    }

    /// Vectorバリアントから Value を作成
    pub fn from_vector(values: Vec<Value>) -> Self {
        Value {
            val_type: ValueType::Vector(values),
        }
    }

    /// DateTimeバリアントから Value を作成
    pub fn from_datetime(fraction: Fraction) -> Self {
        Value {
            val_type: ValueType::DateTime(fraction),
        }
    }
}

/// Vectorから形状を推論する
///
/// ネストされたVectorの形状を再帰的に計算
/// 4次元を超える場合はエラーを返す
pub fn infer_shape(values: &[Value]) -> std::result::Result<Vec<usize>, String> {
    infer_shape_with_depth(values, 1)
}

/// 深さを追跡しながら形状を推論（内部関数）
fn infer_shape_with_depth(values: &[Value], current_depth: usize) -> std::result::Result<Vec<usize>, String> {
    // 次元数チェック
    if current_depth > MAX_VISIBLE_DIMENSIONS {
        return Err(format!(
            "Dimension limit exceeded: Ajisai supports up to 3 visible dimensions (plus dimension 0: the stack). Nesting depth {} exceeds the limit.",
            current_depth
        ));
    }

    if values.is_empty() {
        return Ok(vec![0]);
    }

    // 最初の要素の形状を基準とする
    let first_shape = get_value_shape_with_depth(&values[0], current_depth)?;

    // すべての要素が同じ形状か検証
    for (i, val) in values.iter().enumerate().skip(1) {
        let shape = get_value_shape_with_depth(val, current_depth)?;
        if shape != first_shape {
            return Err(format!(
                "Non-rectangular structure: element {} has shape {:?}, expected {:?}",
                i, shape, first_shape
            ));
        }
    }

    // 全体の形状を構築
    let mut full_shape = vec![values.len()];
    full_shape.extend(first_shape);

    // 最終的な次元数をチェック
    if full_shape.len() > MAX_VISIBLE_DIMENSIONS {
        return Err(format!(
            "Dimension limit exceeded: Ajisai supports up to 3 visible dimensions (plus dimension 0: the stack). Nesting depth {} exceeds the limit.",
            full_shape.len()
        ));
    }

    Ok(full_shape)
}

/// 深さを追跡しながらValueの形状を取得（内部関数）
fn get_value_shape_with_depth(value: &Value, current_depth: usize) -> std::result::Result<Vec<usize>, String> {
    // 次元数チェック
    if current_depth > MAX_VISIBLE_DIMENSIONS {
        return Err(format!(
            "Dimension limit exceeded: Ajisai supports up to 3 visible dimensions (plus dimension 0: the stack). Nesting depth {} exceeds the limit.",
            current_depth
        ));
    }

    match &value.val_type {
        ValueType::Number(_) => Ok(vec![]),  // スカラー
        ValueType::Vector(v) => {
            if v.is_empty() {
                Ok(vec![0])
            } else if v.iter().all(|x| matches!(x.val_type, ValueType::Number(_))) {
                // すべて数値なら1次元
                Ok(vec![v.len()])
            } else {
                // ネストされている場合は再帰的に確認
                infer_shape_with_depth(v, current_depth + 1)
            }
        }
        _ => Err(format!("Cannot get shape of {}", value.val_type)),
    }
}

/// Vectorから数値を平坦化して抽出
pub fn flatten_numbers(values: &[Value]) -> std::result::Result<Vec<Fraction>, String> {
    let mut output = Vec::new();
    flatten_numbers_recursive(values, &mut output)?;
    Ok(output)
}

fn flatten_numbers_recursive(values: &[Value], output: &mut Vec<Fraction>) -> std::result::Result<(), String> {
    for val in values {
        match &val.val_type {
            ValueType::Number(ref f) => {
                output.push(f.clone());
            }
            ValueType::Vector(ref v) => {
                flatten_numbers_recursive(v, output)?;
            }
            _ => {
                return Err(format!("Cannot flatten {}", val.val_type));
            }
        }
    }
    Ok(())
}

pub type Stack = Vec<Value>;
