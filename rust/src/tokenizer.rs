//! Tokenizer for Ajisai source text.
//!
//! Phase 1 grammar:
//!   token   := number | symbol | '#' line-comment
//!   number  := integer | fraction | decimal
//!   integer := -? [0-9]+
//!   fraction:= integer '/' integer
//!   decimal := -? [0-9]* '.' [0-9]+
//!   symbol  := any non-whitespace run that is not a number
//!
//! Comments start with `#` and run to end of line.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Integer(String),
    Fraction(String, String),
    Decimal(String),
    Symbol(String),
}

pub fn tokenize(src: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut chars = src.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c == '#' {
            while let Some(&nc) = chars.peek() {
                if nc == '\n' {
                    break;
                }
                chars.next();
            }
            continue;
        }
        let mut buf = String::new();
        while let Some(&nc) = chars.peek() {
            if nc.is_whitespace() {
                break;
            }
            buf.push(nc);
            chars.next();
        }
        out.push(classify(&buf));
    }
    out
}

fn classify(raw: &str) -> Token {
    if let Some((num, den)) = raw.split_once('/') {
        if is_signed_int(num) && is_signed_int(den) && !den.is_empty() {
            return Token::Fraction(num.to_string(), den.to_string());
        }
    }
    if is_signed_int(raw) {
        return Token::Integer(raw.to_string());
    }
    if is_decimal(raw) {
        return Token::Decimal(raw.to_string());
    }
    Token::Symbol(raw.to_string())
}

fn is_signed_int(s: &str) -> bool {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    !body.is_empty() && body.chars().all(|c| c.is_ascii_digit())
}

fn is_decimal(s: &str) -> bool {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    let mut parts = body.splitn(2, '.');
    let int_part = parts.next().unwrap_or("");
    let frac_part = match parts.next() {
        Some(f) => f,
        None => return false,
    };
    if int_part.is_empty() && frac_part.is_empty() {
        return false;
    }
    int_part.chars().all(|c| c.is_ascii_digit())
        && frac_part.chars().all(|c| c.is_ascii_digit())
        && !frac_part.is_empty()
}
