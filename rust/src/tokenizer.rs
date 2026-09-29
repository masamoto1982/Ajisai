use crate::types::Token;

/// Where a token was written, as a 1-based line and column in characters.
///
/// Ajisai errors used to say only what went wrong — `Stack underflow` with no
/// line, no token and no word — so locating the fault in a six-line program
/// meant running it a line at a time and bisecting. A span costs one small
/// record per token at tokenization and nothing at all at run time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    pub line: u32,
    pub column: u32,
}

impl std::fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    tokenize_with_spans(input).map(|(tokens, _)| tokens)
}

/// Tokenize, and record where each token was written.
///
/// The returned span vector is index-aligned with the token vector, so a
/// caller holding a token index holds its source position. Only the *source*
/// entry point produces spans: tokens reconstructed from a stored word body or
/// from `REFLECT` data were never written anywhere, and inventing a position
/// for them would be worse than having none.
pub fn tokenize_with_spans(input: &str) -> Result<(Vec<Token>, Vec<SourceSpan>), String> {
    let mut tokens = Vec::new();
    let mut spans: Vec<SourceSpan> = Vec::new();
    let chars: Vec<char> = input.chars().collect();

    // Position of every character, so a token's span is a lookup at the index
    // it starts on rather than a second scan.
    let mut positions: Vec<SourceSpan> = Vec::with_capacity(chars.len());
    let mut line: u32 = 1;
    let mut column: u32 = 1;
    for c in &chars {
        positions.push(SourceSpan { line, column });
        if *c == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    let span_at = |index: usize| -> SourceSpan {
        positions
            .get(index)
            .copied()
            .unwrap_or(SourceSpan { line, column })
    };

    let mut i = 0;

    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }

        // SourceDirective: `#` -> COMMENT-LINE (see surface_forms.rs). Not a
        // runtime word; consumed here at the lexical level to end of line.
        // Recognized only at a fresh word position (we only ever reach this
        // branch right after whitespace or a completed token), exactly like
        // Forth's own comment word — `#` glued to a preceding name is just
        // part of that name, not a comment start.
        if chars[i] == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // LiteralSugar: `'` -> STRING-QUOTE (see surface_forms.rs). A string
        // can hold whitespace of its own (`'hello world'`), so it is not
        // bounded by the ordinary word-delimiter rule below — it is its own
        // sub-grammar, delimited by the closing quote rather than by space.
        match parse_string_from_quote(&chars[i..]) {
            QuoteParseResult::StringSuccess(token, consumed) => {
                tokens.push(token);
                spans.push(span_at(i));
                i += consumed;
                continue;
            }
            QuoteParseResult::Unclosed => {
                let quote_char = chars[i];
                return Err(format!("Unclosed literal starting with {}", quote_char));
            }
            QuoteParseResult::NotQuote => {}
        }

        // Whitespace is the sole word delimiter (LANG.SOURCE.TEXT): a token
        // runs to the next whitespace or end of input, full stop — nothing
        // else splits it, the same rule Forth applies to its own words
        // (including its bracket and comment words). No character is checked
        // for validity on the way, because there is no longer any invalid
        // one: every character but whitespace is a name character
        // (`spec/grammar.json`, characterClasses.nameCharacter).
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }

        let token_str: String = chars[start..i].iter().collect();

        // The two structural words of `spec/grammar.json`'s one delimiter
        // pair: like every other Ajisai word (and like Forth's own `[` and
        // `]`), they must stand alone, separated by whitespace. A delimiter
        // glued to anything else — `[1`, `2]`, `[[1]]` — is a source error
        // asking for the space, rather than a silently accepted (and
        // meaningless) name containing a delimiter. This is a
        // whole-lexeme rule, not a per-character one: no character is checked
        // on the way in, and a lexeme either *is* one delimiter or holds none.
        if let Some(token) = delimiter_token(&token_str) {
            tokens.push(token);
            spans.push(span_at(start));
            continue;
        }
        if token_str.contains(DELIMITERS) {
            return Err(format!(
                "'{}' is not a valid token: '[' and ']' must stand alone, separated by whitespace, like every other Ajisai word (LANG.SOURCE.TEXT — whitespace is the sole token delimiter).",
                token_str
            ));
        }

        // The token is interpreted as a whole: a number when the numeric
        // grammar accepts the entire lexeme, otherwise a name. This is why no
        // character needs special treatment — `1/2` is a number because the
        // whole token parses as one, and `/` is a name for the same reason.
        if let Some(token) = parse_number_from_string(&token_str) {
            // `n/0` has the shape of a number and denotes none. Refused here,
            // with the other source errors, so a program that holds one is
            // refused before it runs rather than halfway through it.
            if has_zero_denominator(&token_str) {
                return Err(format!(
                    "zero denominator: '{}' is not a valid fraction literal (the denominator must be non-zero)",
                    token_str
                ));
            }
            tokens.push(token);
            spans.push(span_at(start));
            continue;
        }

        tokens.push(Token::Symbol(token_str.into()));
        spans.push(span_at(start));
    }

    // The one structural gate, over the real tokens, shared with the entry
    // point that reads a stored Vector as code. A second pass over the raw
    // text used to run first for a nicer message, but it tracked `#` and `'`
    // by character rather than by word position, so `[ C# ]` — a name glued
    // to `#` — read to it as a comment swallowing the `]`, and it reported an
    // imbalance the program did not have.
    validate_code_tokens(&tokens)?;
    debug_assert_eq!(
        tokens.len(),
        spans.len(),
        "every token must carry the position it was written at"
    );
    Ok((tokens, spans))
}

