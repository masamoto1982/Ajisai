use rug::Rational;
use std::iter::Peekable;
use std::str::Chars;

use crate::types::Token;

fn is_special(ch: char) -> bool {
    ch.is_whitespace() || ch == '[' || ch == ']' || ch == '{' || ch == '}' || ch == '"'
}

fn read_string(chars: &mut Peekable<Chars>) -> String {
    let mut s = String::new();
    while let Some(&ch) = chars.peek() {
        if ch == '"' {
            break;
        }
        s.push(ch);
        chars.next();
    }
    // " を消費
    chars.next();
    s
}

pub fn tokenize(code: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = code.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch == '[' {
            chars.next();
            tokens.push(Token::VectorStart);
            continue;
        }
        if ch == ']' {
            chars.next();
            tokens.push(Token::VectorEnd);
            continue;
        }
        if ch == '{' {
            chars.next();
            tokens.push(Token::BlockStart);
            continue;
        }
        if ch == '}' {
            chars.next();
            tokens.push(Token::BlockEnd);
            continue;
        }

        if ch == '"' {
            chars.next();
            tokens.push(Token::String(read_string(&mut chars)));
            continue;
        }

        // word or number
        let mut s = String::new();
        while let Some(&ch) = chars.peek() {
            if is_special(ch) {
                break;
            }
            s.push(ch);
            chars.next();
        }

        if let Ok(num) = s.parse::<Rational>() {
            tokens.push(Token::Number(num));
        } else {
            tokens.push(Token::Word(s));
        }
    }

    tokens
}
