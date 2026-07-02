// Restore-path raw-text reconstruction tests (Phase 2 interim until the
// Phase 4 sheet-document persistence): the inverse of the engine's body
// generation must classify back to the same cell content.

import { describe, expect, test } from 'vitest';
import { reconstructCellText } from './formula-preprocessor';
import { classifyCellText } from './formula-preprocessor';

describe('reconstructCellText', () => {
    test('a number body comes back as the number literal', () => {
        expect(reconstructCellText('[ 42 ]')).toBe('42');
        expect(reconstructCellText('[ -1/3 ]')).toBe('-1/3');
        expect(classifyCellText(reconstructCellText('[ 42 ]')).kind).toBe('number');
    });

    test('a string body comes back as plain text', () => {
        expect(reconstructCellText("'hello world'")).toBe('hello world');
        expect(classifyCellText('hello world').kind).toBe('text');
    });

    test('ambiguous strings stay formulas instead of reclassifying', () => {
        // '42' as text would classify back to a number cell; keep it a formula.
        expect(reconstructCellText("'42'")).toBe("= '42'");
        // A leading = would classify back to a formula; keep it explicit.
        expect(reconstructCellText("'=A1'")).toBe("= '=A1'");
        // Inner quotes cannot safely round-trip through classify.
        expect(reconstructCellText("'it's'")).toBe("= 'it's'");
    });

    test('a formula body comes back with the = marker', () => {
        expect(reconstructCellText('TABLE1@A1 TABLE1@B1 +')).toBe('= TABLE1@A1 TABLE1@B1 +');
    });

    test('a vector body that is not a single number stays a formula', () => {
        expect(reconstructCellText('[ 1 2 3 ]')).toBe('= [ 1 2 3 ]');
    });
});
