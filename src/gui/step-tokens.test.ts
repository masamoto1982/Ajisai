// Step mode reports which token it is about to run, and the editor points at
// it. That only works if each token carries where it sits in the source: a
// token's text can repeat, so searching the source for it would land on the
// wrong occurrence.

import { describe, expect, test } from 'vitest';
import { tokenizeWithOffsets } from './step-tokens';

describe('tokenizeWithOffsets', () => {
    test('splits on whitespace, as step mode always has', () => {
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
});
