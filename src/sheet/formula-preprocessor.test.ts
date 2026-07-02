// Sheet view formula preprocessing tests (plan §7: 前処理書換 unit tests).
//
// The rewrite is host-side surface preprocessing only; these tests pin that
// the transformation stays lexical (strings and comments untouched) and that
// its output is plain Ajisai source in the existing language (plan §3.1).

import { describe, expect, test } from 'vitest';
import {
    classifyCellText,
    formatTextCellLiteral,
    isAjisaiNumberLiteral,
    isCellShapedName,
    preprocessFormula,
} from './formula-preprocessor';

const SHEET = { sheetName: 'SHEET' };

describe('classifyCellText (plan §3.1: `=` opens a formula)', () => {
    test('empty and whitespace-only text is empty', () => {
        expect(classifyCellText('')).toEqual({ kind: 'empty' });
        expect(classifyCellText('   ')).toEqual({ kind: 'empty' });
    });

    test('leading = makes a formula, with the marker stripped', () => {
        expect(classifyCellText('= A1 B1 +')).toEqual({
            kind: 'formula',
            formulaSource: ' A1 B1 +',
        });
        expect(classifyCellText('  =A1')).toEqual({ kind: 'formula', formulaSource: 'A1' });
    });

    test('Ajisai number literals become number cells', () => {
        expect(classifyCellText('42')).toEqual({ kind: 'number', literal: '42' });
        expect(classifyCellText('-1/3')).toEqual({ kind: 'number', literal: '-1/3' });
        expect(classifyCellText('3.14')).toEqual({ kind: 'number', literal: '3.14' });
        expect(classifyCellText('1.5e2')).toEqual({ kind: 'number', literal: '1.5e2' });
        expect(classifyCellText('.5')).toEqual({ kind: 'number', literal: '.5' });
    });

    test('everything else is text', () => {
        expect(classifyCellText('hello')).toEqual({ kind: 'text', text: 'hello' });
        expect(classifyCellText('1/0x')).toEqual({ kind: 'text', text: '1/0x' });
        expect(classifyCellText('-')).toEqual({ kind: 'text', text: '-' });
    });
});

describe('isAjisaiNumberLiteral (mirror of the tokenizer number grammar)', () => {
    test('accepts the tokenizer number forms', () => {
        for (const text of ['0', '42', '-42', '+42', '1/3', '3.14', '5.', '.5', '-.5', '2e10', '1.5E-3']) {
            expect(isAjisaiNumberLiteral(text), text).toBe(true);
        }
    });

    test('rejects non-numbers the tokenizer also rejects', () => {
        for (const text of ['', '-', '+', '.', '1/0.5', '1/2e3', '1//2', 'e5', '1e', 'A1']) {
            expect(isAjisaiNumberLiteral(text), text).toBe(false);
        }
    });
});

describe('preprocessFormula: bare A1 rewrite (plan §3.1)', () => {
    test('rewrites in-bounds bare refs and collects references', () => {
        const result = preprocessFormula(' A1 B1 +', SHEET);
        expect(result.source).toBe(' SHEET@A1 SHEET@B1 +');
        expect(result.references).toEqual(['SHEET@A1', 'SHEET@B1']);
        expect(result.error).toBeNull();
    });

    test('normalizes lowercase refs to canonical uppercase', () => {
        const result = preprocessFormula('a1 b2 +', SHEET);
        expect(result.source).toBe('SHEET@A1 SHEET@B2 +');
    });

    test('leaves out-of-bounds A1-form tokens as word names (plan §3.2)', () => {
        const result = preprocessFormula('TAX2024 AA1 A1001 +', SHEET);
        expect(result.source).toBe('TAX2024 AA1 A1001 +');
        expect(result.references).toEqual([]);
    });

    test('deduplicates repeated references, keeping first-appearance order', () => {
        const result = preprocessFormula('B1 A1 B1 + +', SHEET);
        expect(result.references).toEqual(['SHEET@B1', 'SHEET@A1']);
    });

    test('does not rewrite inside string literals', () => {
        const result = preprocessFormula("'A1' A1 CONCAT", SHEET);
        expect(result.source).toBe("'A1' SHEET@A1 CONCAT");
        expect(result.references).toEqual(['SHEET@A1']);
    });

    test('honors the tokenizer string-close rule: quote before a non-delimiter stays inside', () => {
        // 'it's ok' is ONE string in Ajisai ('s is not a close delimiter).
        const result = preprocessFormula("'it's A1' B1", SHEET);
        expect(result.source).toBe("'it's A1' SHEET@B1");
        expect(result.references).toEqual(['SHEET@B1']);
    });

    test('does not rewrite inside comments', () => {
        const result = preprocessFormula('A1 # add B1 here\nB2', SHEET);
        expect(result.source).toBe('SHEET@A1 # add B1 here\nSHEET@B2');
        expect(result.references).toEqual(['SHEET@A1', 'SHEET@B2']);
    });

    test('special characters delimit tokens without spaces', () => {
        const result = preprocessFormula('[A1]{B1}', SHEET);
        expect(result.source).toBe('[SHEET@A1]{SHEET@B1}');
    });

    test('number literals are not cell refs', () => {
        const result = preprocessFormula('A1 [ 0 ] / 1/2 +', SHEET);
        expect(result.source).toBe('SHEET@A1 [ 0 ] / 1/2 +');
        expect(result.references).toEqual(['SHEET@A1']);
    });

    test('plan §3.1 examples pass through structurally intact', () => {
        expect(preprocessFormula(' A1:A10 [ 0 ] { + } FOLD', SHEET).source).toBe(
            ' [ SHEET@A1 SHEET@A2 SHEET@A3 SHEET@A4 SHEET@A5 SHEET@A6 SHEET@A7 SHEET@A8 SHEET@A9 SHEET@A10 ] [ 0 ] { + } FOLD',
        );
        expect(preprocessFormula(' B2 [ 0 ] / ^ [ 999 ]', SHEET).source).toBe(
            ' SHEET@B2 [ 0 ] / ^ [ 999 ]',
        );
    });
});

