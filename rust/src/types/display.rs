use super::fraction::Fraction;
use super::{DisplayHint, Value, ValueData};
use std::fmt;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format_value_auto(&self.data))
    }
}

/// Format a value using a specific display hint (for SemanticRegistry-aware formatting).
pub fn format_with_hint(value: &Value, hint: DisplayHint) -> String {
    match hint {
        DisplayHint::Nil => {
            if matches!(value.data, ValueData::Nil) {
                "NIL".to_string()
            } else {
                format_value_recursive(&value.data, 0)
            }
        }
        DisplayHint::Auto => format_value_auto(&value.data),
        DisplayHint::Number => format_value_recursive(&value.data, 0),
        DisplayHint::String => format_as_string(&value.data),
        DisplayHint::Boolean => format_as_boolean(&value.data),
        DisplayHint::DateTime => format_as_datetime(&value.data),
    }
}

fn format_value_auto(data: &ValueData) -> String {
    match data {
        ValueData::Nil => "NIL".to_string(),
        ValueData::Scalar(f) => format_fraction(f),
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
            if !v.is_empty() && is_string_like(v) {
                return format_as_string(data);
            }
            format_value_recursive(data, 0)
        }
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
    }
}

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
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
    }
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
            Token::SafeMode => "~".to_string(),
            Token::BranchGuard => "$".to_string(),
            Token::LoopGuard => "&".to_string(),
            Token::LineBreak => "\n".to_string(),
        })
        .collect();
    token_strs.join(" ")
}

fn format_fraction(f: &Fraction) -> String {
    if f.is_nil() {
        return "NIL".to_string();
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
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
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
                    ValueData::CodeBlock(_) => "TRUE",
                })
                .collect();
            format!("{{ {} }}", inner.join(" "))
        }
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
    }
}

fn format_as_datetime(data: &ValueData) -> String {
    match data {
        ValueData::Nil => format_value_recursive(data, 0),
        ValueData::Scalar(f) => {
            // @プレフィックスでJS側に日時フォーマットを委譲
            if f.is_integer() {
                format!("@{}", f.numerator())
            } else {
                format!("@{}/{}", f.numerator(), f.denominator())
            }
        }
        ValueData::Vector(_) | ValueData::Record { .. } => format_value_recursive(data, 0),
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
    }
}