/// The delimiter characters: the one pair `spec/grammar.json` declares.
const DELIMITERS: [char; 2] = ['[', ']'];

/// The token a lexeme that is exactly one delimiter emits.
fn delimiter_token(lexeme: &str) -> Option<Token> {
    match lexeme {
        "[" => Some(Token::VectorStart),
        "]" => Some(Token::VectorEnd),
        _ => None,
    }
}

/// Validate the structural grammar of an already-tokenized code value: every
/// `]` closes an open `[`, and every `[` is closed.
pub(crate) fn validate_code_tokens(tokens: &[Token]) -> Result<(), String> {
    let mut depth: usize = 0;
    for token in tokens {
        match token {
            Token::VectorStart => depth += 1,
            Token::VectorEnd => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| "Unexpected ']' without matching '['".to_string())?;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err("Unclosed '[': expected ']'".into());
    }
    Ok(())
}

/// Whether `lexeme` is exactly one Number token under the canonical lexer.
/// Shared by source tokenization and the public code-data decoder so the two
/// entry paths cannot drift into different numeric languages.
pub(crate) fn is_number_token_lexeme(lexeme: &str) -> bool {
    matches!(parse_number_from_string(lexeme), Some(Token::Number(literal)) if literal.lexeme() == lexeme)
}

/// Whether `lexeme` is exactly one Symbol token under the canonical lexer.
/// A delimiter spelling, a number, a comment start and an unclosed quote all
/// fail this test: none of them can be written as one name at a word
/// position, so none of them can name a Word or a binding.
pub(crate) fn is_symbol_token_lexeme(lexeme: &str) -> bool {
    matches!(tokenize(lexeme).ok().as_deref(), Some([Token::Symbol(value)]) if value.as_ref() == lexeme)
}

enum QuoteParseResult {
    StringSuccess(Token, usize),

    Unclosed,

    NotQuote,
}

// LiteralSugar: `'` -> STRING-QUOTE (see surface_forms.rs). A single quote
// serves as both the opening and closing string delimiter; not a runtime word.
fn parse_string_from_quote(chars: &[char]) -> QuoteParseResult {
    if chars.is_empty() {
        return QuoteParseResult::NotQuote;
    }

    let quote_char = chars[0];

    match quote_char {
        '\'' => parse_token_from_string_literal(chars),
        _ => QuoteParseResult::NotQuote,
    }
}

