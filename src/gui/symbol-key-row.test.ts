// The symbol key row carries every symbol the palette it replaced did, in
// pages of six, with the ones a program needs most on the first page.

import { describe, expect, test } from 'vitest';
import { SYMBOL_PAGES } from './symbol-key-row';

const PALETTE_SYMBOLS = [
    '(', ')', '[', ']', '{', '}',
    '<', '>', '+', '-', '*', '/',
    '%', '=', '!', '?', '&', '|',
    '~', '@', '#', '$', '_', '\\',
    ':', ';', '.', ',', "'", '"',
];

describe('SYMBOL_PAGES', () => {
    test('holds all thirty symbols, each exactly once', () => {
        const all = SYMBOL_PAGES.flat();
        expect(all).toHaveLength(PALETTE_SYMBOLS.length);
        expect(new Set(all)).toEqual(new Set(PALETTE_SYMBOLS));
    });

    test('is five pages of six', () => {
        expect(SYMBOL_PAGES.map((page) => page.length)).toEqual([6, 6, 6, 6, 6]);
    });

    test('opens on the Vector brackets, the string quote and the characters of a number', () => {
        expect(SYMBOL_PAGES[0]).toEqual(['[', ']', "'", '/', '.', '-']);
    });
});
