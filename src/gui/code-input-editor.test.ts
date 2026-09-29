// Writing a Dictionary word into the editor: it must stay a word of its own.
// Clicking ADD and then SQRT used to write `ADDSQRT` — one unknown Word where
// the reader had asked for two.

import { describe, expect, test } from 'vitest';
import { extractToken, separateWord } from './code-input-editor';

const write = (text: string, caret: number, word: string): string => {
    const { insertion } = separateWord(text, caret, caret, word);
    return text.slice(0, caret) + insertion + text.slice(caret);
};

describe('separateWord', () => {
    test('writes a word into an empty editor as it is', () => {
        expect(write('', 0, 'ADD')).toBe('ADD');
    });

    test('keeps two clicked words apart', () => {
        expect(write('ADD', 3, 'SQRT')).toBe('ADD SQRT');
    });

    test('adds no second space where one is already there', () => {
        expect(write('1 2 ', 4, 'ADD')).toBe('1 2 ADD');
    });

    test('separates the word from a name after the caret too', () => {
        expect(write('1 2MUL', 3, 'ADD')).toBe('1 2 ADD MUL');
    });

    test('puts the caret after the word, before any space it added after it', () => {
        const { insertion, caretOffset } = separateWord('1 2MUL', 3, 3, 'ADD');
        expect(insertion.slice(0, caretOffset)).toBe(' ADD');
    });

    test('writes the space a click between the word buttons asks for unchanged', () => {
        expect(separateWord('ADD', 3, 3, ' ')).toEqual({ insertion: ' ', caretOffset: 1 });
    });

    test('replaces a selection with the word, separated from both sides', () => {
        const { insertion } = separateWord('1XY2', 1, 3, 'ADD');
        expect(insertion).toBe(' ADD ');
    });
});

// The word under the cursor, as Lookup (Ctrl+Alt+L) and the suggestion panel
// read it. A name is any run of non-whitespace (spec/grammar.json), so Lookup
// must hand the dictionary the same name execution would.
describe('extractToken', () => {
    const at = (text: string, cursor: number): ReturnType<typeof extractToken> =>
        extractToken(text, cursor);

    test('takes the whole word around the cursor', () => {
        expect(at('1 2 ADD', 5)).toEqual({ token: 'ADD', start: 4, end: 7 });
        expect(at('1 2 ADD', 4)).toEqual({ token: 'ADD', start: 4, end: 7 });
        expect(at('1 2 ADD', 7)).toEqual({ token: 'ADD', start: 4, end: 7 });
    });

    test('a name is any run of non-whitespace, so a dotted, symbolic or Japanese User Word is one word', () => {
        expect(at('X.Y', 1).token).toBe('X.Y');
        expect(at('A|B', 2).token).toBe('A|B');
        expect(at('1 合計', 3).token).toBe('合計');
        expect(at('a<=b', 2).token).toBe('a<=b');
    });

    test('a bracket stands alone, so the word in a glued vector is the name beside it', () => {
        expect(at('[SQ', 3)).toEqual({ token: 'SQ', start: 1, end: 3 });
        expect(at('[ADD]', 3)).toEqual({ token: 'ADD', start: 1, end: 4 });
        expect(at('[[1]]', 2)).toEqual({ token: '1', start: 2, end: 3 });
    });

    test('a cursor on a bracket or on whitespace touches no word', () => {
        expect(at('[ 1 ]', 0).token).toBe('');
        expect(at('[ 1 ]', 1).token).toBe('');
        expect(at('1  2', 2).token).toBe('');
        expect(at('', 0)).toEqual({ token: '', start: 0, end: 0 });
    });

    test('a quote is part of the run, so a string prefix is not completed as a word', () => {
        expect(at("'ma", 3).token).toBe("'ma");
    });

    test('splits on the grammar\'s whitespace, not on a byte-order mark', () => {
        expect(at('1\u0085ADD', 3).token).toBe('ADD');
        expect(at('﻿ADD', 2).token).toBe('﻿ADD');
    });

    test('clamps a cursor outside the text', () => {
        expect(at('ADD', 10)).toEqual({ token: 'ADD', start: 0, end: 3 });
        expect(at('ADD', -1)).toEqual({ token: 'ADD', start: 0, end: 3 });
    });
});
