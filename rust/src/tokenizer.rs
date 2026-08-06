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

        // ReservedMarker: `(` -> RESERVED-BEGIN, `)` -> RESERVED-END (see
        // surface_forms.rs). These are never runtime tokens.
        if chars[i] == '(' || chars[i] == ')' {
            let concept = if chars[i] == '(' {
                "RESERVED-BEGIN"
            } else {
                "RESERVED-END"
            };
            return Err(format!(
                "'{}' is a reserved marker ({}) and is not a valid Ajisai source character (Section 3.4). The nested continued-fraction form is a display/serialization artifact only; use '{{' and '}}' for code blocks.",
                chars[i], concept
            ));
        }
        // A structural delimiter is the only kind of character that is a token
        // on its own. Every other character can appear inside a name, so no
        // symbol needs lookahead and none of them splits a word.
        if let Some(token) = structural_token(chars[i]) {
            tokens.push(token);
            spans.push(span_at(i));
            i += 1;
            continue;
        }

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

        // A token runs to the next boundary. `chars[i]` is not one (whitespace,
        // `#`, the reserved parens, the structural delimiters and the quote are
        // all handled above), so this always consumes at least one character.
        let start = i;
        while i < chars.len() && !ends_token(chars[i]) {
            i += 1;
        }

        let token_str: String = chars[start..i].iter().collect();

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
            Token::BlockStart => delimiters.push(Token::BlockStart),
            Token::VectorEnd if delimiters.pop() == Some(Token::VectorStart) => {}
            Token::BlockEnd if delimiters.pop() == Some(Token::BlockStart) => {}
            Token::VectorEnd | Token::BlockEnd => return Err("mismatched code delimiter".into()),
            Token::CondClauseSep if delimiters.last() != Some(&Token::BlockStart) => {
                return Err("'|' separator is only valid directly inside a code block".into())
            }
            _ => {}
        }
    }
    if !delimiters.is_empty() {
        return Err("unclosed code delimiter".into());
    }
    check_cond_clause_per_line_constraint(tokens)
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

/// The characters that build nesting rather than names: they can never be part
/// of a word, so they delimit tokens without any whitespace around them.
fn is_structural_char(c: char) -> bool {
    matches!(c, '[' | ']' | '{' | '}' | '(' | ')')
}

/// A token ends at exactly three things: whitespace, a structural delimiter, or
/// a comment start. Nothing else can end one — every other character,
/// punctuation included, is an ordinary name character, which is why no symbol
/// needs lookahead and no symbol splits a word.
///
/// This is also the rule that closes a string literal (see
/// [`is_string_close_delimiter`]), so source has one notion of "the token stops
/// here" rather than one per construct.
fn ends_token(c: char) -> bool {
    c.is_whitespace() || is_structural_char(c) || c == '#'
}

/// The structural delimiters that are valid source. `(` and `)` are structural
/// too but are reserved markers, rejected earlier with their own diagnostic.
fn structural_token(c: char) -> Option<Token> {
    match c {
        '[' => Some(Token::VectorStart),
        ']' => Some(Token::VectorEnd),
        '{' => Some(Token::BlockStart),
        '}' => Some(Token::BlockEnd),
        _ => None,
    }
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
            '[' | '{' => stack.push(c),
            ']' => match stack.pop() {
                Some('[') => {}
                Some(open) => {
                    return Err(format!(
                        "Mismatched brackets: '{}' is closed by ']', expected '{}'",
                        open,
                        closing_bracket(open)
                    ));
                }
                None => {
                    return Err("Unexpected ']' without matching '['".to_string());
                }
            },
            '}' => match stack.pop() {
                Some('{') => {}
                Some(open) => {
                    return Err(format!(
                        "Mismatched brackets: '{}' is closed by '}}', expected '{}'",
                        open,
                        closing_bracket(open)
                    ));
                }
                None => {
                    return Err("Unexpected '}' without matching '{'".to_string());
                }
            },
            _ => {}
        }
        i += 1;
    }

    if let Some(open) = stack.last() {
        return Err(format!(
            "Unclosed '{}': expected '{}'",
            open,
            closing_bracket(*open)
        ));
    }

    Ok(())
}

fn closing_bracket(open: char) -> char {
    match open {
        '[' => ']',
        '{' => '}',
        _ => '?',
    }
}

fn check_cond_clause_per_line_constraint(tokens: &[Token]) -> Result<(), String> {
    let mut i: usize = 0;
    let mut cond_clause_blocks_in_line: usize = 0;

    while i < tokens.len() {
        match &tokens[i] {
            Token::LineBreak => {
                cond_clause_blocks_in_line = 0;
                i += 1;
            }
            Token::BlockStart => {
                let mut depth: i32 = 1;
                let mut j: usize = i + 1;
                let mut has_clause_sep: bool = false;
                while j < tokens.len() && depth > 0 {
                    match &tokens[j] {
                        Token::BlockStart => depth += 1,
                        Token::BlockEnd => depth -= 1,
                        Token::CondClauseSep if depth == 1 => has_clause_sep = true,
                        _ => {}
                    }
                    j += 1;
                }

                if has_clause_sep {
                    cond_clause_blocks_in_line += 1;
                    if cond_clause_blocks_in_line > 1 {
                        return Err(
                            "COND: | clauses must be written one clause per line".to_string()
                        );
                    }
                }

                // Descend into the block (rather than skipping past its
                // matching `}`) so the one-clause-per-line rule keeps applying
                // to `|` clauses nested inside a multi-line `{ }` body. Each
                // BlockStart is still visited exactly once, so no clause is
                // double-counted, and LineBreaks inside the block reset the
                // per-line counter as expected.
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
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

/// A quote closes the string when the next character would end a token anyway;
/// end of input closes it too, which the callers check. A quote followed by
/// anything else is content, which is what lets a string carry an apostrophe
/// (`'It's fine'`) in a language with no escape character and no second quote
/// spelling.
fn is_string_close_delimiter(c: char) -> bool {
    ends_token(c)
}

/// The spelled-out control directive `VENT` is the canonical name
/// of the sugars `^` and `~` (SPEC §6.4, core_word_aliases.rs). Emit the *same*
/// dedicated control token the sugar produces so the canonical name and its
/// sugar share one token stream and one lazy execution path — the spelled-out
/// name must not fall through to a stack-consuming builtin or an `UnknownWord`.
///
/// Matching is case-folded (`vent` == `VENT`) but only on a bare, whole-word
/// token: a qualified name such as `MATH@VENT` is a single token containing `@`
/// and never compares equal, and string literals are lexed earlier, so neither
/// is misconverted. Because the tokenizer emits the control token directly,
/// these names are also not shadowable by a user definition.
fn parse_control_directive_word(s: &str) -> Option<Token> {
    // A control directive is emitted as its own token because the execution loop
    // reads the *following* source unit positionally. Both spellings of VENT and
    // the COND clause separator arrive here as ordinary name tokens, so they obey
    // exactly the same boundary rule as every other name.
    match s {
        "^" => Some(Token::NilCoalesce),
        "|" => Some(Token::CondClauseSep),
        _ if s.eq_ignore_ascii_case("VENT") => Some(Token::NilCoalesce),
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
