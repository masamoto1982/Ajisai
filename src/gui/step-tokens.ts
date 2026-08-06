// Splitting source into the pieces step mode executes, with each piece's
// position in the source.
//
// Kept apart from `step-executor` so it is a pure function with no worker, no
// interpreter and no DOM behind it — the shape the test suite exercises
// directly.

/// One whitespace-separated piece of the source, with where it sits in it.
///
/// The offsets travel with the text rather than being recomputed later: a
/// token's text can repeat, so searching the source for it would land on the
/// wrong occurrence. Step mode used to report only "Step 4/9" and the token's
/// text, which left following a run to counting tokens by eye against the
/// source — exactly the accounting a step view exists to remove.
export interface StepToken {
    readonly text: string;
    readonly start: number;
    readonly end: number;
}

/// Split on whitespace — the same division step mode has always executed —
/// while keeping each piece's offset in `code`.
export const tokenizeWithOffsets = (code: string): StepToken[] => {
    const tokens: StepToken[] = [];
    const pattern = /\S+/g;
    let match: RegExpExecArray | null;
    while ((match = pattern.exec(code)) !== null) {
        tokens.push({ text: match[0], start: match.index, end: match.index + match[0].length });
    }
    return tokens;
};
