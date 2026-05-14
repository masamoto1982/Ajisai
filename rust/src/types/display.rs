use super::fraction::Fraction;
use super::{DenseTensor, DisplayHint, Value, ValueData};
use std::fmt;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format_with_hint(self, self.hint))
    }
}

pub fn format_with_hint(value: &Value, hint: DisplayHint) -> String {
    match hint {
        DisplayHint::Nil => {
            if matches!(value.data, ValueData::Nil) {
                "NIL".to_string()
            } else {
                format_value_recursive(&value.data, 0)
            }
        }
        DisplayHint::Auto => format_value_auto(value),
        DisplayHint::Number => format_value_recursive(&value.data, 0),
        DisplayHint::Interval => format_as_interval(value),
        DisplayHint::String => format_as_string(&value.data),
        DisplayHint::Boolean => format_as_boolean(&value.data),
        DisplayHint::DateTime => format_as_datetime(&value.data),
    }
}

fn format_as_interval(value: &Value) -> String {
    match &value.data {
        ValueData::Vector(v) if v.len() == 2 => {
            let lo = match &v[0].data {
                ValueData::Scalar(f) => format_fraction(f),
                _ => format_value_recursive(&v[0].data, 0),
            };
            let hi = match &v[1].data {
                ValueData::Scalar(f) => format_fraction(f),
                _ => format_value_recursive(&v[1].data, 0),
            };
            format!("[{}, {}]", lo, hi)
        }
        ValueData::Tensor { data, shape } if shape.as_slice() == [2] && data.len() == 2 => {
            format!(
                "[{}, {}]",
                format_fraction(&data.fraction_or_nil(0)),
                format_fraction(&data.fraction_or_nil(1))
            )
        }
        _ => format_value_recursive(&value.data, 0),
    }
}

fn format_value_auto(value: &Value) -> String {
    match &value.data {
        ValueData::Nil => "NIL".to_string(),
        ValueData::Scalar(f) => format_fraction(f),
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
            // NOTE: string-like 推論は Auto hint の表示 fallback でのみ使用すること。
            if !v.is_empty() && is_string_like(v) {
                return format_as_string(&value.data);
            }
            format_value_recursive(&value.data, 0)
        }
        ValueData::Tensor { data, shape } => {
            if shape.len() == 1 && !data.is_empty() && is_fraction_string_like(data) {
                return format_as_string(&value.data);
            }
            format_value_recursive(&value.data, 0)
        }
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
}

