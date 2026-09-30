// Adversarial robustness for the import-document parser: an imported .json file
// is fully untrusted, so `parseImportDocument` must honour its `Result` contract
// (never throw) and only forward well-formed words downstream, whatever a
// malformed entry looks like (null / name-less / non-string name).

import { describe, expect, test } from 'vitest';
import { checkHasSavedDictionary, createExportData, normalizeWordEntry, parseImportDocument, summarizeImport } from './interpreter-state-persistence';
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

describe('summarizeImport', () => {
    const ids = (entries: Record<string, string>): Map<string, string> => new Map(Object.entries(entries));

    test('counts an arrival as added and an identical word already present as unchanged', () => {
        const requested: UserWord[] = [
            { name: 'NEW', definition: '[ 1 ]' },
            { name: 'SAME', definition: '[ 2 ]' },
        ];
        const summary = summarizeImport(requested, [], ids({ SAME: 's' }), ids({ NEW: 'n', SAME: 's' }), null);
        expect(summary.added).toEqual(['NEW']);
        expect(summary.unchanged).toEqual(['SAME']);
        expect(summary.skipped).toEqual([]);
    });

    // The interpreter refused the file's body and kept the old one: the
    // dictionary looks the same before and after, which used to read as
    // "unchanged (deduplicated by content identity)".
    test('reports a refused redefinition as skipped, never as unchanged', () => {
        const requested: UserWord[] = [{ name: 'INC', definition: '5 ADD' }];
        const skipped = [{ name: 'INC', reason: "Cannot redefine 'INC': referenced by INC2" }];
        const summary = summarizeImport(requested, skipped, ids({ INC: 'a' }), ids({ INC: 'a' }), null);
        expect(summary.unchanged).toEqual([]);
        expect(summary.added).toEqual([]);
        expect(summary.skipped).toEqual(skipped);
    });

    // An entry with no body is passed over by the interpreter and asked for
    // nothing; it used to be counted as imported.
    test('does not count a definition-less entry as imported', () => {
        const requested: UserWord[] = [
            { name: 'NO-BODY', definition: null },
            { name: 'REAL', definition: '[ 1 ]' },
        ];
        const summary = summarizeImport(requested, [], ids({}), ids({ REAL: 'r' }), null);
        expect(summary.added).toEqual(['REAL']);
        expect(summary.unchanged).toEqual([]);
    });

    // A word answers to either spelling, so matching is through the same
    // normalization the dictionary uses.
    test('matches the dictionary through the normalized name', () => {
        const requested: UserWord[] = [{ name: 'lower', definition: '[ 1 ]' }];
        const summary = summarizeImport(requested, [], ids({}), ids({ LOWER: 'l' }), null);
        expect(summary.added).toEqual(['lower']);
    });

    test('names a word whose embedded identity is not the identity it has here', () => {
        const requested: UserWord[] = [{ name: 'DBL', definition: '3 MUL' }];
        const summary = summarizeImport(requested, [], ids({ DBL: 'old' }), ids({ DBL: 'old' }), ids({ DBL: 'deadbeef' }));
        expect(summary.idMismatches).toEqual(['DBL']);
    });
});

// The saved session goes through the same entry check as an import file, so
// one malformed entry costs that entry rather than the whole dictionary.
describe('normalizeWordEntry', () => {
    test('keeps a well-formed entry', () => {
        expect(normalizeWordEntry({ name: 'A', definition: '1', description: 'd' })).toEqual({ name: 'A', definition: '1', description: 'd' });
    });

    for (const raw of [null, 1, 'A', { definition: '1' }, { name: 2 }, { name: null }]) {
        test(`drops ${JSON.stringify(raw)}`, () => {
            expect(normalizeWordEntry(raw)).toBeNull();
        });
    }

    test('reads a non-string definition as absent', () => {
        expect(normalizeWordEntry({ name: 'A', definition: 7 })).toEqual({ name: 'A', definition: null, description: undefined });
    });
});

// `createExportData` does not filter: every User Word it is given comes out.
describe('createExportData', () => {
    const fakeInterpreter = (words: string[]): AjisaiInterpreter => ({
        collect_user_words_info: () =>
            words.map(name => [name, false] as [string, boolean]),
        collect_word_identities: () =>
            words.map(name => [name, `id-${name}`] as [string, string]),
        lookup_word_definition: (name: string) => `[ '${name}' ]`,
        lookup_word_description: () => null,
    } as unknown as AjisaiInterpreter);

    test('exports every user word', () => {
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

describe('checkHasSavedDictionary', () => {
    test('treats an empty saved dictionary as a dictionary, so deleting every User Word survives a reload', () => {
        expect(checkHasSavedDictionary({ userWords: [] })).toBe(true);
    });

    test('treats a saved dictionary with words as a dictionary', () => {
        expect(checkHasSavedDictionary({ userWords: [{ name: 'SQ', definition: '[ 2 POW ]', description: null }] as never })).toBe(true);
    });

    test('seeds the Example Words only when no dictionary was saved at all', () => {
        expect(checkHasSavedDictionary({ userWords: undefined as never })).toBe(false);
    });
});
