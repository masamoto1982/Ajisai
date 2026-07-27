//! Source text to program tree.
//!
//! Ajisai has one syntactic layer. Tokens are separated by whitespace, and the
//! parser builds a tree of [`Node`]s. There are exactly four kinds of node,
//! and one of them — the node — is also the definition of the "source unit"
//! that `VENT` releases or blocks. Nothing in the language needs a second,
//! looser notion of "the next thing".

use std::fmt;
use std::sync::Arc;

use crate::alias;
use crate::error::{Error, Result};
use crate::number::Number;
use crate::role::Role;
use crate::value::Value;

/// One source unit.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A number or text literal. Evaluates by flowing onto the stack.
    Literal(Value),
    /// `[ ... ]` — a basin. The body runs on a fresh empty stack and whatever
    /// stands in that stack afterwards becomes a vector. `[ 1 2 3 ]` and
    /// `[ 1 2 ADD ]` are the same construct, not a literal plus an exception.
    Basin(Arc<Vec<Node>>),
    /// `{ ... }` — a quote. An unevaluated flow, held as a value.
    Quote(Arc<Vec<Node>>),
    /// A word, already normalized to its canonical name.
    Word(String),
}

/// Split source into whitespace-delimited tokens, keeping text literals whole.
fn tokenize(source: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c == '#' {
            // Comment to end of line.
            for c in chars.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            continue;
        }
        if c == '"' {
            chars.next();
            let mut text = String::from('"');
            let mut closed = false;
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => text.push('\n'),
                        Some('t') => text.push('\t'),
                        Some('"') => text.push('"'),
                        Some('\\') => text.push('\\'),
                        Some(other) => {
                            return Err(Error::MalformedToken(format!("\\{other}")));
                        }
                        None => break,
                    }
                    continue;
                }
                if c == '"' {
                    closed = true;
                    break;
                }
                text.push(c);
            }
            if !closed {
                return Err(Error::Unbalanced {
                    delimiter: "\"".to_string(),
                });
            }
            tokens.push(text);
            continue;
        }
        let mut token = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                break;
            }
            token.push(c);
            chars.next();
        }
        tokens.push(token);
    }
    Ok(tokens)
}

/// Parse source into a program tree.
pub fn parse(source: &str) -> Result<Vec<Node>> {
    let tokens = tokenize(source)?;
    let mut cursor = 0usize;
    let nodes = parse_body(&tokens, &mut cursor, None)?;
    Ok(nodes)
}

fn parse_body(tokens: &[String], cursor: &mut usize, closer: Option<&str>) -> Result<Vec<Node>> {
    let mut nodes = Vec::new();
    while *cursor < tokens.len() {
        let token = &tokens[*cursor];
        *cursor += 1;
        match token.as_str() {
            "[" => {
                let body = parse_body(tokens, cursor, Some("]"))?;
                nodes.push(Node::Basin(Arc::new(body)));
            }
            "{" => {
                let body = parse_body(tokens, cursor, Some("}"))?;
                nodes.push(Node::Quote(Arc::new(body)));
            }
            "]" | "}" => {
                if closer == Some(token.as_str()) {
                    return Ok(nodes);
                }
                return Err(Error::Unbalanced {
                    delimiter: token.clone(),
                });
            }
            _ => nodes.push(parse_atom(token)?),
        }
    }
    match closer {
        None => Ok(nodes),
        Some(delim) => Err(Error::Unbalanced {
            delimiter: match delim {
                "]" => "[".to_string(),
                _ => "{".to_string(),
            },
        }),
    }
}

fn parse_atom(token: &str) -> Result<Node> {
    if let Some(body) = token.strip_prefix('"') {
        let codepoints: Vec<Value> = body.chars().map(|c| Value::integer(c as i64)).collect();
        return Ok(Node::Literal(
            Value::vector(codepoints).with_role(Role::Text),
        ));
    }
    if let Some(number) = Number::parse(token) {
        return Ok(Node::Literal(Value::number(number)));
    }
    // A token that begins like a number but does not parse as one is a
    // malformed literal, not a word named `1..2`.
    let first = token.chars().next().unwrap_or(' ');
    if first.is_ascii_digit()
        || (first == '-' && token.len() > 1 && token[1..].starts_with(|c: char| c.is_ascii_digit()))
    {
        return Err(Error::MalformedToken(token.to_string()));
    }
    Ok(Node::Word(alias::canonical(token)))
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Literal(value) => write!(f, "{value}"),
            Node::Basin(body) => write_body(f, "[", body, "]"),
            Node::Quote(body) => write_body(f, "{", body, "}"),
            Node::Word(name) => f.write_str(name),
        }
    }
}

fn write_body(f: &mut fmt::Formatter<'_>, open: &str, body: &[Node], close: &str) -> fmt::Result {
    if body.is_empty() {
        return write!(f, "{open} {close}");
    }
    write!(f, "{open}")?;
    for node in body {
        write!(f, " {node}")?;
    }
    write!(f, " {close}")
}

/// Render a program tree back to canonical source: canonical word names, one
/// space between units, no alias forms. This is the formatter, and it is the
/// same function the `ajisai fmt` subcommand uses.
pub fn render_program(nodes: &[Node]) -> String {
    nodes
        .iter()
        .map(|node| node.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}
