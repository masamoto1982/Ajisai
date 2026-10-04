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

    // One pass over the text, by character: `pos` is the byte offset of the
    // next character and `here` its line and column, so a token's span is the
    // cursor's position where it starts. Lexemes are slices of `input`, not
    // copies, until a token keeps one.
    let mut cursor = Cursor {
        input,
        pos: 0,
        here: SourceSpan { line: 1, column: 1 },
    };

    while let Some(c) = cursor.peek() {
        if c.is_whitespace() {
            cursor.bump();
            continue;
        }

        // SourceDirective: `#` -> COMMENT-LINE (see surface_forms.rs). Not a
        // runtime word; consumed here at the lexical level to end of line.
        // Recognized only at a fresh word position (we only ever reach this
        // branch right after whitespace or a completed token), exactly like
        // Forth's own comment word — `#` glued to a preceding name is just
        // part of that name, not a comment start.
        if c == '#' {
            while cursor.peek().is_some_and(|c| c != '\n') {
                cursor.bump();
            }
            continue;
        }

        // LiteralSugar: `'` -> STRING-QUOTE (see surface_forms.rs). A string
        // can hold whitespace of its own (`'hello world'`), so it is not
        // bounded by the ordinary word-delimiter rule below — it is its own
        // sub-grammar, delimited by the closing quote rather than by space.
        if c == '\'' {
            let span = cursor.here;
            let Some(content) = string_literal_content(&input[cursor.pos..]) else {
                return Err(format!("Unclosed literal starting with {}", c));
            };
            // The opening quote, the content and the closing quote.
            let end = cursor.pos + content.len() + 2;
            while cursor.pos < end {
                cursor.bump();
            }
            tokens.push(Token::String(content.into()));
            spans.push(span);
            continue;
        }

        // Whitespace is the sole word delimiter (LANG.SOURCE.TEXT): a token
        // runs to the next whitespace or end of input, full stop — nothing
        // else splits it, the same rule Forth applies to its own words
        // (including its bracket and comment words). No character is checked
        // for validity on the way, because there is no longer any invalid
        // one: every character but whitespace is a name character
        // (`spec/grammar.json`, characterClasses.nameCharacter).
        let span = cursor.here;
        let start = cursor.pos;
        while cursor.peek().is_some_and(|c| !c.is_whitespace()) {
            cursor.bump();
        }
        let token_str = &input[start..cursor.pos];

        // The two structural words of `spec/grammar.json`'s one delimiter
        // pair: like every other Ajisai word (and like Forth's own `[` and
        // `]`), they must stand alone, separated by whitespace. A delimiter
        // glued to anything else — `[1`, `2]`, `[[1]]` — is a source error
        // asking for the space, rather than a silently accepted (and
        // meaningless) name containing a delimiter. This is a
        // whole-lexeme rule, not a per-character one: no character is checked
        // on the way in, and a lexeme either *is* one delimiter or holds none.
        if let Some(token) = delimiter_token(token_str) {
            tokens.push(token);
            spans.push(span);
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
        if let Some(token) = parse_number_from_string(token_str) {
            // `n/0` has the shape of a number and denotes none. Refused here,
            // with the other source errors, so a program that holds one is
            // refused before it runs rather than halfway through it.
            if has_zero_denominator(token_str) {
                return Err(format!(
                    "zero denominator: '{}' is not a valid fraction literal (the denominator must be non-zero)",
                    token_str
                ));
            }
            tokens.push(token);
            spans.push(span);
            continue;
        }

        tokens.push(Token::Symbol(token_str.into()));
        spans.push(span);
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

/// A position in the source being tokenized, advanced one character at a
/// time so that its line and column stay those of the next character.
struct Cursor<'a> {
    input: &'a str,
    pos: usize,
    here: SourceSpan,
}

impl Cursor<'_> {
    #[inline]
    fn peek(&self) -> Option<char> {
        // Source is nearly all ASCII, which is its own character; decoding
        // UTF-8 from a fresh slice per character was most of this loop.
        match *self.input.as_bytes().get(self.pos)? {
            byte if byte.is_ascii() => Some(char::from(byte)),
            _ => self.input[self.pos..].chars().next(),
        }
    }

    #[inline]
    fn bump(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
            if c == '\n' {
                self.here.line += 1;
                self.here.column = 1;
            } else {
                self.here.column += 1;
            }
        }
    }
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
    // The common case decided without tokenizing: printable ASCII with no
    // whitespace, quote, bracket or `#` is one lexeme that opens no string,
    // comment or delimiter, and it is a name unless it is a number. Anything
    // else is tokenized, which is the definition.
    let plain = !lexeme.is_empty()
        && lexeme
            .bytes()
            .all(|b| b.is_ascii_graphic() && !matches!(b, b'\'' | b'[' | b']' | b'#'));
    if plain {
        return parse_number_from_string(lexeme).is_none();
    }
    matches!(tokenize(lexeme).ok().as_deref(), Some([Token::Symbol(value)]) if value.as_ref() == lexeme)
}

/// Whether `content`, written between quotes as `'content'`, is exactly one
/// String token holding it under the canonical lexer. A quote closes the
/// string only when whitespace or the end of input follows it
/// (`is_string_close_delimiter`), so the one text with no spelling is one
/// holding a quote right before whitespace: `'a' b'` reads as the String `a`
/// and the name `b'`. The bridge that writes a value back as source
/// (`interpreter::value_as_code`) asks this before writing a String literal,
/// so what it writes is what this lexer reads back.
pub(crate) fn is_string_token_content(content: &str) -> bool {
    matches!(
        tokenize(&format!("'{content}'")).ok().as_deref(),
        Some([Token::String(value)]) if value.as_ref() == content
    )
}

/// The content of the string literal `text` opens with its quote, or `None`
/// when no quote closes it.
///
/// LiteralSugar: `'` -> STRING-QUOTE (see surface_forms.rs). A single quote
/// serves as both the opening and closing string delimiter; not a runtime
/// word. The closing quote is the first one after the opening quote that is
/// followed by whitespace or the end of input; every other character,
/// quotes included, is content.
fn string_literal_content(text: &str) -> Option<&str> {
    let body = text.strip_prefix('\'')?;
    let mut chars = body.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        if c == '\''
            && chars
                .peek()
                .is_none_or(|&(_, next)| is_string_close_delimiter(next))
        {
            return Some(&body[..at]);
        }
    }
    None
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
    // The numeric grammar is ASCII, so it reads bytes: a byte of a
    // multi-byte character is never a digit, a sign, a point, an `e` or a
    // `/`, and stops a number exactly where that character would.
    let chars = s.as_bytes();
    if chars.is_empty() {
        return None;
    }

    let mut i = 0;

    if chars[i] == b'-' || chars[i] == b'+' {
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

    if i < chars.len() && chars[i] == b'/' {
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
    if i < chars.len() && chars[i] == b'.' {
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

    if i < chars.len() && (chars[i] == b'e' || chars[i] == b'E') {
        i += 1;
        if i < chars.len() && (chars[i] == b'-' || chars[i] == b'+') {
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
