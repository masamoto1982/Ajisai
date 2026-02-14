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

fn auto_display(data: &ValueData) -> String {
    match data {
        ValueData::Nil => "NIL".to_string(),
        ValueData::Scalar(f) => format_fraction(f),
        ValueData::Vector(v) => {
            if v.len() > 1 && looks_like_string(v) {
                return display_as_string(data);
            }
            display_value(data, 0)
        }
        ValueData::CodeBlock(tokens) => display_code_block(tokens),
    }
}

fn looks_like_string(values: &[Value]) -> bool {
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
        ValueData::CodeBlock(tokens) => display_code_block(tokens),
    }
}

fn display_code_block(tokens: &[super::Token]) -> String {
    use super::Token;
    let token_strs: Vec<String> = tokens.iter().map(|t| {
        match t {
            Token::Number(n) => n.clone(),
            Token::String(s) => format!("'{}'", s),
            Token::Symbol(s) => s.clone(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::CodeBlockStart => ":".to_string(),
            Token::CodeBlockEnd => ";".to_string(),
            Token::ChevronBranch => ">>".to_string(),
            Token::ChevronDefault => ">>>".to_string(),
            Token::Pipeline => "==".to_string(),
            Token::NilCoalesce => "=>".to_string(),
            Token::SafeMode => "~".to_string(),
            Token::LineBreak => "\n".to_string(),
        }
    }).collect();
    format!(": {} ;", token_strs.join(" "))
}

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

fn display_as_string(data: &ValueData) -> String {
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
        ValueData::Vector(v) => {
            if v.is_empty() {
                return "''".to_string();
            }

            let chars: String = v.iter()
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
        ValueData::CodeBlock(tokens) => display_code_block(tokens),
    }
}

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
                        ValueData::CodeBlock(_) => "TRUE",
                    }
                })
                .collect();
            format!("{{ {} }}", inner.join(" "))
        }
        ValueData::CodeBlock(tokens) => display_code_block(tokens),
    }
}

fn display_as_datetime(data: &ValueData) -> String {
    match data {
        ValueData::Nil => display_value(data, 0),
        ValueData::Scalar(f) => {
            // @プレフィックスでJS側に日時フォーマットを委譲
            if f.is_integer() {
                format!("@{}", f.numerator)
            } else {
                format!("@{}/{}", f.numerator, f.denominator)
            }
        }
        ValueData::Vector(_) => {
            display_value(data, 0)
        }
        ValueData::CodeBlock(tokens) => display_code_block(tokens),
    }
}
