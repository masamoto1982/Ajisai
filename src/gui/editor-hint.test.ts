import { describe, expect, it } from 'vitest';
import {
    DESKTOP_EDITOR_HINT,
    MOBILE_EDITOR_HINT,
    formatHintSheetText,
    parseKeyChords
} from './editor-hint';

describe('parseKeyChords', () => {
    it('splits alternatives and the keys of each chord', () => {
        expect(parseKeyChords('Ctrl+Up / Ctrl+Down')).toEqual([['Ctrl', 'Up'], ['Ctrl', 'Down']]);
        expect(parseKeyChords('Escape')).toEqual([['Escape']]);
    });
});

describe('formatHintSheetText', () => {
    it('aligns each group and keeps notes and headings as their own lines', () => {
        expect(formatHintSheetText({
            lead: 'Enter code here',
            groups: [
                { entries: [{ label: 'run', keys: 'Shift+Enter' }, { label: 'format', gesture: 'icon' }] },
                { heading: 'notes:', entries: [{ label: 'a note' }] }
            ]
        })).toBe([
            'Enter code here',
            '',
            'run    → Shift+Enter',
            'format → icon',
            '',
            'notes:',
            'a note'
        ].join('\n'));
    });

    it('starts both sheets with the lead line', () => {
        for (const sheet of [DESKTOP_EDITOR_HINT, MOBILE_EDITOR_HINT]) {
            expect(formatHintSheetText(sheet).split('\n')[0]).toBe('Enter code here');
        }
    });
});
