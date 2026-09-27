// Splitting source into the pieces step mode executes, with each piece's
// position in the source.
//
// Kept apart from `step-executor` so it is a pure function with no worker, no
// interpreter and no DOM behind it — the shape the test suite exercises
// directly.
//
// The unit is a *balanced* piece of source, not a whitespace-separated atom.
// Step mode runs each piece on its own against the persisted interpreter
// state, so a piece that cannot stand alone cannot be stepped: `[ 42 ]` is the
// idiomatic scalar, and splitting it on whitespace would hand the interpreter
// a bare `[`.

import { scanAtoms } from './source-atoms';

// One piece of source that step mode can execute on its own, with where it
// sits in the source. The offsets travel with the text rather than being
// recomputed later: a piece's text can repeat, so searching the source for it
// would land on the wrong occurrence.
export interface StepToken {
    readonly text: string;
    readonly start: number;
    readonly end: number;
}

// Split `code` into the balanced pieces step mode executes, in order.
//
// A piece is one atom at bracket depth zero, or a whole `[ ... ]` group with
// everything nested inside it. Interior whitespace and line breaks are
// preserved, because the piece is executed as the source text it is: a
// multi-line vector is one value and stepping through it half-built would
// execute something the program never contains. Comments are dropped: a
// comment is not a step.
//
// Malformed source is deliberately *not* repaired here. An unclosed `[` yields
// one final piece running to the end of the source, a stray `]` yields a
// piece of its own, and an unclosed string runs to the end; either way the
// interpreter reports the real source error against the real text, which is
// the same error a plain run would give.
export const tokenizeWithOffsets = (code: string): StepToken[] => {
    const pieces: StepToken[] = [];
    const all = scanAtoms(code).filter((atom) => atom.kind !== 'comment');
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
