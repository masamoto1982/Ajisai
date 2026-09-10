// Step mode reports which piece it is about to run, and the editor points at
// it. That only works if each piece carries where it sits in the source: a
// piece's text can repeat, so searching the source for it would land on the
// wrong occurrence.
//
// The other half of what these tests hold is that a piece can be *executed on
// its own*. Step mode runs each piece separately against the persisted state,
// so a split that cuts a vector in half does not merely look odd — it hands
// the interpreter an unclosed bracket and ends step mode on a source error.

import { describe, expect, test } from 'vitest';
import { tokenizeWithOffsets } from './step-tokens';

describe('tokenizeWithOffsets', () => {
    test('splits bracket-free source on whitespace, as step mode always has', () => {
        expect(tokenizeWithOffsets('1 2 ADD').map((t) => t.text)).toEqual(['1', '2', 'ADD']);
    });

    test('each token carries the range it occupies', () => {
        const tokens = tokenizeWithOffsets('1 2 ADD');
        expect(tokens.map((t) => [t.start, t.end])).toEqual([
            [0, 1],
            [2, 3],
            [4, 7]
        ]);
    });

    test('a repeated token gets its own occurrence, not the first one', () => {
        const tokens = tokenizeWithOffsets('ADD 1 ADD');
        expect(tokens[2]).toEqual({ text: 'ADD', start: 6, end: 9 });
    });

    test('newlines and runs of spaces are boundaries like any other whitespace', () => {
        const tokens = tokenizeWithOffsets('1\n\n  2');
        expect(tokens).toEqual([
            { text: '1', start: 0, end: 1 },
            { text: '2', start: 5, end: 6 }
        ]);
    });

    test('empty and whitespace-only source yields no tokens', () => {
        expect(tokenizeWithOffsets('')).toEqual([]);
        expect(tokenizeWithOffsets('  \n ')).toEqual([]);
    });

    // The reported defect. `[ 1 ] [ 2 ] +` split to `[`, `1`, `]`, … and the
    // first step alone was `Unclosed '[': expected ']'`, which reset step mode
    // before anything ran. `[ 42 ]` is the idiomatic scalar, so this was very
    // nearly every real program.
    test('a vector is one step, not a bracket and its contents', () => {
        expect(tokenizeWithOffsets('[ 1 ] [ 2 ] +').map((t) => t.text)).toEqual([
            '[ 1 ]',
            '[ 2 ]',
            '+'
        ]);
    });

    test('a nested vector is still one step', () => {
        expect(tokenizeWithOffsets('[ [ 1 ] [ 2 3 ] ] LENGTH').map((t) => t.text)).toEqual([
            '[ [ 1 ] [ 2 3 ] ]',
            'LENGTH'
        ]);
    });

    test('a code block is a vector, so a definition steps in three', () => {
        expect(tokenizeWithOffsets("[ [ 1 ] + ] 'INC' DEF").map((t) => t.text)).toEqual([
            '[ [ 1 ] + ]',
            "'INC'",
            'DEF'
        ]);
    });

    test('a multi-line vector is one step, interior line breaks and all', () => {
        const tokens = tokenizeWithOffsets('[ 1\n  2 ]\nLENGTH');
        expect(tokens.map((t) => t.text)).toEqual(['[ 1\n  2 ]', 'LENGTH']);
    });

    test('a string holds its own whitespace rather than splitting', () => {
        expect(tokenizeWithOffsets("'hello world' PRINT").map((t) => t.text)).toEqual([
            "'hello world'",
            'PRINT'
        ]);
    });

    test('a bracket inside a string is text, not nesting', () => {
        expect(tokenizeWithOffsets("'[ 1' PRINT").map((t) => t.text)).toEqual(["'[ 1'", 'PRINT']);
    });

    test('a comment is not a step', () => {
        expect(tokenizeWithOffsets('# note\n1 2 +').map((t) => t.text)).toEqual(['1', '2', '+']);
    });

    test('a hash glued to a name is part of that name, not a comment', () => {
        expect(tokenizeWithOffsets('a#b 1').map((t) => t.text)).toEqual(['a#b', '1']);
    });

    // Malformed source is passed through rather than repaired: the reader gets
    // the same error a plain run would give, against the text they wrote.
    test('an unclosed bracket becomes one final step, not a silent repair', () => {
        expect(tokenizeWithOffsets('1 [ 2 3').map((t) => t.text)).toEqual(['1', '[ 2 3']);
    });

    test('a stray closing bracket is its own step', () => {
        expect(tokenizeWithOffsets('1 ] 2').map((t) => t.text)).toEqual(['1', ']', '2']);
    });

    test('offsets still address the exact occurrence for a repeated vector', () => {
        const tokens = tokenizeWithOffsets('[ 1 ] [ 1 ]');
        expect(tokens[1]).toEqual({ text: '[ 1 ]', start: 6, end: 11 });
    });

    test('every piece is exactly the source it points at', () => {
        const code = "[ 1 ] [ [ 2 ] 'x' ] +";
        for (const token of tokenizeWithOffsets(code)) {
            expect(code.slice(token.start, token.end)).toBe(token.text);
        }
    });
});