fn is_fraction_string_like(data: &DenseTensor) -> bool {
    data.iter().all(|f| {
        f.is_integer()
            && f.to_i64().is_some_and(|n| {
                (0..=0x10FFFF).contains(&n)
                    && char::from_u32(n as u32)
                        .is_some_and(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t')
            })
    })
}

/// Returns whether every element can be interpreted as a printable Unicode code point.
/// This heuristic must be used only for DisplayHint::Auto fallback formatting.
pub fn is_string_like(values: &[Value]) -> bool {
    values.iter().all(|v| {
        if let ValueData::Scalar(f) = &v.data {
            f.is_integer() && {
                if let Some(n) = f.to_i64() {
                    if n >= 0 && n <= 0x10FFFF {
                        if let Some(c) = char::from_u32(n as u32) {
                            !c.is_control() || c == '\n' || c == '\r' || c == '\t'
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        } else {
            false
        }
    })
}

fn format_value_recursive(data: &ValueData, depth: usize) -> String {
    match data {
        ValueData::Nil => "NIL".to_string(),
        ValueData::Scalar(f) => format_fraction(f),
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
            if v.is_empty() {
                return "[ ]".to_string();
            }

            let open = '[';
            let close = ']';

            let inner: Vec<String> = v
                .iter()
                .map(|child| format_value_recursive(&child.data, depth + 1))
                .collect();

            format!("{} {} {}", open, inner.join(" "), close)
        }
        ValueData::Tensor { data, shape } => format_tensor_recursive(data, shape, depth),
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
}

fn format_tensor_recursive(data: &DenseTensor, shape: &[usize], _depth: usize) -> String {
    if shape.is_empty() {
        return "[ ]".to_string();
    }
    if shape.len() == 1 {
        if data.is_empty() {
            return "[ ]".to_string();
        }
        let inner: Vec<String> = data.iter().map(|f| format_fraction(&f)).collect();
        return format!("[ {} ]", inner.join(" "));
    }
    let outer = shape[0];
    let rest = &shape[1..];
    let stride: usize = rest.iter().product();
    if outer == 0 || stride == 0 {
        return "[ ]".to_string();
    }
    let flat = data.to_fractions();
    let inner: Vec<String> = (0..outer)
        .map(|i| {
            format_tensor_slice_recursive(&flat[i * stride..(i + 1) * stride], rest, _depth + 1)
        })
        .collect();
    format!("[ {} ]", inner.join(" "))
}

fn format_tensor_slice_recursive(data: &[Fraction], shape: &[usize], _depth: usize) -> String {
    if shape.is_empty() {
        return "[ ]".to_string();
    }
    if shape.len() == 1 {
        if data.is_empty() {
            return "[ ]".to_string();
        }
        let inner: Vec<String> = data.iter().map(format_fraction).collect();
        return format!("[ {} ]", inner.join(" "));
    }
    let outer = shape[0];
    let rest = &shape[1..];
    let stride: usize = rest.iter().product();
    if outer == 0 || stride == 0 {
        return "[ ]".to_string();
    }
    let inner: Vec<String> = (0..outer)
        .map(|i| {
            format_tensor_slice_recursive(&data[i * stride..(i + 1) * stride], rest, _depth + 1)
        })
        .collect();
    format!("[ {} ]", inner.join(" "))
}

fn format_code_block(tokens: &[super::Token]) -> String {
    use super::Token;
    let token_strs: Vec<String> = tokens
        .iter()
        .map(|t| match t {
            Token::Number(n) => n.to_string(),
            Token::String(s) => format!("'{}'", s),
            Token::Symbol(s) => s.to_string(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::BlockStart => "{".to_string(),
            Token::BlockEnd => "}".to_string(),
            Token::Pipeline => "==".to_string(),
            Token::NilCoalesce => "=>".to_string(),
            Token::CondClauseSep => "$".to_string(),
            Token::SafeMode => "~".to_string(),
            Token::LineBreak => "\n".to_string(),
        })
        .collect();
    token_strs.join(" ")
}

fn format_fraction(f: &Fraction) -> String {
    if f.is_nil() {
        return "NIL".to_string();
    }
    if let Some(source) = f.display_source() {
        return source.to_string();
    }
    if f.is_integer() {
        f.numerator().to_string()
    } else {
        format!("{}/{}", f.numerator(), f.denominator())
    }
}

fn format_as_string(data: &ValueData) -> String {
    match data {
        ValueData::Nil => "''".to_string(),
        ValueData::Scalar(f) => {
            if let Some(n) = f.to_i64() {
                if n >= 0 && n <= 0x10FFFF {
                    if let Some(c) = char::from_u32(n as u32) {
                        return format!("'{}'", c);
                    }
                }
            }
            format!("'{}'", format_fraction(f))
        }
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
            if v.is_empty() {
                return "''".to_string();
            }

            let chars: String = v
                .iter()
                .filter_map(|child| {
                    if let ValueData::Scalar(f) = &child.data {
                        f.to_i64().and_then(|n| {
                            if n >= 0 && n <= 0x10FFFF {
                                char::from_u32(n as u32)
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                })
                .collect();

            format!("'{}'", chars)
        }
        ValueData::Tensor { data, .. } => {
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
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
}

fn format_as_boolean(data: &ValueData) -> String {
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
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
            if v.is_empty() {
                return "FALSE".to_string();
            }

            let inner: Vec<&str> = v
                .iter()
                .map(|child| match &child.data {
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
                    ValueData::Vector(_) | ValueData::Record { .. } => {
                        let cv = match &child.data {
                            ValueData::Vector(v) => v,
                            ValueData::Record { pairs, .. } => pairs,
                            _ => unreachable!(),
                        };
                        if cv.is_empty() {
                            "FALSE"
                        } else {
                            "TRUE"
                        }
                    }
                    ValueData::Tensor { data, .. } => {
                        if data.is_empty() {
                            "FALSE"
                        } else {
                            "TRUE"
                        }
                    }
                    ValueData::CodeBlock(_) => "TRUE",
                    ValueData::ProcessHandle(_) | ValueData::SupervisorHandle(_) => "TRUE",
                })
                .collect();
            format!("{{ {} }}", inner.join(" "))
        }
        ValueData::Tensor { data, .. } => {
            if data.is_empty() {
                return "FALSE".to_string();
            }
            let inner: Vec<&str> = data
                .iter()
                .map(|f| {
                    if f.is_nil() {
                        "NIL"
                    } else if f.is_zero() {
                        "FALSE"
                    } else {
                        "TRUE"
                    }
                })
                .collect();
            format!("{{ {} }}", inner.join(" "))
        }
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
}

fn format_as_datetime(data: &ValueData) -> String {
    match data {
        ValueData::Nil => format_value_recursive(data, 0),
        ValueData::Scalar(f) => {
            if f.is_integer() {
                format!("@{}", f.numerator())
            } else {
                format!("@{}/{}", f.numerator(), f.denominator())
            }
        }
        ValueData::Vector(_) | ValueData::Tensor { .. } | ValueData::Record { .. } => {
            format_value_recursive(data, 0)
        }
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
}
