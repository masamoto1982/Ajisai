// Sheet view cell addressing tests (plan §7: アドレス変換 unit tests).

import { describe, expect, test } from 'vitest';
import {
    DEFAULT_GRID_LIMITS,
    columnIndexToLetters,
    columnLettersToIndex,
    expandRangeRows,
    formatCellRef,
    isWithinLimits,
    parseCellRef,
    parseRangeRef,
    rangeCellCount,
} from './cell-address';

describe('column letters ⇔ index (bijective base-26)', () => {
    test('single letters map A→0 … Z→25', () => {
        expect(columnLettersToIndex('A')).toBe(0);
        expect(columnLettersToIndex('Z')).toBe(25);
    });

    test('multi-letter columns continue at AA→26, AZ→51, BA→52', () => {
        expect(columnLettersToIndex('AA')).toBe(26);
        expect(columnLettersToIndex('AZ')).toBe(51);
        expect(columnLettersToIndex('BA')).toBe(52);
    });

    test('lowercase input is accepted', () => {
        expect(columnLettersToIndex('aa')).toBe(26);
    });

    test('non-letter input returns null', () => {
        expect(columnLettersToIndex('A1')).toBeNull();
        expect(columnLettersToIndex('')).toBeNull();
    });

    test('round-trips through columnIndexToLetters', () => {
        for (const index of [0, 25, 26, 51, 52, 701, 702]) {
            expect(columnLettersToIndex(columnIndexToLetters(index))).toBe(index);
        }
        expect(columnIndexToLetters(701)).toBe('ZZ');
        expect(columnIndexToLetters(702)).toBe('AAA');
    });

    test('negative or fractional index throws', () => {
        expect(() => columnIndexToLetters(-1)).toThrow(RangeError);
        expect(() => columnIndexToLetters(1.5)).toThrow(RangeError);
    });
});

describe('parseCellRef / formatCellRef', () => {
    test('parses A1 to origin', () => {
        expect(parseCellRef('A1')).toEqual({ col: 0, row: 0 });
    });

    test('parses lowercase and formats back canonical uppercase', () => {
        const coord = parseCellRef('b12');
        expect(coord).toEqual({ col: 1, row: 11 });
        expect(formatCellRef(coord as { col: number; row: number })).toBe('B12');
    });

    test('rejects non-A1 tokens: word names, ranges, leading zeros', () => {
        expect(parseCellRef('SUM')).toBeNull();
        expect(parseCellRef('A0')).toBeNull();
        expect(parseCellRef('A01')).toBeNull();
        expect(parseCellRef('A1:B2')).toBeNull();
        expect(parseCellRef('A1B')).toBeNull();
        expect(parseCellRef('1A')).toBeNull();
        expect(parseCellRef('')).toBeNull();
    });
});

describe('isWithinLimits (collision boundary, plan §3.2)', () => {
    test('phase-1 grid accepts A1 and Z1000, rejects AA1 and A1001', () => {
        expect(isWithinLimits({ col: 0, row: 0 }, DEFAULT_GRID_LIMITS)).toBe(true);
        expect(isWithinLimits({ col: 25, row: 999 }, DEFAULT_GRID_LIMITS)).toBe(true);
        expect(isWithinLimits({ col: 26, row: 0 }, DEFAULT_GRID_LIMITS)).toBe(false);
        expect(isWithinLimits({ col: 0, row: 1000 }, DEFAULT_GRID_LIMITS)).toBe(false);
    });

    test('a year-suffixed word name like TAX2024 falls outside the grid', () => {
        const coord = parseCellRef('TAX2024');
        expect(coord).not.toBeNull();
        expect(isWithinLimits(coord as { col: number; row: number }, DEFAULT_GRID_LIMITS)).toBe(
            false,
        );
    });
});

describe('parseRangeRef / expandRangeRows', () => {
    test('parses and normalizes a reversed range', () => {
        expect(parseRangeRef('B3:A1')).toEqual({
            start: { col: 0, row: 0 },
            end: { col: 1, row: 2 },
        });
    });

    test('rejects malformed ranges', () => {
        expect(parseRangeRef('A1')).toBeNull();
        expect(parseRangeRef('A1:')).toBeNull();
        expect(parseRangeRef(':A1')).toBeNull();
        expect(parseRangeRef('A1:B2:C3')).toBeNull();
        expect(parseRangeRef('A1:SUM')).toBeNull();
    });

    test('expands row-major', () => {
        const range = parseRangeRef('A1:B2');
        expect(expandRangeRows(range as NonNullable<typeof range>)).toEqual([
            [
                { col: 0, row: 0 },
                { col: 1, row: 0 },
            ],
            [
                { col: 0, row: 1 },
                { col: 1, row: 1 },
            ],
        ]);
    });

    test('rangeCellCount covers the full rectangle', () => {
        const range = parseRangeRef('A1:C4');
        expect(rangeCellCount(range as NonNullable<typeof range>)).toBe(12);
    });
});
