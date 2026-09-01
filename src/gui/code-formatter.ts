// Ajisai source formatter.
//
// Goal: tidy messy input into the canonical written form without ever changing
// what the code means. In Ajisai a line break at a definition body's own level
// is a statement separator (SPECIFICATION.html 3.5), so line breaks are
// semantically significant. The formatter therefore preserves the line
// structure exactly and rewrites only the *insignificant* whitespace: the
// spacing between tokens and the indentation at the start of each line.
//
// Per line it:
//   - collapses runs of spaces/tabs to a single space;
//   - surrounds the always-standalone delimiters [ ] with spaces, so
//     `[1 2 3]` becomes `[ 1 2 3 ]` and `[[1]]` becomes `[ [ 1 ] ]`;
//   - keeps string literals ('...') and comments (#...) verbatim;
//   - re-indents the line by the bracket/block nesting depth open at its start.
//
// It adds a line break in exactly one situation: when two or more `|` COND
// clauses begin on the same line. That is a layout choice, not a repair — such
// a COND runs either way — but one clause per line is the canonical form, and
// it is the form in which a COND can be read down its guards. If the input
// contains something it cannot rewrite safely (an unterminated string, or a
// newline inside a string literal) it returns the input unchanged.

const INDENT_UNIT = '  ';

// Characters that are always their own token in Ajisai source and can never be
// part of a word or number. This is tokenizer.rs::is_structural_char: only the
// bracket family. Every other punctuation character — including `|` (the COND
// clause separator) and `^` — obeys the ordinary word-boundary rule instead:
// it ends a token only where whitespace, a bracket, or a comment already would,
// so it is glued into the surrounding word when written without a space (SPEC
// AQ-VER-002-D/E). Forcing any of them apart here would turn one Symbol token
// into several, which is exactly the meaning change this formatter must not
// make. `{`/`}` were retired as source characters entirely (docs/dev/type-
// unification-work-order-2026-08.md): `[ ]` is the sole bracket, for both
// data and code, so they are no longer structural here either.
const STANDALONE_DELIMITERS = new Set(['[', ']']);
const OPENING_BRACKETS = new Set(['[']);
const CLOSING_BRACKETS = new Set([']']);

// Mirrors tokenizer.rs::is_string_close_delimiter, which is exactly
// tokenizer.rs::ends_token: a `'` closes a string when the next character is
// whitespace, a structural delimiter, or `#`. `{`/`}` are included even though
// they are no longer valid source: tokenizer.rs's `is_structural_char` still
// treats them as token-enders (so a name or string ends cleanly at one instead
// of a misplaced `{` being swallowed into it), which is what lets the
// tokenizer raise its "not a valid Ajisai source character" error precisely
// there. `>`, `=`, `|`, and `^` do NOT end a token in the real tokenizer (they
// are ordinary word characters, resolved to a Word only after the whole token
// is scanned), so a `'` immediately followed by one of them does not close the
// string either — the real tokenizer keeps scanning and eventually reports an
// unclosed literal for such input, which this formatter mirrors by refusing to
// reformat it (see the `closed` check in scanLines).
const STRING_CLOSE_SPECIALS = new Set(['[', ']', '{', '}', '(', ')', '#']);
const isStringCloseDelimiter = (ch: string | undefined): boolean =>
    ch === undefined || /\s/.test(ch) || STRING_CLOSE_SPECIALS.has(ch);

