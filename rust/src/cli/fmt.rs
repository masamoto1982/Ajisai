//! `ajisai fmt` — canonical source formatter (Phase 8A).
//!
//! This is a faithful port of the GUI formatter (`src/gui/code-formatter.ts`).
//! Both implementations are pinned to a shared corpus
//! (`tests/formatter-corpus.json`) so neither drifts and neither becomes a
//! second syntax canon — the corpus is the source of truth, the code the
//! executors.
//!
//! Goal: tidy messy input into the canonical written form **without ever
//! changing what the code means**. A line break inside a `{ }` block is a
//! statement separator (SPEC §3.5) and each `|` COND clause occupies one line
//! (§3.6), so line breaks are semantically significant. The formatter preserves
//! the line structure exactly and rewrites only the insignificant whitespace:
//! the spacing between tokens and the indentation at the start of each line. It
//! never adds or removes line breaks, never touches the inside of a string or a
//! comment, and never expands sugar (`;`, `>CF`, ...). When it meets input it
//! cannot rewrite safely (an unterminated string, or a newline inside a string)
//! it returns the input unchanged.

const INDENT_UNIT: &str = "  ";

/// Characters that are always their own token and can never be part of a word.
/// This is `tokenizer::is_special_char` minus the context-sensitive operator
/// characters (`' # > = ( )`), left untouched so `>CF` is never mis-split.
fn is_standalone_delimiter(c: char) -> bool {
    matches!(c, '[' | ']' | '{' | '}' | '|' | '~' | '^')
}

fn is_opening_bracket(token: &str) -> bool {
    token == "[" || token == "{"
}

fn is_closing_bracket(token: &str) -> bool {
    token == "]" || token == "}"
}

/// Mirrors `tokenizer::is_string_close_delimiter`: a `'` closes a string when
/// the next character is whitespace, end-of-input, or a special character other
/// than another quote.
fn is_string_close_delimiter(ch: Option<char>) -> bool {
    match ch {
        None => true,
        Some(c) => {
            c.is_whitespace()
                || matches!(
                    c,
                    '[' | ']' | '{' | '}' | '(' | ')' | '#' | '>' | '=' | '|' | '~' | '^'
                )
        }
    }
}

/// Tokenize the source into lines of token strings. Strings and comments are
/// captured verbatim as single tokens; delimiters and words each become their
/// own token. Returns `None` when the source cannot be safely reformatted (an
/// unterminated string, or a newline inside a string literal).
fn scan_lines(source: &str) -> Option<Vec<Vec<String>>> {
    let chars: Vec<char> = source.chars().collect();
    let n = chars.len();
    let mut lines: Vec<Vec<String>> = Vec::new();
    let mut line: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut i = 0usize;

    macro_rules! push_word {
        () => {
            if !word.is_empty() {
                line.push(std::mem::take(&mut word));
            }
        };
    }

    while i < n {
        let c = chars[i];

        if c == '\n' {
            push_word!();
            lines.push(std::mem::take(&mut line));
            i += 1;
            continue;
        }

        if c == '#' {
            // Comment runs to end of line; keep its inner spacing verbatim,
            // trimming only trailing whitespace.
            push_word!();
            let mut comment = String::new();
            while i < n && chars[i] != '\n' {
                comment.push(chars[i]);
                i += 1;
            }
            line.push(comment.trim_end().to_string());
            continue;
        }

        if c == '\'' {
            push_word!();
            let mut s = String::from("'");
            let mut j = i + 1;
            let mut closed = false;
            while j < n {
                let cj = chars[j];
                if cj == '\n' {
                    return None; // newline inside a string: refuse to reformat
                }
                s.push(cj);
                if cj == '\'' && is_string_close_delimiter(chars.get(j + 1).copied()) {
                    closed = true;
                    j += 1;
                    break;
                }
                j += 1;
            }
            if !closed {
                return None; // unterminated string: refuse to reformat
            }
            line.push(s);
            i = j;
            continue;
        }

        if c.is_whitespace() {
            push_word!();
            i += 1;
            continue;
        }

        if is_standalone_delimiter(c) {
            push_word!();
            line.push(c.to_string());
            i += 1;
            continue;
        }

        word.push(c);
        i += 1;
    }

    push_word!();
    lines.push(std::mem::take(&mut line));
    Some(lines)
}

