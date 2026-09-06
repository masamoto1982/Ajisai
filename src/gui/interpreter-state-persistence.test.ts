// Adversarial robustness for the import-document parser: an imported .json file
// is fully untrusted, so `parseImportDocument` must honour its `Result` contract
// (never throw) and only forward well-formed words downstream. Regression for
// the fuzzing finding that malformed v2 entries (null / name-less / non-string
// name) threw a TypeError out of the parser.

import { describe, expect, test } from 'vitest';
import { backfillExampleDescriptions, parseImportDocument } from './interpreter-state-persistence';
import { EXAMPLE_USER_WORDS } from './example-words';
import type { UserWord } from '../wasm-interpreter-types';

describe('parseImportDocument robustness', () => {
    const malformed = [
        '{"words":[null]}',
        '{"words":[{"id":"x"}]}',
        '{"words":[{"id":"x","name":123}]}',
        '{"words":[{"id":"x","name":null}]}',
        '{"words":[{"id":"x","name":{"nested":1}}]}',
        '{"words":[{"name":123,"id":true}]}',
        '{"words":[null,1,"str",true,[],{}]}',
        '[null,1,"str",{"name":2}]',
        'null', '123', '"str"', 'true',
    ];
    for (const doc of malformed) {
        test(`never throws on ${doc.slice(0, 40)}`, () => {
            expect(() => parseImportDocument(doc)).not.toThrow();
        });
    }

    test('drops malformed entries but keeps valid words (v2)', () => {
        const doc = '{"formatVersion":2,"dictionary":"D","words":[null,{"name":"OK","definition":"{ 1 }","id":"abc"},{"id":"y"}]}';
        const result = parseImportDocument(doc);
        expect(result.ok).toBe(true);
        if (!result.ok) return;
        expect(result.value.words).toEqual([{ name: 'OK', definition: '{ 1 }' }]);
        expect(result.value.embeddedIds?.get('OK')).toBe('abc');
    });

    test('rejects the alpha bare-array export instead of migrating it', () => {
        const result = parseImportDocument('[{"name":"A","definition":"{ 1 }"},{"name":"B","definition":null}]');
        expect(result.ok).toBe(false);
        if (result.ok) return;
        expect(result.error.message).toContain('Invalid file format');
    });

    test('rejects an unrecognized shape with a clean error', () => {
        const result = parseImportDocument('{"unexpected":true}');
        expect(result.ok).toBe(false);
        if (result.ok) return;
        expect(result.error.message).toContain('Invalid file format');
    });
});

// Regression for a session that saved FIZZBUZZ before the seed data gained a
// `description`: `loadDatabaseData` only reseeds Example Words when the saved
// state has none at all, so that session's own untouched copy of FIZZBUZZ —
// same name, same body, no description — would otherwise never pick one up.
describe('backfillExampleDescriptions', () => {
    const fizzbuzz = EXAMPLE_USER_WORDS.find(w => w.name === 'FIZZBUZZ')!;

    test('fills in a seeded description for an untouched saved copy', () => {
        const word: UserWord = { name: 'FIZZBUZZ', definition: fizzbuzz.definition };
        expect(backfillExampleDescriptions([word])).toBe(true);
        expect(word.description).toBe(fizzbuzz.description);
    });

    test('leaves a customized body alone', () => {
        const word: UserWord = { name: 'FIZZBUZZ', definition: "'Fizz' PRINT" };
        expect(backfillExampleDescriptions([word])).toBe(false);
        expect(word.description).toBeUndefined();
    });

    test('leaves an existing description alone', () => {
        const word: UserWord = { name: 'FIZZBUZZ', definition: fizzbuzz.definition, description: 'my own note' };
        expect(backfillExampleDescriptions([word])).toBe(false);
        expect(word.description).toBe('my own note');
    });

    test('ignores a word with no matching Example Word', () => {
        const word: UserWord = { name: 'MY-OWN-WORD', definition: '1 +' };
        expect(backfillExampleDescriptions([word])).toBe(false);
        expect(word.description).toBeUndefined();
    });

    test('is a no-op for an Example Word that has no description of its own', () => {
        const greet = EXAMPLE_USER_WORDS.find(w => w.name === 'GREET')!;
        const word: UserWord = { name: 'GREET', definition: greet.definition };
        expect(backfillExampleDescriptions([word])).toBe(false);
        expect(word.description).toBeUndefined();
    });
});
