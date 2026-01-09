// rust/src/types/display.rs
//
// 値の表示ロジック
//
// DisplayHint に基づいて、または自動判定で適切な形式に変換する。
// 形状情報に基づいて深さに応じた括弧を使用する。

use super::{Value, DisplayHint, BracketType};
use super::fraction::Fraction;
use std::fmt;
use num_traits::One;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.display_hint {
            DisplayHint::Auto => write!(f, "{}", auto_display(&self.data, &self.shape)),
            DisplayHint::Number => write!(f, "{}", display_with_shape(&self.data, &self.shape, 0)),
            DisplayHint::String => write!(f, "{}", display_as_string(&self.data)),
            DisplayHint::Boolean => write!(f, "{}", display_as_boolean(&self.data)),
            DisplayHint::DateTime => write!(f, "{}", display_as_datetime(&self.data)),
        }
    }
}

/// 自動判定による表示
fn auto_display(data: &[Fraction], shape: &[usize]) -> String {
    // 空なら NIL
    if data.is_empty() {
        return "NIL".to_string();
    }

    // すべてが印字可能な ASCII 文字なら文字列として表示
    if data.len() > 1 && looks_like_string(data) {
        return display_as_string(data);
    }

    // それ以外は数値として表示（形状情報付き）
    display_with_shape(data, shape, 0)
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

/// 形状情報を使用した表示（深さに応じた括弧）
fn display_with_shape(data: &[Fraction], shape: &[usize], depth: usize) -> String {
    if data.is_empty() {
        return "NIL".to_string();
    }

    let bracket = BracketType::from_depth(depth);
    let open = bracket.opening_char();
    let close = bracket.closing_char();

    // 形状が空、または1次元の場合
    if shape.is_empty() || shape.len() == 1 {
        let inner: Vec<String> = data.iter().map(format_fraction).collect();
        if inner.len() == 1 {
            format!("{}{}{}", open, inner[0], close)
        } else {
            format!("{} {} {}", open, inner.join(" "), close)
        }
    } else {
        // 多次元の場合: 再帰的に処理
        let outer_size = shape[0];
        let inner_shape = &shape[1..];
        let inner_size: usize = inner_shape.iter().product();

        let mut parts = Vec::new();
        for i in 0..outer_size {
            let start = i * inner_size;
            let end = start + inner_size;
            if end <= data.len() {
                let slice = &data[start..end];
                parts.push(display_with_shape(slice, inner_shape, depth + 1));
            }
        }

        format!("{} {} {}", open, parts.join(" "), close)
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
        return display_with_shape(data, &[data.len()], 0);
    }

    // Unix タイムスタンプとして解釈
    // @プレフィックスで表示（JavaScript側で詳細な日時フォーマットを行う）
    if data[0].denominator.is_one() {
        format!("@{}", data[0].numerator)
    } else {
        format!("@{}/{}", data[0].numerator, data[0].denominator)
    }
}
