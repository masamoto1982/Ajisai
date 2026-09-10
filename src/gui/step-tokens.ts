// Splitting source into the pieces step mode executes, with each piece's
// position in the source.
//
// Kept apart from `step-executor` so it is a pure function with no worker, no
// interpreter and no DOM behind it — the shape the test suite exercises
// directly.
//
// The unit is a *balanced* piece of source, not a whitespace-separated token.
// That distinction is the whole of this module. Step mode runs each piece on
// its own against the persisted interpreter state, so a piece that cannot
// stand alone cannot be stepped: splitting `[ 1 ] [ 2 ] +` on whitespace hands
// the interpreter a bare `[`, which is the source error `Unclosed '[':
// expected ']'`, and step mode reset on it. Since `[ 42 ]` is the idiomatic
// scalar and Vectors are the language's central data structure, that made the
// feature unusable for very nearly every real program while `1 2 +` — the one
// shape without brackets — kept working, which is why it read as fine.

/// One piece of source that step mode can execute on its own, with where it
/// sits in the source.
///
/// The offsets travel with the text rather than being recomputed later: a
/// piece's text can repeat, so searching the source for it would land on the
/// wrong occurrence. Step mode used to report only "Step 4/9" and the token's
/// text, which left following a run to counting tokens by eye against the
/// source — exactly the accounting a step view exists to remove.
export interface StepToken {
    readonly text: string;
    readonly start: number;
    readonly end: number;
}

interface Atom {
    readonly text: string;
    readonly start: number;
    readonly end: number;
}

const isWhitespace = (character: string): boolean => /\s/.test(character);

/// Where the string literal starting at `start` ends.
///
/// Mirrors the tokenizer's own sub-grammar (LANG.SOURCE.TEXT): `'` is the sole
/// literal delimiter, a string may hold whitespace internally, and the closing
/// `'` must be followed by whitespace or end of input to close. An unterminated
/// string runs to the end, which hands the interpreter the same text it would
/// have received in one go — and so the same error.
const endOfString = (code: string, start: number): number => {
    for (let i = start + 1; i < code.length; i += 1) {
        if (code[i] !== "'") continue;
        const next = code[i + 1];
        if (next === undefined || isWhitespace(next)) return i + 1;
    }
    return code.length;
};

/// Split into whitespace-separated atoms, with strings and comments kept whole.
///
/// A `#` only starts a comment at the start of an atom — glued to a preceding
/// lexeme it is part of that name, not a comment — which is the tokenizer's
/// rule and is reproduced here rather than approximated. Comments are dropped:
/// a comment is not a step, and stepping onto one would spend a keystroke to
/// execute nothing.
const atoms = (code: string): Atom[] => {
    const out: Atom[] = [];
    let i = 0;
    while (i < code.length) {
        if (isWhitespace(code[i]!)) {
            i += 1;
            continue;
        }
        const start = i;
        if (code[i] === '#') {
            while (i < code.length && code[i] !== '\n') i += 1;
            continue;
        }
        if (code[i] === "'") {
            i = endOfString(code, i);
        } else {
            while (i < code.length && !isWhitespace(code[i]!) && code[i] !== "'") i += 1;
        }
        out.push({ text: code.slice(start, i), start, end: i });
    }
    return out;
};

/// Split `code` into the balanced pieces step mode executes, in order.
///
/// A piece is one atom at bracket depth zero, or a whole `[ ... ]` group with
/// everything nested inside it. Interior whitespace and line breaks are
/// preserved, because the piece is executed as the source text it is: a
/// multi-line vector is one value and stepping through it half-built would
/// execute something the program never contains.
///
/// Malformed bracketing is deliberately *not* repaired here. An unclosed `[`
/// yields one final piece running to the end of the source, and a stray `]`
/// yields a piece of its own; either way the interpreter reports the real
/// source error against the real text, which is the same error a plain run
/// would give. A splitter that silently balanced the source would step through
/// a program the editor does not contain.
export const tokenizeWithOffsets = (code: string): StepToken[] => {
    const pieces: StepToken[] = [];
    const all = atoms(code);
    let index = 0;
    while (index < all.length) {
        const first = all[index]!;
        let depth = 0;
        let last = first;
        do {
            const atom = all[index]!;
            if (atom.text === '[') depth += 1;
            else if (atom.text === ']') depth -= 1;
            last = atom;
            index += 1;
        } while (depth > 0 && index < all.length);
        pieces.push({ text: code.slice(first.start, last.end), start: first.start, end: last.end });
    }
    return pieces;
};
