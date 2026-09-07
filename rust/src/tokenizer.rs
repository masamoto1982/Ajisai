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
            if chars[i] == '\n' && tokens.last() != Some(&Token::LineBreak) {
                tokens.push(Token::LineBreak);
                spans.push(span_at(i));
            }
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
            let had_token_before = !tokens.is_empty() && tokens.last() != Some(&Token::LineBreak);

            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }

            if !had_token_before && i < chars.len() && chars[i] == '\n' {
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
        // (including its bracket and comment words). `(` `)` `{` `}` are
        // never valid Ajisai source characters at all, so they are rejected
        // the moment one turns up, wherever in the word it sits — that is a
        // character-validity rule, not a delimiter.
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            if chars[i] == '(' || chars[i] == ')' {
                let concept = if chars[i] == '(' {
                    "RESERVED-BEGIN"
                } else {
                    "RESERVED-END"
                };
                return Err(format!(
                    "'{}' is a reserved marker ({}) and is not a valid Ajisai source character (LANG.SOURCE.TEXT). '[' and ']' are the sole bracket in Ajisai, for code blocks and for the continued-fraction display form alike.",
                    chars[i], concept
                ));
            }
            if chars[i] == '{' || chars[i] == '}' {
                return Err(format!(
                    "'{}' is not a valid Ajisai source character: '{{' and '}}' were retired when code blocks and vectors were unified — use '[' and ']' for both data and code.",
                    chars[i]
                ));
            }
            i += 1;
        }

        let token_str: String = chars[start..i].iter().collect();

        // `[` and `]` are reserved structural words: like every other Ajisai
        // word (and like Forth's own `[` and `]`), they must stand alone,
        // separated by whitespace. A bracket glued to anything else — `[1`,
        // `2]`, `[[1]]` — is a source error asking for the space, rather than
        // a silently accepted (and meaningless) name containing a bracket.
        if token_str == "[" {
            tokens.push(Token::VectorStart);
            spans.push(span_at(start));
            continue;
        }
        if token_str == "]" {
            tokens.push(Token::VectorEnd);
            spans.push(span_at(start));
            continue;
        }
        if token_str.contains('[') || token_str.contains(']') {
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
            tokens.push(token);
            spans.push(span_at(start));
            continue;
        }

        if let Some(token) = parse_control_directive_word(&token_str) {
            tokens.push(token);
            spans.push(span_at(start));
            continue;
        }

        tokens.push(Token::Symbol(token_str.into()));
        spans.push(span_at(start));
    }

    if tokens.last() == Some(&Token::LineBreak) {
        tokens.pop();
        spans.pop();
    }

    check_bracket_matching(input)?;
    // Keep source entry and token-native entry points on one structural
    // validator. The source-oriented bracket check above is retained for its
    // precise diagnostics; this call is the shared semantic acceptance gate.
    validate_code_tokens(&tokens)?;
    debug_assert_eq!(
        tokens.len(),
        spans.len(),
        "every token must carry the position it was written at"
    );
    Ok((tokens, spans))
}

/// Validate the structural grammar of an already-tokenized code value.
pub(crate) fn validate_code_tokens(tokens: &[Token]) -> Result<(), String> {
    let mut delimiters = Vec::new();
    for token in tokens {
        match token {
            Token::VectorStart => delimiters.push(Token::VectorStart),
            Token::VectorEnd if delimiters.pop() == Some(Token::VectorStart) => {}
            Token::VectorEnd => return Err("mismatched code delimiter".into()),
            Token::CondClauseSep if !matches!(delimiters.last(), Some(&Token::VectorStart)) => {
                return Err("'|' separator is only valid directly inside a code block".into())
            }
            _ => {}
        }
    }
    if !delimiters.is_empty() {
        return Err("unclosed code delimiter".into());
    }
    Ok(())
}

/// Whether `lexeme` is exactly one Number token under the canonical lexer.
/// Shared by source tokenization and the public code-data decoder so the two
/// entry paths cannot drift into different numeric languages.
pub(crate) fn is_number_token_lexeme(lexeme: &str) -> bool {
    matches!(parse_number_from_string(lexeme), Some(Token::Number(value)) if value.as_ref() == lexeme)
}

/// Whether `lexeme` is exactly one Symbol token under the canonical lexer.
/// Control directives and delimiter spellings deliberately fail this test:
/// their canonical code-data representation uses their dedicated token tag.
pub(crate) fn is_symbol_token_lexeme(lexeme: &str) -> bool {
    matches!(tokenize(lexeme).ok().as_deref(), Some([Token::Symbol(value)]) if value.as_ref() == lexeme)
}

fn check_bracket_matching(input: &str) -> Result<(), String> {
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut in_comment = false;
    let chars: Vec<char> = input.chars().collect();
    let mut i: usize = 0;

    while i < chars.len() {
        let c: char = chars[i];

        if c == '\n' {
            in_comment = false;
            i += 1;
            continue;
        }

        if in_comment {
            i += 1;
            continue;
        }

        if c == '#' {
            in_comment = true;
            i += 1;
            continue;
        }

        if c == '\'' {
            if in_string {
                if i + 1 >= chars.len() || is_string_close_delimiter(chars[i + 1]) {
                    in_string = false;
                }
            } else {
                in_string = true;
            }
            i += 1;
            continue;
        }

        if in_string {
            i += 1;
            continue;
        }

        match c {
            '[' => stack.push(c),
            ']' => match stack.pop() {
                Some('[') => {}
                None => {
                    return Err("Unexpected ']' without matching '['".to_string());
                }
                Some(_) => unreachable!("only '[' is ever pushed"),
            },
            _ => {}
        }
        i += 1;
    }

    if stack.last().is_some() {
        return Err("Unclosed '[': expected ']'".to_string());
    }

    Ok(())
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

/// `OR-NIL` (SPEC §6.4, core_word_aliases.rs) has no symbol or legacy-name
/// sugar: it is emitted as its own dedicated control token directly from the
/// bare word, because the execution loop reads the *following* source unit
/// positionally — a spelled-out control directive must not fall through to a
/// stack-consuming builtin or an `UnknownWord`.
///
/// Matching is case-folded (`or-nil` == `OR-NIL`) but only on a bare, whole-word
/// token: a qualified name such as `MATH@OR-NIL` is a single token containing `@`
/// and never compares equal, and string literals are lexed earlier, so neither
/// is misconverted. Because the tokenizer emits the control token directly,
/// this name is also not shadowable by a user definition.
fn parse_control_directive_word(s: &str) -> Option<Token> {
    // The COND clause separator obeys the same boundary rule as every other
    // name.
    match s {
        "|" => Some(Token::CondClauseSep),
        _ if s.eq_ignore_ascii_case("OR-NIL") => Some(Token::NilCoalesce),
        _ => None,
    }
}

fn parse_number_from_string(s: &str) -> Option<Token> {
    if s.is_empty() {
        return None;
    }

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    if chars[i] == '-' || chars[i] == '+' {
        // The sign must be followed by a digit; otherwise the token is a name,
        // not a number. This is what leaves `-` free to be the SUB spelling.
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
        let _slash_pos = i;
        i += 1;

        if i >= chars.len() || !chars[i].is_ascii_digit() {
            return None;
        }
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }

        if i == chars.len() {
            return Some(Token::Number(s.into()));
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
        Some(Token::Number(s.into()))
    } else {
        None
    }
}
