// Adversarial robustness for the import-document parser: an imported .json file
// is fully untrusted, so `parseImportDocument` must honour its `Result` contract
// (never throw) and only forward well-formed words downstream. Regression for
// the fuzzing finding that malformed v2 entries (null / name-less / non-string
// name) threw a TypeError out of the parser.

import { describe, expect, test } from 'vitest';
import { parseImportDocument } from './interpreter-state-persistence';

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

    test('parses a valid legacy v1 array', () => {
        const result = parseImportDocument('[{"name":"A","definition":"{ 1 }"},{"name":"B","definition":null}]');
        expect(result.ok).toBe(true);
        if (!result.ok) return;
        expect(result.value.words).toEqual([
            { name: 'A', definition: '{ 1 }' },
            { name: 'B', definition: null },
        ]);
        expect(result.value.embeddedIds).toBeNull();
    });

    test('rejects an unrecognized shape with a clean error', () => {
        const result = parseImportDocument('{"unexpected":true}');
        expect(result.ok).toBe(false);
        if (result.ok) return;
        expect(result.error.message).toContain('Invalid file format');
    });
});
