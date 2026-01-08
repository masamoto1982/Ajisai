// rust/src/types/display.rs
//
// 値の表示ロジック
//
// DisplayHint に基づいて、または自動判定で適切な形式に変換する。

use super::{Value, DisplayHint};
use super::fraction::Fraction;
use std::fmt;
use num_traits::One;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.display_hint {
            DisplayHint::Auto => write!(f, "{}", auto_display(&self.data)),
            DisplayHint::Number => write!(f, "{}", display_as_numbers(&self.data)),
            DisplayHint::String => write!(f, "{}", display_as_string(&self.data)),
            DisplayHint::Boolean => write!(f, "{}", display_as_boolean(&self.data)),
            DisplayHint::DateTime => write!(f, "{}", display_as_datetime(&self.data)),
        }
    }
}

/// 自動判定による表示
fn auto_display(data: &[Fraction]) -> String {
    // 空なら NIL
    if data.is_empty() {
        return "NIL".to_string();
    }

    // すべてが印字可能な ASCII 文字なら文字列として表示
    if data.len() > 1 && looks_like_string(data) {
        return display_as_string(data);
    }

    // それ以外は数値として表示
    display_as_numbers(data)
}

/// 文字列っぽいかどうかを判定
fn looks_like_string(data: &[Fraction]) -> bool {
    data.iter().all(|f| {
        f.is_integer() && {
            if let Some(n) = f.to_i64() {
                // 印字可能 ASCII または一般的な制御文字
                (n >= 32 && n < 127) || n == 10 || n == 13 || n == 9
            } else {
                false
            }
        }
    })
}

/// 数値として表示
fn display_as_numbers(data: &[Fraction]) -> String {
    if data.is_empty() {
        return "NIL".to_string();
    }

    let inner: Vec<String> = data.iter().map(format_fraction).collect();

    // 単一要素の場合はスペースなし、複数要素の場合はスペースあり
    if data.len() == 1 {
        format!("{{{}}}", inner[0])
    } else {
        format!("{{ {} }}", inner.join(" "))
    }
}

/// Fractionを表示用にフォーマット
fn format_fraction(f: &Fraction) -> String {
    if f.denominator.is_one() {
        f.numerator.to_string()
    } else {
        format!("{}/{}", f.numerator, f.denominator)
    }
}

/// 文字列として表示
fn display_as_string(data: &[Fraction]) -> String {
    if data.is_empty() {
        return "''".to_string();
    }

    let chars: String = data
        .iter()
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

    format!("'{}'", chars)
}

/// 真偽値として表示
fn display_as_boolean(data: &[Fraction]) -> String {
    if data.is_empty() {
        return "FALSE".to_string();
    }

    // 単一要素の場合
    if data.len() == 1 {
        if data[0].is_zero() {
            "FALSE".to_string()
        } else {
            "TRUE".to_string()
        }
    } else {
        // 複数要素の場合は各要素を真偽値として
        let inner: Vec<&str> = data
            .iter()
            .map(|f| if f.is_zero() { "FALSE" } else { "TRUE" })
            .collect();
        format!("{{ {} }}", inner.join(" "))
    }
}

/// 日時として表示
fn display_as_datetime(data: &[Fraction]) -> String {
    if data.is_empty() || data.len() != 1 {
        return display_as_numbers(data);
    }

    // Unix タイムスタンプとして解釈
    // @プレフィックスで表示（JavaScript側で詳細な日時フォーマットを行う）
    if data[0].denominator.is_one() {
        format!("@{}", data[0].numerator)
    } else {
        format!("@{}/{}", data[0].numerator, data[0].denominator)
    }
}
