// Adversarial robustness for the import-document parser: an imported .json file
// is fully untrusted, so `parseImportDocument` must honour its `Result` contract
// (never throw) and only forward well-formed words downstream. Regression for
// the fuzzing finding that malformed v2 entries (null / name-less / non-string
// name) threw a TypeError out of the parser.

import { describe, expect, test } from 'vitest';
import { createExportData, namesThatDidNotRestore, parseImportDocument } from './interpreter-state-persistence';
import type { AjisaiInterpreter, UserWord } from '../wasm-interpreter-types';

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

describe('namesThatDidNotRestore', () => {
    // Only `collect_user_words_info` is consulted, so the rest of the
    // interpreter surface is not modelled.
    const withWords = (present: string[]): AjisaiInterpreter => ({
        collect_user_words_info: () =>
            present.map(name => ['USER', name, false] as [string, string, boolean]),
    } as unknown as AjisaiInterpreter);

    test('names a requested word that is not in the dictionary afterwards', () => {
        const requested: UserWord[] = [
            { name: 'KEPT', definition: '[ 1 ]' },
            { name: 'LEGACY', definition: '[1]' },
        ];
        expect(namesThatDidNotRestore(withWords(['KEPT']), requested)).toEqual(['LEGACY']);
    });

    test('reports nothing when every requested word arrived', () => {
        const requested: UserWord[] = [
            { name: 'ONE', definition: '[ 1 ]' },
            { name: 'TWO', definition: '[ 2 ]' },
        ];
        expect(namesThatDidNotRestore(withWords(['ONE', 'TWO']), requested)).toEqual([]);
    });

    // An entry with no body asked for nothing, so its absence is not a loss.
    test('does not report a definition-less entry', () => {
        const requested: UserWord[] = [
            { name: 'NO-BODY', definition: null },
            { name: 'REAL', definition: '[ 1 ]' },
        ];
        expect(namesThatDidNotRestore(withWords(['REAL']), requested)).toEqual([]);
    });

    // A word answers to either spelling, so matching is through the same
    // normalization the dictionary uses.
    test('matches the dictionary through the normalized name', () => {
        const requested: UserWord[] = [{ name: 'lower', definition: '[ 1 ]' }];
        expect(namesThatDidNotRestore(withWords(['LOWER']), requested)).toEqual([]);
    });
});

// Regression for a bug where the export document was filtered by a
// per-dictionary label sourced from a hidden `<select>` element left over
// from the module/multi-dictionary era. `collect_user_words_info` reports a
// constant "USER" label for every word (the dictionary has one exportable
// tier), so any filter that could disagree with that label silently dropped
// every word instead of exporting them. `createExportData` no longer
// filters at all: every User Word it is given comes out.
describe('createExportData', () => {
    const fakeInterpreter = (words: string[]): AjisaiInterpreter => ({
        collect_user_words_info: () =>
            words.map(name => ['USER', name, false] as [string, string, boolean]),
        collect_word_identities: () =>
            words.map(name => [name, `id-${name}`] as [string, string]),
        lookup_word_definition: (name: string) => `[ '${name}' ]`,
        lookup_word_description: () => null,
    } as unknown as AjisaiInterpreter);

    test('exports every user word regardless of any dictionary label', () => {
        const data = createExportData(fakeInterpreter(['ALPHA', 'BETA']));
        expect(data.words.map(w => w.name)).toEqual(['ALPHA', 'BETA']);
    });

    test('exports nothing only when there truly are no user words', () => {
        const data = createExportData(fakeInterpreter([]));
        expect(data.words).toEqual([]);
    });

    test('carries each word\'s content identity', () => {
        const data = createExportData(fakeInterpreter(['ALPHA']));
        expect(data.words[0]?.id).toBe('id-ALPHA');
    });
});
