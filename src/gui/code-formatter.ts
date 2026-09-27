// Ajisai source formatter.
//
// Tidies input into the canonical written form without changing what the code
// means. A line break is whitespace like any other in Ajisai (LANG.SOURCE.TEXT)
// except that it ends a `#` comment, and the layout is the author's, so the
// line structure is preserved exactly and only the spacing between tokens and
// the indentation at the start of each line are rewritten.
//
// Per line it:
//   - collapses runs of spaces/tabs to a single space;
//   - surrounds the always-standalone delimiters [ ] with spaces, so
//     `[1 2 3]` becomes `[ 1 2 3 ]` and `[[1]]` becomes `[ [ 1 ] ]`;
//   - keeps string literals ('...') and comments (#...) verbatim;
//   - re-indents the line by the bracket nesting depth open at its start.
//
// It never adds or removes a line break. Input it cannot rewrite safely (an
// unterminated string, or a newline inside a string literal) is returned
// unchanged.

import { scanAtoms } from './source-atoms';

const INDENT_UNIT = '  ';

// `[` and `]` are the one delimiter pair of spec/grammar.json, and like every
// other word they must stand alone: the tokenizer (rust/src/tokenizer.rs)
// rejects a bracket glued to adjacent text rather than splitting it off. The
// formatter supplies that whitespace proactively, so `[1 2 3]` becomes valid,
// canonical source instead of a tokenizer error. No other character is split
// out of a word — `^`, `>=`, `#` and `'` inside a name are all part of it — so
// the atoms of source-atoms.ts are kept whole except for these two.
const splitBrackets = (word: string): string[] => word.match(/[[\]]|[^[\]]+/g) ?? [];

// The source as lines of tokens. Returns null when the source cannot be safely
// reformatted (unterminated string, or a newline inside a string literal).
const scanLines = (source: string): string[][] | null => {
    const lines: string[][] = [];
    let line: string[] = [];
    let scanned = 0;

    // Line breaks live in the whitespace between atoms (and inside a string,
    // which is refused below), so the line structure is read off the gaps.
    const advanceTo = (offset: number): void => {
        for (let i = scanned; i < offset; i += 1) {
            if (source[i] === '\n') {
                lines.push(line);
                line = [];
            }
        }
        scanned = offset;
    };

    for (const atom of scanAtoms(source)) {
        advanceTo(atom.start);
        scanned = atom.end;
        switch (atom.kind) {
            case 'string':
                if (!atom.closed || atom.text.includes('\n')) return null;
                line.push(atom.text);
                break;
            case 'comment':
                line.push(atom.text.trimEnd());
                break;
            case 'word':
                line.push(...splitBrackets(atom.text));
                break;
        }
    }
    advanceTo(source.length);
    lines.push(line);
    return lines;
};

const countLeadingClosers = (tokens: string[]): number => {
    let leading = 0;
    while (tokens[leading] === ']') leading += 1;
    return leading;
};

const netBracketDelta = (tokens: string[]): number => {
    let net = 0;
    for (const token of tokens) {
        if (token === '[') net += 1;
        else if (token === ']') net -= 1;
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
            if (out.length > 0) pendingBlank = true;
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
    if (lines === null) return source;
    return renderLines(lines);
};
