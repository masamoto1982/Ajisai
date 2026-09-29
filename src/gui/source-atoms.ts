// The lexical sub-grammar of Ajisai source (LANG.SOURCE.TEXT), as the GUI
// needs it: the one scanner behind the formatter and step mode, mirroring
// rust/src/tokenizer.rs so the two never disagree with each other or with it.
//
// Whitespace is the sole delimiter. At a fresh atom position (after whitespace
// or at the start of the source):
//   - `#` starts a comment that runs to the end of the line;
//   - `'` starts a string, which may hold whitespace and line breaks, and is
//     closed by a `'` that is followed by whitespace or the end of input;
//   - anything else is a word that runs to the next whitespace, full stop —
//     a `#` or `'` inside it is part of the name.
//
// Nothing is repaired or rejected here. A string that never closes runs to the
// end of the source and is marked `closed: false`; a word that holds a bracket
// is returned as written. Each caller decides what to do with those, because
// they differ: the formatter refuses to rewrite an unclosed string, step mode
// hands it to the interpreter to report.

export type AtomKind = 'word' | 'string' | 'comment';

export interface Atom {
    readonly kind: AtomKind;
    /** The atom's text, exactly as written (a comment excludes its line break). */
    readonly text: string;
    /** Offset of the first character, in UTF-16 code units of the source. */
    readonly start: number;
    /** Offset one past the last character. */
    readonly end: number;
    /** For a string: whether a closing quote was found. Always true otherwise. */
    readonly closed: boolean;
}

// The whitespace class of spec/grammar.json (characterClasses.whitespace):
// Unicode White_Space, enumerated. ECMAScript's `\s` is not that class — it
// counts U+FEFF as whitespace, which the grammar makes an ordinary name
// character (a byte-order mark glues to the first word), and leaves out U+0085,
// which the grammar makes whitespace — so the code points are spelled out here
// rather than delegated to the host, as the grammar's own note asks. Every one
// of them is a single UTF-16 code unit, so the scanner's per-unit walk below
// sees each exactly once; a surrogate half is never whitespace.
export const isSourceWhitespace = (character: string): boolean => {
    const code = character.charCodeAt(0);
    return (code >= 0x0009 && code <= 0x000d)
        || code === 0x0020
        || code === 0x0085
        || code === 0x00a0
        || code === 0x1680
        || (code >= 0x2000 && code <= 0x200a)
        || code === 0x2028
        || code === 0x2029
        || code === 0x202f
        || code === 0x205f
        || code === 0x3000;
};

const isWhitespace = isSourceWhitespace;

/** How many characters of source whitespace `text` opens with. */
export const countLeadingSourceWhitespace = (text: string): number => {
    let count = 0;
    while (count < text.length && isSourceWhitespace(text[count]!)) count += 1;
    return count;
};

const countTrailingSourceWhitespace = (text: string): number => {
    let count = 0;
    while (count < text.length && isSourceWhitespace(text[text.length - 1 - count]!)) count += 1;
    return count;
};

// `String.prototype.trim` strips by ECMAScript's class, so it takes a leading
// byte-order mark off a program the tokenizer would have read as glued to its
// first word; the GUI and the CLI must run the same program from the same
// text, so source is trimmed by the grammar's class instead.
export const trimSource = (text: string): string => {
    const leading = countLeadingSourceWhitespace(text);
    if (leading === text.length) return '';
    return text.slice(leading, text.length - countTrailingSourceWhitespace(text));
};

export const trimSourceEnd = (text: string): string =>
    text.slice(0, text.length - countTrailingSourceWhitespace(text));

const endOfComment = (source: string, start: number): number => {
    let i = start;
    while (i < source.length && source[i] !== '\n') i += 1;
    return i;
};

/** `[end, closed]` of the string literal opening at `start`. */
const endOfString = (source: string, start: number): [number, boolean] => {
    for (let i = start + 1; i < source.length; i += 1) {
        if (source[i] !== "'") continue;
        const next = source[i + 1];
        if (next === undefined || isWhitespace(next)) return [i + 1, true];
    }
    return [source.length, false];
};

const endOfWord = (source: string, start: number): number => {
    let i = start;
    while (i < source.length && !isWhitespace(source[i]!)) i += 1;
    return i;
};

export const scanAtoms = (source: string): Atom[] => {
    const atoms: Atom[] = [];
    let i = 0;
    while (i < source.length) {
        if (isWhitespace(source[i]!)) {
            i += 1;
            continue;
        }
        const start = i;
        let kind: AtomKind;
        let closed = true;
        if (source[i] === '#') {
            kind = 'comment';
            i = endOfComment(source, start);
        } else if (source[i] === "'") {
            kind = 'string';
            [i, closed] = endOfString(source, start);
        } else {
            kind = 'word';
            i = endOfWord(source, start);
        }
        atoms.push({ kind, text: source.slice(start, i), start, end: i, closed });
    }
    return atoms;
};
