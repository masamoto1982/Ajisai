// Writing a Dictionary word into the editor: it must stay a word of its own.
// Clicking ADD and then SQRT used to write `ADDSQRT` — one unknown Word where
// the reader had asked for two.

import { describe, expect, test } from 'vitest';
import { separateWord } from './code-input-editor';

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
