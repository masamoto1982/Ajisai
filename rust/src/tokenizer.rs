//! Tokenizer for Ajisai source text.
//!
//! Grammar:
//!   token   := number | string | symbol | '#' line-comment
//!   number  := integer | fraction | decimal
//!   integer := -? [0-9]+
//!   fraction:= integer '/' integer
//!   decimal := -? [0-9]* '.' [0-9]+
//!   string  := "'" ( any-char | "'" followed-by non-delimiter )* "'" delimiter
//!   symbol  := any non-whitespace run that is not a number or string
//!
//! A `'` only opens a string literal when it appears as the first character
//! of a token (i.e. immediately after whitespace or at the start of input).
//! Apostrophes inside a symbol — `O'Brien`, `it's` — therefore remain part
//! of that symbol.
//!
//! Inside a string literal, a `'` closes the literal **only** when the next
//! character is whitespace, `#`, or end-of-input. Otherwise the `'` is
//! taken as content and reading continues. This is the legacy Ajisai rule:
//! `'TE''ST'` is one literal whose content is the 6 characters `TE''ST`,
//! `'O'Brien'` is one literal whose content is `O'Brien`, and the rendered
//! form `'...'` simply wraps the content — no escape transformation occurs.
//!
//! Comments start with `#` and run to end of line.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    Integer(String),
    Fraction(String, String),
    Decimal(String),
    StringLit(String),
    Symbol(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenizeError {
    UnterminatedString,
}

pub fn tokenize(src: &str) -> Result<Vec<Token>, TokenizeError> {
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
        if c == '\'' {
            chars.next();
            let mut s = String::new();
            let mut closed = false;
            while let Some(nc) = chars.next() {
                if nc == '\'' {
                    // Legacy rule: a `'` closes the literal only when the
                    // following character is whitespace, `#`, or EOF. In
                    // every other case the `'` is taken as content.
                    let closes = match chars.peek() {
                        None => true,
                        Some(&next) => next.is_whitespace() || next == '#',
                    };
                    if closes {
                        closed = true;
                        break;
                    }
                    s.push('\'');
                    continue;
                }
                s.push(nc);
            }
            if !closed {
                return Err(TokenizeError::UnterminatedString);
            }
            out.push(Token::StringLit(s));
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
    Ok(out)
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
