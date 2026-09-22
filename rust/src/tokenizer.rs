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
        // (including its bracket and comment words). No character is checked
        // for validity on the way, because there is no longer any invalid
        // one: `(` `)` `{` `}` and a bare `|` used to be refused here and are
        // now ordinary name characters like every other punctuation mark
        // (`spec/grammar.json`, characterClasses.nameCharacter).
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() {
            i += 1;
        }

        let token_str: String = chars[start..i].iter().collect();

        // The four structural words, one pair per delimiter pair of
        // `spec/grammar.json`: like every other Ajisai word (and like Forth's
        // own `[` and `]`), they must stand alone, separated by whitespace. A
        // delimiter glued to anything else — `[1`, `2]`, `[[1]]`, `{1`, `a}`
        // — is a source error asking for the space, rather than a silently
        // accepted (and meaningless) name containing a delimiter. This is a
        // whole-lexeme rule, not a per-character one: no character is checked
        // on the way in, and a lexeme either *is* one delimiter or holds none.
        if let Some(token) = delimiter_token(&token_str) {
            tokens.push(token);
            spans.push(span_at(start));
            continue;
        }
        if token_str.contains(DELIMITERS) {
            return Err(format!(
                "'{}' is not a valid token: '[', ']', '{{' and '}}' must stand alone, separated by whitespace, like every other Ajisai word (LANG.SOURCE.TEXT — whitespace is the sole token delimiter).",
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

/// The delimiter characters, in the pairs `spec/grammar.json` declares.
const DELIMITERS: [char; 4] = ['[', ']', '{', '}'];

/// The token a lexeme that is exactly one delimiter emits.
fn delimiter_token(lexeme: &str) -> Option<Token> {
    match lexeme {
        "[" => Some(Token::VectorStart),
        "]" => Some(Token::VectorEnd),
        "{" => Some(Token::RecordStart),
        "}" => Some(Token::RecordEnd),
        _ => None,
    }
}

/// The opener a closing delimiter token must find on the stack, if the token
/// is a closer at all.
fn opener_of(token: &Token) -> Option<Token> {
    match token {
        Token::VectorEnd => Some(Token::VectorStart),
        Token::RecordEnd => Some(Token::RecordStart),
        _ => None,
    }
}

/// Validate the structural grammar of an already-tokenized code value.
///
/// One stack over both pairs, so a crossed `[ }` is mismatched rather than
/// accepted — the property a pair-per-counter version cannot see.
pub(crate) fn validate_code_tokens(tokens: &[Token]) -> Result<(), String> {
    let mut delimiters = Vec::new();
    for token in tokens {
        match token {
            Token::VectorStart | Token::RecordStart => delimiters.push(token.clone()),
            Token::VectorEnd | Token::RecordEnd => {
                let innermost = delimiters.pop();
                if innermost != opener_of(token) {
                    return Err("mismatched code delimiter".into());
                }
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
    matches!(parse_number_from_string(lexeme), Some(Token::Number(literal)) if literal.lexeme() == lexeme)
}

/// Whether `lexeme` is exactly one Symbol token under the canonical lexer.
/// Control directives and delimiter spellings deliberately fail this test:
/// their canonical code-data representation uses their dedicated token tag.
pub(crate) fn is_symbol_token_lexeme(lexeme: &str) -> bool {
    matches!(tokenize(lexeme).ok().as_deref(), Some([Token::Symbol(value)]) if value.as_ref() == lexeme)
}

/// The text-level precheck the grammar documents as deliberately partial: it
/// may miss an imbalance, never invent one, because [`validate_code_tokens`]
/// runs afterwards on the real tokens and has the final say. It is kept for
/// its message, which names the pair and the character.
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
            '[' | '{' => stack.push(c),
            ']' | '}' => {
                let expected = if c == ']' { '[' } else { '{' };
                match stack.pop() {
                    Some(open) if open == expected => {}
                    None => {
                        return Err(format!("Unexpected '{c}' without matching '{expected}'"));
                    }
                    // A crossed pair: the fault is `mismatched code delimiter`,
                    // which the token-level validator reports. This pass has
                    // just lost an opener, so every later reading of its stack
                    // would be a guess — stopping is how it stays incapable of
                    // inventing a condition.
                    Some(_) => return Ok(()),
                }
            }
            _ => {}
        }
        i += 1;
    }

    match stack.last() {
        Some('[') => Err("Unclosed '[': expected ']'".to_string()),
        Some('{') => Err("Unclosed '{': expected '}'".to_string()),
        Some(other) => unreachable!("only an opener is ever pushed, got {other:?}"),
        None => Ok(()),
    }
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