// Tokenize the whole source into lines of token strings. Strings and comments
// are captured verbatim as single tokens; structural delimiters and words each
// become their own token. Returns null when the source cannot be safely
// reformatted (unterminated string, or a newline inside a string literal).
const scanLines = (source: string): string[][] | null => {
    const lines: string[][] = [];
    let line: string[] = [];
    let word = '';

    const pushWord = (): void => {
        if (word.length > 0) {
            line.push(word);
            word = '';
        }
    };
    const endLine = (): void => {
        pushWord();
        lines.push(line);
        line = [];
    };

    const chars = Array.from(source);
    let i = 0;

    while (i < chars.length) {
        const c = chars[i]!;

        if (c === '\n') {
            endLine();
            i += 1;
            continue;
        }

        if (c === '#') {
            // Comment runs to end of line; keep its inner spacing verbatim.
            pushWord();
            let comment = '';
            while (i < chars.length && chars[i] !== '\n') {
                comment += chars[i];
                i += 1;
            }
            line.push(comment.replace(/\s+$/, ''));
            continue;
        }

        if (c === "'") {
            pushWord();
            let str = "'";
            let j = i + 1;
            let closed = false;
            while (j < chars.length) {
                const cj = chars[j]!;
                if (cj === '\n') {
                    return null; // newline inside a string: refuse to reformat
                }
                str += cj;
                if (cj === "'" && isStringCloseDelimiter(chars[j + 1])) {
                    closed = true;
                    j += 1;
                    break;
                }
                j += 1;
            }
            if (!closed) {
                return null; // unterminated string: refuse to reformat
            }
            line.push(str);
            i = j;
            continue;
        }

        if (/\s/.test(c)) {
            pushWord();
            i += 1;
            continue;
        }

        if (STANDALONE_DELIMITERS.has(c)) {
            pushWord();
            line.push(c);
            i += 1;
            continue;
        }

        word += c;
        i += 1;
    }

    endLine();
    return lines;
};

// Whether the vector opening at `start` (`tokens[start] === '['`) contains a
// `|` at its own top level — that is, whether it is a COND guard clause rather
// than an ordinary vector.
const isCondClauseBlock = (tokens: string[], start: number): boolean => {
    let depth = 0;
    for (let i = start; i < tokens.length; i += 1) {
        const token = tokens[i];
        if (token === '[') {
            depth += 1;
        } else if (token === ']') {
            depth -= 1;
            if (depth === 0) return false;
        } else if (token === '|' && depth === 1) {
            return true;
        }
    }
    return false;
};

// Break a line so that at most one COND guard clause begins on it.
//
// The formatter otherwise never adds a line break. It adds one here because a
// COND read down its guards is a different thing to read than a COND read
// along a line, and the vertical form is what the reference and every example
// show. The language accepts both — this is the canonical form, not a repair.
//
// The break goes immediately before each clause after the first, and nowhere
// else, so everything the author wrote stays where they put it. Clauses nested
// inside a clause body are split the same way, so the form is the same at
// every depth.
const splitCondClauseLines = (tokens: string[]): string[][] => {
    const out: string[][] = [];
    let current: string[] = [];
    let clausesOnLine = 0;

    for (let i = 0; i < tokens.length; i += 1) {
        if (tokens[i] === '[' && isCondClauseBlock(tokens, i)) {
            if (clausesOnLine >= 1 && current.length > 0) {
                out.push(current);
                current = [];
                clausesOnLine = 0;
            }
            clausesOnLine += 1;
        }
        current.push(tokens[i]!);
    }

    out.push(current);
    return out;
};

const countLeadingClosers = (tokens: string[]): number => {
    let leading = 0;
    while (leading < tokens.length && CLOSING_BRACKETS.has(tokens[leading]!)) {
        leading += 1;
    }
    return leading;
};

const netBracketDelta = (tokens: string[]): number => {
    let net = 0;
    for (const token of tokens) {
        if (OPENING_BRACKETS.has(token)) {
            net += 1;
        } else if (CLOSING_BRACKETS.has(token)) {
            net -= 1;
        }
    }
    return net;
};

const renderLines = (lines: string[][]): string => {
    const out: string[] = [];
    let depth = 0;
    let pendingBlank = false;

    for (const tokens of lines) {
        if (tokens.length === 0) {
            // Collapse runs of blank lines and drop leading/trailing ones.
            if (out.length > 0) {
                pendingBlank = true;
            }
            continue;
        }

        if (pendingBlank) {
            out.push('');
            pendingBlank = false;
        }

        const indent = Math.max(0, depth - countLeadingClosers(tokens));
        out.push(INDENT_UNIT.repeat(indent) + tokens.join(' '));
        depth = Math.max(0, depth + netBracketDelta(tokens));
    }

    return out.join('\n');
};

// Format Ajisai source into its canonical written form. Returns the input
// unchanged when it cannot be reformatted without risking a semantic change.
export const formatAjisaiSource = (source: string): string => {
    const lines = scanLines(source);
    if (lines === null) {
        return source;
    }
    return renderLines(lines.flatMap(splitCondClauseLines));
};
