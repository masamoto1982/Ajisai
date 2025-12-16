// rust/src/types.rs
//
// Vector指向型システム
// 全てのコンテナデータはVectorで表現し、Fractionによる正確な有理数演算を維持する

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
    pub fn from_depth(depth: usize) -> Self {
        match depth % 3 {
            0 => BracketType::Square,
            1 => BracketType::Curly,
            2 => BracketType::Round,
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

    /// 数値を単一要素Vectorでラップして作成
    pub fn wrap_number(fraction: Fraction) -> Self {
        Value {
            val_type: ValueType::Vector(vec![Value::from_number(fraction)]),
        }
    }

    /// Vectorへの参照を取得（Vectorバリアントの場合のみ）
    pub fn as_vector(&self) -> std::result::Result<&Vec<Value>, String> {
        match &self.val_type {
            ValueType::Vector(v) => Ok(v),
            _ => Err(format!("Expected vector, got {}", self.val_type)),
        }
    }

    /// Vectorへの可変参照を取得（Vectorバリアントの場合のみ）
    pub fn as_vector_mut(&mut self) -> std::result::Result<&mut Vec<Value>, String> {
        if let ValueType::Vector(ref mut v) = self.val_type {
            Ok(v)
        } else {
            Err(format!("Expected vector, got {}", self.val_type))
        }
    }

    /// Vectorを取り出す（所有権を移動）
    pub fn into_vector(self) -> std::result::Result<Vec<Value>, String> {
        match self.val_type {
            ValueType::Vector(v) => Ok(v),
            _ => Err(format!("Expected vector, got {}", self.val_type)),
        }
    }

    /// Stringへの参照を取得（Stringバリアントの場合のみ）
    pub fn as_string(&self) -> std::result::Result<&str, String> {
        match &self.val_type {
            ValueType::String(s) => Ok(s),
            _ => Err(format!("Expected string, got {}", self.val_type)),
        }
    }

    /// 数値への参照を取得（Numberバリアントの場合のみ）
    pub fn as_number(&self) -> std::result::Result<&Fraction, String> {
        match &self.val_type {
            ValueType::Number(n) => Ok(n),
            _ => Err(format!("Expected number, got {}", self.val_type)),
        }
    }
}

/// Vectorから形状を推論する
///
/// ネストされたVectorの形状を再帰的に計算
pub fn infer_shape(values: &[Value]) -> std::result::Result<Vec<usize>, String> {
    if values.is_empty() {
        return Ok(vec![0]);
    }

    // 最初の要素の形状を基準とする
    let first_shape = get_value_shape(&values[0])?;

    // すべての要素が同じ形状か検証
    for (i, val) in values.iter().enumerate().skip(1) {
        let shape = get_value_shape(val)?;
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
    Ok(full_shape)
}

/// Valueの形状を取得
fn get_value_shape(value: &Value) -> std::result::Result<Vec<usize>, String> {
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
                infer_shape(v)
            }
        }
        _ => Err(format!("Cannot get shape of {}", value.val_type)),
    }
}

/// 矩形かどうかを検証
///
/// 同一次元内のすべての要素が同じ形状であることを確認
pub fn is_rectangular(values: &[Value]) -> bool {
    infer_shape(values).is_ok()
}

/// すべての要素が数値かチェック
pub fn all_numbers(values: &[Value]) -> bool {
    values.iter().all(|v| {
        match &v.val_type {
            ValueType::Number(_) => true,
            ValueType::Vector(inner) => all_numbers(inner),
            _ => false,
        }
    })
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

/// 形状とデータからネストされたVectorを構築
pub fn build_nested_vector(shape: &[usize], data: &[Fraction]) -> std::result::Result<Value, String> {
    if shape.is_empty() {
        // スカラー
        if data.len() != 1 {
            return Err("Scalar requires exactly one data element".to_string());
        }
        return Ok(Value::from_number(data[0].clone()));
    }

    let expected_size: usize = shape.iter().product();
    if data.len() != expected_size {
        return Err(format!(
            "Shape {:?} requires {} elements, but got {}",
            shape, expected_size, data.len()
        ));
    }

    if shape.len() == 1 {
        // 1次元
        let values: Vec<Value> = data.iter()
            .map(|f| Value::from_number(f.clone()))
            .collect();
        return Ok(Value::from_vector(values));
    }

    // 多次元
    let outer_size = shape[0];
    let inner_shape = &shape[1..];
    let inner_size: usize = inner_shape.iter().product();

    let mut values = Vec::with_capacity(outer_size);
    for i in 0..outer_size {
        let start = i * inner_size;
        let inner_data = &data[start..start + inner_size];
        values.push(build_nested_vector(inner_shape, inner_data)?);
    }

    Ok(Value::from_vector(values))
}

pub type Stack = Vec<Value>;