fn parse_token_from_string_literal(chars: &[char]) -> QuoteParseResult {
    if chars.is_empty() || chars[0] != '\'' {
        return QuoteParseResult::NotQuote;
    }

    let mut string = String::new();
    let mut i = 1;

    while i < chars.len() {
        if chars[i] == '\'' {
            if i + 1 >= chars.len() || is_string_close_delimiter(chars[i + 1]) {
                return QuoteParseResult::StringSuccess(Token::String(string.into()), i + 1);
            } else {
                string.push(chars[i]);
                i += 1;
            }
        } else {
            string.push(chars[i]);
            i += 1;
        }
    }

    QuoteParseResult::Unclosed
}

/// A quote closes the string when the next character is whitespace (or end of
/// input, which the callers check separately) — whitespace is the sole token
/// delimiter, so it is the sole string terminator too. A quote followed by
/// anything else is content, which is what lets a string carry an apostrophe
/// (`'It's fine'`) in a language with no escape character and no second quote
/// spelling; it is also why a string glued to what follows it (`'foo'[1]`,
/// `'foo'BAR`) never closes there — the closing quote needs a space after it
/// like every other token boundary.
fn is_string_close_delimiter(c: char) -> bool {
    c.is_whitespace()
}

/// Whether a numeric lexeme is a rational whose denominator is zero.
fn has_zero_denominator(lexeme: &str) -> bool {
    lexeme
        .split_once('/')
        .is_some_and(|(_, den)| den.chars().all(|c| c == '0'))
}

/// How many digits the number a numeric lexeme denotes can take to write out:
/// the digits written, plus the exponent's magnitude, since `1e5000` builds a
/// 5001-digit integer from five characters. This, not the written length, is
/// what the numeric-literal ceiling bounds (LANG.MACHINE.LIMITS), at every
/// entry point that reads the numeric grammar — source, `NUM`, `JSON-DECODE`
/// — because it is what decides how large an integer the parse builds.
/// Saturates rather than overflowing for an exponent no machine could build;
/// a zero mantissa counts only its written digits, since zero is built at no
/// scale.
pub(crate) fn denoted_digit_count(lexeme: &str) -> u64 {
    let (mantissa, exponent) = match lexeme.find(['e', 'E']) {
        Some(at) => (&lexeme[..at], Some(&lexeme[at + 1..])),
        None => (lexeme, None),
    };
    let written = mantissa.chars().filter(|c| c.is_ascii_digit()).count() as u64;
    // Zero is zero at any scale, and the parse builds nothing for it.
    if mantissa.chars().all(|c| !c.is_ascii_digit() || c == '0') {
        return written;
    }
    let scale = exponent.map_or(0, |e| {
        e.trim_start_matches(['+', '-'])
            .parse::<u64>()
            .unwrap_or(u64::MAX)
    });
    written.saturating_add(scale)
}

fn parse_number_from_string(s: &str) -> Option<Token> {
    if s.is_empty() {
        return None;
    }

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    if chars[i] == '-' || chars[i] == '+' {
        // The sign must be followed by a digit; otherwise the token is a name,
        // not a number. This is what leaves a bare `-` an ordinary name.
        if chars.len() == 1 || !chars[i + 1].is_ascii_digit() {
            return None;
        }
        i += 1;
    }

    // A decimal literal carries digits on *both* sides of the point: `0.5`, not
    // `.5` or `5.`. The point is therefore never the first or last character of
    // a number, which is what keeps a bare `.` unambiguously a name and leaves
    // the character allocatable as a symbol later without a second breaking
    // change to the numeric language.
    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return None;
    }

    let start = i;

    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    if i < chars.len() && chars[i] == '/' {
        i += 1;

        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }

        if i == chars.len() {
            return Some(Token::number(s));
        } else {
            return None;
        }
    }

    let mut has_dot = false;
    if i < chars.len() && chars[i] == '.' {
        has_dot = true;
        i += 1;
        // At least one digit after the point: `5.` and `5.e3` are not numbers.
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }

    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        i += 1;
        if i < chars.len() && (chars[i] == '-' || chars[i] == '+') {
            i += 1;
        }
        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }

    if i == start && !has_dot {
        return None;
    }

    if i == chars.len() {
        Some(Token::number(s))
    } else {
        None
    }
}
