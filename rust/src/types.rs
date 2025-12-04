// rust/src/types.rs

pub mod fraction;
pub mod tensor;

use std::collections::HashSet;
use std::fmt;
use num_bigint::BigInt;
use num_traits::One;
use self::fraction::Fraction;
use self::tensor::Tensor;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(String),
    String(String),
    Boolean(bool),
    Symbol(String),
    VectorStart(BracketType),
    VectorEnd(BracketType),
    GuardSeparator,  // : または ;
    Nil,
    LineBreak,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value {
    pub val_type: ValueType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    Number(Fraction),
    /// テンソル（N次元配列） - 次元モデルの新しい表現
    Tensor(Tensor),
    String(String),
    Boolean(bool),
    Symbol(String),
    // ブラケットタイプは表示層で深さから計算される
    // 注: Vectorは後方互換性のために残されています。段階的にTensorに移行します。
    Vector(Vec<Value>),
    Nil,
}

// Display トレイトの実装を追加
impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValueType::Number(_) => write!(f, "number"),
            ValueType::Tensor(_) => write!(f, "tensor"),
            ValueType::String(_) => write!(f, "string"),
            ValueType::Boolean(_) => write!(f, "boolean"),
            ValueType::Symbol(_) => write!(f, "symbol"),
            ValueType::Vector(_) => write!(f, "vector"),
            ValueType::Nil => write!(f, "nil"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BracketType {
    Square, Curly, Round,
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
    // ブラケットタイプは深さに基づいて計算される
    fn fmt_with_depth(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        match &self.val_type {
            ValueType::Number(n) => {
                if n.denominator == BigInt::one() {
                    write!(f, "{}", n.numerator)
                } else {
                    write!(f, "{}/{}", n.numerator, n.denominator)
                }
            }
            ValueType::Tensor(t) => {
                // テンソルの表示（形状に基づいて再帰的に表示）
                self.fmt_tensor_recursive(f, t, depth, 0, &mut 0)
            }
            ValueType::String(s) => write!(f, "'{}'", s),
            ValueType::Boolean(b) => write!(f, "{}", if *b { "TRUE" } else { "FALSE" }),
            ValueType::Symbol(s) => write!(f, "{}", s),
            ValueType::Vector(v) => {
                // 深さに基づいてブラケットタイプを決定
                let (open, close) = match depth % 3 {
                    0 => ('[', ']'),  // Square
                    1 => ('{', '}'),  // Curly
                    2 => ('(', ')'),  // Round
                    _ => unreachable!(),
                };
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

    // テンソルを再帰的にフォーマット
    fn fmt_tensor_recursive(
        &self,
        f: &mut fmt::Formatter<'_>,
        tensor: &Tensor,
        depth: usize,
        dim_index: usize,
        data_index: &mut usize,
    ) -> fmt::Result {
        let shape = tensor.shape();
        let data = tensor.data();

        // ブラケットタイプを決定
        let (open, close) = match depth % 3 {
            0 => ('[', ']'),  // Square
            1 => ('{', '}'),  // Curly
            2 => ('(', ')'),  // Round
            _ => unreachable!(),
        };

        if dim_index >= shape.len() {
            // 最内次元：数値を表示
            let n = &data[*data_index];
            *data_index += 1;
            if n.denominator == BigInt::one() {
                write!(f, "{}", n.numerator)
            } else {
                write!(f, "{}/{}", n.numerator, n.denominator)
            }
        } else {
            // 中間次元：再帰的に表示
            write!(f, "{}", open)?;
            let size = shape[dim_index];
            for i in 0..size {
                if i > 0 { write!(f, " ")?; }
                self.fmt_tensor_recursive(f, tensor, depth + 1, dim_index + 1, data_index)?;
            }
            write!(f, "{}", close)
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
    /// Tensorバリアントから Value を作成
    pub fn from_tensor(tensor: Tensor) -> Self {
        Value {
            val_type: ValueType::Tensor(tensor),
        }
    }

    /// Numberバリアントから Value を作成
    pub fn from_number(fraction: Fraction) -> Self {
        Value {
            val_type: ValueType::Number(fraction),
        }
    }

    /// Tensorへの参照を取得（Tensorバリアントの場合のみ）
    pub fn as_tensor(&self) -> std::result::Result<&Tensor, String> {
        match &self.val_type {
            ValueType::Tensor(t) => Ok(t),
            _ => Err(format!("Expected tensor, got {}", self.val_type)),
        }
    }

    /// Tensorへの可変参照を取得（Tensorバリアントの場合のみ）
    pub fn as_tensor_mut(&mut self) -> std::result::Result<&mut Tensor, String> {
        if let ValueType::Tensor(ref mut t) = self.val_type {
            Ok(t)
        } else {
            Err(format!("Expected tensor, got {}", self.val_type))
        }
    }

    /// Tensorを取り出す（所有権を移動）
    pub fn into_tensor(self) -> std::result::Result<Tensor, String> {
        match self.val_type {
            ValueType::Tensor(t) => Ok(t),
            _ => Err(format!("Expected tensor, got {}", self.val_type)),
        }
    }

    /// Stringへの参照を取得（Stringバリアントの場合のみ）
    pub fn as_string(&self) -> std::result::Result<&str, String> {
        match &self.val_type {
            ValueType::String(s) => Ok(s),
            _ => Err(format!("Expected string, got {}", self.val_type)),
        }
    }

    /// VectorからTensorに変換（互換性レイヤー）
    ///
    /// この関数は段階的な移行のために使用されます。
    /// すべての要素が数値であることを確認し、矩形制約を検証します。
    pub fn vector_to_tensor(vector: &[Value]) -> std::result::Result<Tensor, String> {
        // すべての要素が数値かどうかを確認
        let all_numbers = vector.iter().all(|v| matches!(v.val_type, ValueType::Number(_)));

        if all_numbers {
            // 1次元のテンソルとして変換
            let fractions: Vec<Fraction> = vector
                .iter()
                .filter_map(|v| {
                    if let ValueType::Number(ref f) = v.val_type {
                        Some(f.clone())
                    } else {
                        None
                    }
                })
                .collect();
            Ok(Tensor::vector(fractions))
        } else {
            // ネストされたベクタの可能性があるため、再帰的に変換
            Self::vector_to_tensor_recursive(vector)
        }
    }

    /// ネストされたVectorからTensorへの再帰的変換
    ///
    /// 矩形制約を検証しながら変換を行います。
    fn vector_to_tensor_recursive(vector: &[Value]) -> std::result::Result<Tensor, String> {
        if vector.is_empty() {
            return Ok(Tensor::vector(vec![]));
        }

        // 形状を検証
        let shape = validate_rectangular(vector)?;

        // データを平坦化して抽出
        let mut data = Vec::new();
        flatten_vector_data(vector, &mut data)?;

        Tensor::new(shape, data)
            .map_err(|e| format!("Failed to create tensor: {}", e))
    }
}

/// ネストされた構造が矩形かどうかを検証
///
/// 同一次元内のすべての要素が同じ形状であることを確認します。
pub fn validate_rectangular(values: &[Value]) -> std::result::Result<Vec<usize>, String> {
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
                "Non-rectangular tensor: element {} has shape {:?}, expected {:?}",
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
        ValueType::Tensor(t) => Ok(t.shape().to_vec()),
        ValueType::Vector(v) => {
            if v.is_empty() {
                Ok(vec![0])
            } else if v.iter().all(|x| matches!(x.val_type, ValueType::Number(_))) {
                // すべて数値なら1次元
                Ok(vec![v.len()])
            } else {
                // ネストされている場合は再帰的に確認
                let inner_shape = validate_rectangular(v)?;
                Ok(inner_shape)
            }
        }
        _ => Err(format!("Cannot get shape of {}", value.val_type)),
    }
}

/// Vectorのデータを再帰的に平坦化
fn flatten_vector_data(values: &[Value], output: &mut Vec<Fraction>) -> std::result::Result<(), String> {
    for val in values {
        match &val.val_type {
            ValueType::Number(ref f) => {
                output.push(f.clone());
            }
            ValueType::Vector(ref v) => {
                flatten_vector_data(v, output)?;
            }
            ValueType::Tensor(ref t) => {
                output.extend_from_slice(t.data());
            }
            _ => {
                return Err(format!("Cannot flatten {}", val.val_type));
            }
        }
    }
    Ok(())
}

pub type Stack = Vec<Value>;
