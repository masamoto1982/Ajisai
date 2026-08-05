// Recall of submitted programs. A successful Run empties the editor and Reset
// empties it too, so the only way back to what you just wrote is this history.

import { describe, expect, test } from 'vitest';
import { MAX_HISTORY_ENTRIES, createEditorHistory } from './editor-history';

describe('createEditorHistory', () => {
    test('walks back through submitted programs, oldest last', () => {
        const history = createEditorHistory();
        history.record('1 2 +');
        history.record('3 4 *');

        expect(history.recallOlder('')).toBe('3 4 *');
        expect(history.recallOlder('')).toBe('1 2 +');
    });

    test('stops at the oldest entry instead of wrapping', () => {
        const history = createEditorHistory();
        history.record('1 2 +');

        expect(history.recallOlder('')).toBe('1 2 +');
        expect(history.recallOlder('')).toBeNull();
    });

    test('stepping forward returns the draft that was being typed', () => {
        const history = createEditorHistory();
        history.record('1 2 +');

        expect(history.recallOlder('3 4')).toBe('1 2 +');
        expect(history.recallNewer()).toBe('3 4');
    });

    test('there is nothing newer than the draft', () => {
        const history = createEditorHistory();
        history.record('1 2 +');

        expect(history.recallNewer()).toBeNull();
    });

    test('recording rewinds the cursor, so the next recall is the newest entry', () => {
        const history = createEditorHistory();
        history.record('1 2 +');
        history.recallOlder('');
        history.record('3 4 *');

        expect(history.recallOlder('')).toBe('3 4 *');
    });

    test('an empty or blank submission is not recorded', () => {
        const history = createEditorHistory();
        history.record('   ');
        history.record('');

        expect(history.entries()).toEqual([]);
        expect(history.recallOlder('')).toBeNull();
    });

    test('re-running the same program does not bury the history in duplicates', () => {
        const history = createEditorHistory();
        history.record('1 2 +');
        history.record('1 2 +');
        history.record('1 2 +');

        expect(history.entries()).toEqual(['1 2 +']);
    });

    test('entries are stored trimmed, as submitted rather than as laid out', () => {
        const history = createEditorHistory();
        history.record('  1 2 +\n');

        expect(history.entries()).toEqual(['1 2 +']);
    });

    test('the oldest entries drop once the limit is reached', () => {
        const history = createEditorHistory(3);
        for (const source of ['A', 'B', 'C', 'D']) history.record(source);

        expect(history.entries()).toEqual(['B', 'C', 'D']);
    });

    test('the default limit is the documented one', () => {
        const history = createEditorHistory();
        for (let i = 0; i <= MAX_HISTORY_ENTRIES; i++) history.record(`${i} 1 +`);

        expect(history.entries()).toHaveLength(MAX_HISTORY_ENTRIES);
        expect(history.entries()[0]).toBe('1 1 +');
    });

    test('an empty history recalls nothing in either direction', () => {
        const history = createEditorHistory();

        expect(history.recallOlder('draft')).toBeNull();
        expect(history.recallNewer()).toBeNull();
    });
});