describe('preprocessFormula: ranges (plan §3.1: 範囲＝Vector)', () => {
    test('a column range expands to one flat Vector', () => {
        const result = preprocessFormula('A1:A3', SHEET);
        expect(result.source).toBe('[ SHEET@A1 SHEET@A2 SHEET@A3 ]');
        expect(result.references).toEqual(['SHEET@A1', 'SHEET@A2', 'SHEET@A3']);
    });

    test('a row range expands to one flat Vector', () => {
        const result = preprocessFormula('A1:C1', SHEET);
        expect(result.source).toBe('[ SHEET@A1 SHEET@B1 SHEET@C1 ]');
    });

    test('a rectangle nests one Vector per row', () => {
        const result = preprocessFormula('A1:B2', SHEET);
        expect(result.source).toBe('[ [ SHEET@A1 SHEET@B1 ] [ SHEET@A2 SHEET@B2 ] ]');
        expect(result.references).toEqual(['SHEET@A1', 'SHEET@B1', 'SHEET@A2', 'SHEET@B2']);
    });

    test('a reversed range normalizes to the same rectangle', () => {
        expect(preprocessFormula('A3:A1', SHEET).source).toBe(
            preprocessFormula('A1:A3', SHEET).source,
        );
    });

    test('an out-of-bounds endpoint leaves the token untouched', () => {
        const result = preprocessFormula('A1:A1001', SHEET);
        expect(result.source).toBe('A1:A1001');
        expect(result.references).toEqual([]);
    });

    test('an oversized range reports an error instead of exploding', () => {
        const result = preprocessFormula('A1:Z1000', SHEET);
        expect(result.error).toMatch(/26000 cells/);
        expect(result.source).toBe('A1:Z1000');
    });
});

describe('preprocessFormula: qualified names (plan §3.1: 多シート越境参照)', () => {
    test('a qualified cell stays as written but is recorded as a reference', () => {
        const result = preprocessFormula('SHEET2@B3 A1 +', SHEET);
        expect(result.source).toBe('SHEET2@B3 SHEET@A1 +');
        expect(result.references).toEqual(['SHEET2@B3', 'SHEET@A1']);
    });

    test('a qualified range expands against its own dictionary', () => {
        const result = preprocessFormula('SHEET2@A1:A2', SHEET);
        expect(result.source).toBe('[ SHEET2@A1 SHEET2@A2 ]');
        expect(result.references).toEqual(['SHEET2@A1', 'SHEET2@A2']);
    });

    test('a qualified non-cell word is not a cell reference', () => {
        const result = preprocessFormula('ALGO@SORT A1 +', SHEET);
        expect(result.source).toBe('ALGO@SORT SHEET@A1 +');
        expect(result.references).toEqual(['SHEET@A1']);
    });
});

describe('formatTextCellLiteral (no escaping mechanism exists)', () => {
    test('plain text round-trips', () => {
        expect(formatTextCellLiteral('hello world')).toEqual({
            literal: "'hello world'",
            error: null,
        });
    });

    test('an inner quote before a non-delimiter is representable', () => {
        expect(formatTextCellLiteral("it's fine")).toEqual({
            literal: "'it's fine'",
            error: null,
        });
    });

    test('a trailing quote is representable (closing quote is not a delimiter)', () => {
        expect(formatTextCellLiteral("rock'")).toEqual({ literal: "'rock''", error: null });
    });

    test('a quote followed by a space cannot round-trip', () => {
        const result = formatTextCellLiteral("it' s");
        expect(result.literal).toBeNull();
        expect(result.error).toMatch(/cannot be represented/);
    });
});

describe('isCellShapedName (plan §3.2: 逆方向の保護)', () => {
    test('in-bounds A1-form names are reserved for cells', () => {
        expect(isCellShapedName('A1')).toBe(true);
        expect(isCellShapedName('z1000')).toBe(true);
    });

    test('word names and out-of-bounds forms are free', () => {
        expect(isCellShapedName('TAX')).toBe(false);
        expect(isCellShapedName('TAX2024')).toBe(false);
        expect(isCellShapedName('AA1')).toBe(false);
    });
});
