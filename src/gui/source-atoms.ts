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

const isWhitespace = (character: string): boolean => /\s/.test(character);

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