fn count_leading_closers(tokens: &[String]) -> usize {
    tokens.iter().take_while(|t| is_closing_bracket(t)).count()
}

fn net_bracket_delta(tokens: &[String]) -> i64 {
    let mut net = 0i64;
    for token in tokens {
        if is_opening_bracket(token) {
            net += 1;
        } else if is_closing_bracket(token) {
            net -= 1;
        }
    }
    net
}

fn render_lines(lines: &[Vec<String>]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut depth: i64 = 0;
    let mut pending_blank = false;

    for tokens in lines {
        if tokens.is_empty() {
            // Collapse runs of blank lines and drop leading/trailing ones.
            if !out.is_empty() {
                pending_blank = true;
            }
            continue;
        }

        if pending_blank {
            out.push(String::new());
            pending_blank = false;
        }

        let indent = (depth - count_leading_closers(tokens) as i64).max(0) as usize;
        out.push(format!(
            "{}{}",
            INDENT_UNIT.repeat(indent),
            tokens.join(" ")
        ));
        depth = (depth + net_bracket_delta(tokens)).max(0);
    }

    out.join("\n")
}

/// Format Ajisai source into its canonical written form. Returns the input
/// unchanged when it cannot be reformatted without risking a semantic change.
/// The result carries no trailing newline; the file convention is applied by
/// the CLI command.
pub(crate) fn format_ajisai_source(source: &str) -> String {
    match scan_lines(source) {
        Some(lines) => render_lines(&lines),
        None => source.to_string(),
    }
}

/// `ajisai fmt <file>`: rewrite the file into its canonical written form. It
/// tidies only insignificant whitespace and indentation and never changes
/// meaning; by default it prints the result to stdout, `--write` formats in
/// place, and `--check` verifies without writing (exit 1 when the file is not
/// already canonical). The canonical file is the formatter's content plus a
/// single trailing newline (an empty file stays empty).
pub(crate) fn cmd_fmt(path: &str, opts: &super::Opts) -> i32 {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(e) => {
            eprintln!("ajisai: cannot read {}: {}", path, e);
            return 2;
        }
    };
    let formatted = format_ajisai_source(&source);
    let canonical = if formatted.is_empty() {
        String::new()
    } else {
        format!("{}\n", formatted)
    };

    if opts.fmt_check {
        if source == canonical {
            0
        } else {
            eprintln!(
                "{}: not formatted (run `ajisai fmt --write {}`)",
                path, path
            );
            1
        }
    } else if opts.fmt_write {
        if source == canonical {
            return 0;
        }
        match std::fs::write(path, &canonical) {
            Ok(()) => {
                eprintln!("{}: formatted", path);
                0
            }
            Err(e) => {
                eprintln!("ajisai: cannot write {}: {}", path, e);
                2
            }
        }
    } else {
        print!("{}", canonical);
        0
    }
}

#[cfg(test)]
mod tests {
    use super::format_ajisai_source;

    #[derive(serde::Deserialize)]
    struct Corpus {
        cases: Vec<Case>,
    }

    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        input: String,
        expected: String,
    }

    /// The CLI formatter must reproduce every shared-corpus pair exactly — the
    /// same corpus the GUI formatter is pinned to, so the two never drift.
    #[test]
    fn matches_shared_corpus() {
        let raw = include_str!("../../../tests/formatter-corpus.json");
        let corpus: Corpus = serde_json::from_str(raw).expect("corpus parses");
        assert!(!corpus.cases.is_empty(), "corpus is non-empty");
        for case in &corpus.cases {
            assert_eq!(
                format_ajisai_source(&case.input),
                case.expected,
                "formatter corpus case `{}`",
                case.name
            );
        }
    }

    /// Formatting is a fixpoint: re-formatting canonical output changes nothing.
    #[test]
    fn is_idempotent_over_the_corpus() {
        let raw = include_str!("../../../tests/formatter-corpus.json");
        let corpus: Corpus = serde_json::from_str(raw).expect("corpus parses");
        for case in &corpus.cases {
            let once = format_ajisai_source(&case.input);
            assert_eq!(
                format_ajisai_source(&once),
                once,
                "formatter must be idempotent on case `{}`",
                case.name
            );
        }
    }
}
