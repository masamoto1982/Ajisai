// rust/src/types/display.rs
//
// 値の表示ロジック
//
// DisplayHint に基づいて、または自動判定で適切な形式に変換する。
// 深さに応じた括弧を使用する。

use super::{Value, ValueData, DisplayHint, BracketType};
use super::fraction::Fraction;
use std::fmt;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.display_hint {
            DisplayHint::Nil => {
                if matches!(self.data, ValueData::Nil) {
                    write!(f, "NIL")
                } else {
                    write!(f, "{}", display_value(&self.data, 0))
                }
            }
            DisplayHint::Auto => write!(f, "{}", auto_display(&self.data)),
            DisplayHint::Number => write!(f, "{}", display_value(&self.data, 0)),
            DisplayHint::String => write!(f, "{}", display_as_string(&self.data)),
            DisplayHint::Boolean => write!(f, "{}", display_as_boolean(&self.data)),
            DisplayHint::DateTime => write!(f, "{}", display_as_datetime(&self.data)),
        }
    }
}

/// 自動判定による表示
fn auto_display(data: &ValueData) -> String {
    match data {
        ValueData::Nil => "NIL".to_string(),
        ValueData::Scalar(f) => format_fraction(f),
        ValueData::Vector(v) => {
            // すべてが印字可能な ASCII スカラーなら文字列として表示
            if v.len() > 1 && looks_like_string(v) {
                return display_as_string(data);
            }
            // それ以外は数値として表示
            display_value(data, 0)
        }
    }
}

/// 文字列っぽいかどうかを判定
fn looks_like_string(values: &[Value]) -> bool {
    values.iter().all(|v| {
        if let ValueData::Scalar(f) = &v.data {
            f.is_integer() && {
                if let Some(n) = f.to_i64() {
                    // 印字可能 ASCII または一般的な制御文字
                    (n >= 32 && n < 127) || n == 10 || n == 13 || n == 9
                } else {
                    false
                }
            }
        } else {
            false
        }
    })
}

/// 再帰的にValueを表示（深さに応じた括弧）
fn display_value(data: &ValueData, depth: usize) -> String {
    match data {
        ValueData::Nil => "NIL".to_string(),
        ValueData::Scalar(f) => format_fraction(f),
        ValueData::Vector(v) => {
            if v.is_empty() {
                let bracket = BracketType::from_depth(depth);
                return format!("{} {}", bracket.opening_char(), bracket.closing_char());
            }

            let bracket = BracketType::from_depth(depth);
            let open = bracket.opening_char();
            let close = bracket.closing_char();

            let inner: Vec<String> = v.iter()
                .map(|child| display_value(&child.data, depth + 1))
                .collect();

            format!("{} {} {}", open, inner.join(" "), close)
        }
    }
}

/// Fractionを表示用にフォーマット
fn format_fraction(f: &Fraction) -> String {
    if f.is_nil() {
        return "NIL".to_string();
    }
    if f.is_integer() {
        f.numerator.to_string()
    } else {
        format!("{}/{}", f.numerator, f.denominator)
    }
}

/// 文字列として表示
///
/// UTF-8バイト列として保存されたデータを文字列に復元する。
/// 各Fractionは0-255のバイト値として解釈される。
fn display_as_string(data: &ValueData) -> String {
    match data {
        ValueData::Nil => "''".to_string(),
        ValueData::Scalar(f) => {
            // 単一文字
            if let Some(n) = f.to_i64() {
                if n >= 0 && n <= 255 {
                    let c = n as u8 as char;
                    return format!("'{}'", c);
                }
            }
            format!("'{}'", format_fraction(f))
        }
        ValueData::Vector(v) => {
            if v.is_empty() {
                return "''".to_string();
            }

            // 各Valueをバイトとして収集
            let bytes: Vec<u8> = v.iter()
                .filter_map(|child| {
                    if let ValueData::Scalar(f) = &child.data {
                        f.to_i64().and_then(|n| {
                            if n >= 0 && n <= 255 {
                                Some(n as u8)
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // UTF-8として復元
            let chars = String::from_utf8_lossy(&bytes);
            format!("'{}'", chars)
        }
    }
}

/// 真偽値として表示
fn display_as_boolean(data: &ValueData) -> String {
    match data {
        ValueData::Nil => "NIL".to_string(),
        ValueData::Scalar(f) => {
            if f.is_nil() {
                "NIL".to_string()
            } else if f.is_zero() {
                "FALSE".to_string()
            } else {
                "TRUE".to_string()
            }
        }
        ValueData::Vector(v) => {
            if v.is_empty() {
                return "FALSE".to_string();
            }

            // 複数要素の場合は各要素を真偽値として
            let inner: Vec<&str> = v.iter()
                .map(|child| {
                    match &child.data {
                        ValueData::Nil => "NIL",
                        ValueData::Scalar(f) => {
                            if f.is_nil() {
                                "NIL"
                            } else if f.is_zero() {
                                "FALSE"
                            } else {
                                "TRUE"
                            }
                        }
                        ValueData::Vector(inner) => {
                            if inner.is_empty() {
                                "FALSE"
                            } else {
                                "TRUE"
                            }
                        }
                    }
                })
                .collect();
            format!("{{ {} }}", inner.join(" "))
        }
    }
}

/// 日時として表示
fn display_as_datetime(data: &ValueData) -> String {
    match data {
        ValueData::Nil => display_value(data, 0),
        ValueData::Scalar(f) => {
            // Unix タイムスタンプとして解釈
            // @プレフィックスで表示（JavaScript側で詳細な日時フォーマットを行う）
            if f.is_integer() {
                format!("@{}", f.numerator)
            } else {
                format!("@{}/{}", f.numerator, f.denominator)
            }
        }
        ValueData::Vector(_) => {
            // ベクターの場合は通常表示
            display_value(data, 0)
        }
    }
}
