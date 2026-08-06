// The Core dictionary sheet must be able to show every canonical Word.
//
// The sheet filters the interpreter's Core payload through
// `isCanonicalCoreWordName`, so that predicate is the last gate between a Word
// existing and a reader being able to find it. The authority for what a
// canonical name looks like is spec/words.json, so this test reads that file
// rather than restating a list: a name shape the vocabulary adopts later is
// covered the moment it lands.

import { describe, expect, test } from 'vitest';
import { readFileSync } from 'node:fs';
import { isCanonicalCoreWordName } from './core-word-name';

interface WordEntry {
    readonly name: string;
    readonly aliases?: readonly string[];
}

const registry = JSON.parse(readFileSync('spec/words.json', 'utf8')) as { entries: WordEntry[] };

describe('isCanonicalCoreWordName', () => {
    test('the registry is the 59 canonical Words the Specification claims', () => {
        expect(registry.entries).toHaveLength(59);
    });

    test.each(registry.entries.map(word => word.name))('accepts the canonical name %s', name => {
        expect(isCanonicalCoreWordName(name)).toBe(true);
    });

    test('every canonical Word survives the sheet filter', () => {
        const listed = registry.entries.map(word => word.name).filter(isCanonicalCoreWordName);
        expect(listed).toHaveLength(registry.entries.length);
    });

    // The regression this predicate was fixed for: `NIL?` is a canonical Word
    // and was the one the Core sheet dropped, leaving 56 of 57 visible.
    test('accepts a name ending in the absence-question mark', () => {
        expect(isCanonicalCoreWordName('NIL?')).toBe(true);
    });

    test('rejects the symbolic aliases, which are surface forms rather than Words', () => {
        const symbolic = registry.entries
            .flatMap(word => word.aliases ?? [])
            .filter(alias => !/^[A-Za-z]/.test(alias));
        expect(symbolic.length).toBeGreaterThan(0);
        for (const alias of symbolic) {
            expect(isCanonicalCoreWordName(alias)).toBe(false);
        }
    });

    test('rejects lower-case and empty spellings', () => {
        expect(isCanonicalCoreWordName('nil?')).toBe(false);
        expect(isCanonicalCoreWordName('')).toBe(false);
        expect(isCanonicalCoreWordName('1ADD')).toBe(false);
    });
});
