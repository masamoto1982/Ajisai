use crate::types::Token;

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i].is_whitespace() {
            if chars[i] == '\n'
                && tokens.last() != Some(&Token::LineBreak) {
                    tokens.push(Token::LineBreak);
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
        // ModifierSugar: `;` -> TOP-EAT (`. ,`), `;;` -> STAK-KEEP (`.. ,,`)
        // (see surface_forms.rs). Expanded here into the underlying modifiers.
        if chars[i] == ';' {
            if i + 1 < chars.len() && chars[i + 1] == ';' {
                tokens.push(Token::Symbol("..".into()));
                tokens.push(Token::Symbol(",,".into()));
                i += 2;
                continue;
            }
            tokens.push(Token::Symbol(".".into()));
            tokens.push(Token::Symbol(",".into()));
            i += 1;
            continue;
        }
        if let Some((token, consumed)) = parse_token_from_single_char(chars[i]) {
            tokens.push(token);
            i += consumed;
            continue;
        }

        if chars[i] == '=' {
            tokens.push(Token::Symbol("=".into()));
            i += 1;
            continue;
        }

        if chars[i] == '<' {
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                tokens.push(Token::Symbol("<=".into()));
                i += 2;
                continue;
            }

            if i + 1 < chars.len() && chars[i + 1] == '>' {
                tokens.push(Token::Symbol("<>".into()));
                i += 2;
                continue;
            }

            tokens.push(Token::Symbol("<".into()));
            i += 1;
            continue;
        }

        if chars[i] == '>' {
            // ConversionWord: `>NAME` (e.g. `>CF`) is a single conversion-word
            // token whose canonical home is the runtime word of the same name
            // (see surface_forms::is_conversion_word_token). This is distinct
            // from `>` -> GT and `>=` -> GTE (core_word_aliases.rs).
            if i + 1 < chars.len() && chars[i + 1].is_ascii_alphabetic() {
                let start = i;
                i += 1;
                while i < chars.len() && !chars[i].is_whitespace() && !is_special_char(chars[i]) {
                    i += 1;
                }
                let token_str: String = chars[start..i].iter().collect();
                tokens.push(Token::Symbol(token_str.into()));
                continue;
            }
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                tokens.push(Token::Symbol(">=".into()));
                i += 2;
                continue;
            }
            tokens.push(Token::Symbol(">".into()));
            i += 1;
            continue;
        }

        match parse_string_from_quote(&chars[i..]) {
            QuoteParseResult::StringSuccess(token, consumed) => {
                tokens.push(token);
                i += consumed;
                continue;
            }
            QuoteParseResult::Unclosed => {
                let quote_char = chars[i];
                return Err(format!("Unclosed literal starting with {}", quote_char));
            }
            QuoteParseResult::NotQuote => {}
        }

        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && !is_special_char(chars[i]) {
            i += 1;
        }

        if i == start {
            return Err(format!("Unexpected character: {}", chars[i]));
        }

        let token_str: String = chars[start..i].iter().collect();

        if let Some(token) = parse_keyword_from_string(&token_str) {
            tokens.push(token);
            continue;
        }

        if let Some(token) = parse_control_directive_word(&token_str) {
            tokens.push(token);
            continue;
        }

        if let Some(expanded) = split_compound_modifier(&token_str) {
            for symbol in expanded {
                tokens.push(Token::Symbol(symbol.into()));
            }
            continue;
        }

        if let Some(token) = parse_number_from_string(&token_str) {
            tokens.push(token);
            continue;
        }

        tokens.push(Token::Symbol(token_str.into()));
    }

    if tokens.last() == Some(&Token::LineBreak) {
        tokens.pop();
    }

    check_bracket_matching(input)?;
    check_cond_clause_per_line_constraint(&tokens)?;
    Ok(tokens)
}

fn is_special_char(c: char) -> bool {
    matches!(
        c,
        '[' | ']' | '{' | '}' | '(' | ')' | '#' | '\'' | '>' | '=' | '|' | '~' | '^'
    )
}

fn parse_token_from_single_char(c: char) -> Option<(Token, usize)> {
    // DelimiterSugar / ControlDirective surface forms (see surface_forms.rs):
    // `[` -> BEGIN-VECTOR, `]` -> END-VECTOR, `{` -> BEGIN-BLOCK,
    // `}` -> END-BLOCK, `|` -> COND-CLAUSE. None of these are runtime words.
    match c {
        '[' => Some((Token::VectorStart, 1)),
        ']' => Some((Token::VectorEnd, 1)),
        '{' => Some((Token::BlockStart, 1)),
        '}' => Some((Token::BlockEnd, 1)),

        '|' => Some((Token::CondClauseSep, 1)),

        // Word aliases `~` -> FLOW (visual pipeline marker) and `^` -> VENT
        // (NIL coalescing). Emitted directly as their dedicated tokens; the
        // canonical names live in core_word_aliases.rs.
        '~' => Some((Token::Pipeline, 1)),
        '^' => Some((Token::NilCoalesce, 1)),

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

fn is_string_close_delimiter(c: char) -> bool {
    c.is_whitespace() || (is_special_char(c) && c != '\'')
}

fn parse_keyword_from_string(s: &str) -> Option<Token> {
    match s {
        "." => Some(Token::Symbol(".".into())),
        ".." => Some(Token::Symbol("..".into())),
        "," => Some(Token::Symbol(",".into())),
        ",," => Some(Token::Symbol(",,".into())),
        _ => None,
    }
}

/// The spelled-out control directives `VENT` and `FLOW` are the canonical names
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
    if s.eq_ignore_ascii_case("VENT") {
        Some(Token::NilCoalesce)
    } else if s.eq_ignore_ascii_case("FLOW") {
        Some(Token::Pipeline)
    } else {
        None
    }
}

fn split_compound_modifier(s: &str) -> Option<Vec<String>> {
    let mut remaining = s;
    let mut parts: Vec<String> = Vec::new();
    while !remaining.is_empty() {
        let matched = if let Some(rest) = remaining.strip_prefix("..") {
            parts.push("..".to_string());
            rest
        } else if let Some(rest) = remaining.strip_prefix(",,") {
            parts.push(",,".to_string());
            rest
        } else if let Some(rest) = remaining.strip_prefix('.') {
            parts.push(".".to_string());
            rest
        } else if let Some(rest) = remaining.strip_prefix(',') {
            parts.push(",".to_string());
            rest
        } else {
            return None;
        };
        remaining = matched;
    }
    if parts.len() >= 2 {
        Some(parts)
    } else {
        None
    }
}

fn parse_number_from_string(s: &str) -> Option<Token> {
    if s.is_empty() {
        return None;
    }

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    if chars[i] == '-' || chars[i] == '+' {
        if chars.len() == 1 {
            return None;
        }
        // The sign must be followed by a digit or a leading-dot decimal
        // (`-.5`, `+.5`); otherwise it is a word symbol, not a number.
        let next_is_digit = chars[i + 1].is_ascii_digit();
        let next_is_dot_digit =
            chars[i + 1] == '.' && i + 2 < chars.len() && chars[i + 2].is_ascii_digit();
        if !next_is_digit && !next_is_dot_digit {
            return None;
        }
        i += 1;
    }

    // A leading-dot decimal (`.5`, `-.5`) has an empty integer part: the dot
    // must be followed by at least one digit (SPEC §3.2). Bare `.` / `..` are
    // modifier sugar already handled before this function is reached.
    let has_leading_dot_digits =
        i < chars.len() && chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit();

    if !has_leading_dot_digits && (i >= chars.len() || !chars[i].is_ascii_digit()) {
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
