// Ajisai source formatter.
//
// Goal: tidy messy input into the canonical written form without ever changing
// what the code means. In Ajisai a line break inside a `{ }` block is a
// statement separator (SPECIFICATION.html 3.5) and each `|` COND clause must
// occupy exactly one line (3.6), so line breaks are semantically significant.
// The formatter therefore preserves the line structure exactly and rewrites
// only the *insignificant* whitespace: the spacing between tokens and the
// indentation at the start of each line.
//
// Per line it:
//   - collapses runs of spaces/tabs to a single space;
//   - surrounds the always-standalone delimiters [ ] { } | ~ ^ with spaces, so
//     `[1 2 3]` becomes `[ 1 2 3 ]` and `[[1]]` becomes `[ [ 1 ] ]`;
//   - keeps string literals ('...') and comments (#...) verbatim;
//   - re-indents the line by the bracket/block nesting depth open at its start.
//
// It never adds or removes line breaks. If the input contains something it
// cannot rewrite safely (an unterminated string, or a newline inside a string
// literal) it returns the input unchanged.

const INDENT_UNIT = '  ';

// Characters that are always their own token in Ajisai source and can never be
// part of a word or number. This is tokenizer.rs::is_special_char minus the
// operator characters whose tokenization depends on context (' # > = ( )),
// which we deliberately leave untouched so we never mis-split e.g. `>CF`.
const STANDALONE_DELIMITERS = new Set(['[', ']', '{', '}', '|', '~', '^']);
const OPENING_BRACKETS = new Set(['[', '{']);
const CLOSING_BRACKETS = new Set([']', '}']);

// Mirrors tokenizer.rs::is_string_close_delimiter: a `'` closes a string when
// the next character is whitespace, end-of-input, or a special character other
// than another quote.
const STRING_CLOSE_SPECIALS = new Set([
    '[', ']', '{', '}', '(', ')', '#', '>', '=', '|', '~', '^',
]);
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
    return renderLines(lines);
};
