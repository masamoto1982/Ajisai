// Where the Sheet puts each User Word: a view the session keeps, never a
// second dictionary.

import { describe, expect, it } from 'vitest';
import {
    columnName,
    formatAddress,
    gridExtent,
    parseAddress,
    placeWords,
    readSavedLayout
} from './user-sheet-layout';

describe('cell addresses', () => {
    it('names columns as a spreadsheet does and reads them back', () => {
        expect([0, 25, 26, 27, 51, 701].map(columnName)).toEqual(['A', 'Z', 'AA', 'AB', 'AZ', 'ZZ']);
        for (const address of ['A1', 'Z9', 'AA10', 'ZZ9999']) {
            const cell = parseAddress(address);
            expect(cell).not.toBeNull();
            expect(formatAddress(cell!)).toBe(address);
        }
        expect(parseAddress('b3')).toEqual({ column: 1, row: 2 });
        for (const text of ['A0', 'AAA1', 'A10000', 'PRICE', '1A', '']) expect(parseAddress(text)).toBeNull();
    });
});

describe('placing Words on the grid', () => {
    it('keeps a saved place, stands an address-named Word at its address, and stacks the rest in column A', () => {
        const placed = placeWords(['TOTAL', 'B2', 'PRICE', 'QTY'], { PRICE: 'C1', GONE: 'D4' });
        expect(placed).toEqual({ PRICE: 'C1', B2: 'B2', TOTAL: 'A1', QTY: 'A2' });
    });

    it('never puts two Words in one cell', () => {
        const placed = placeWords(['A1', 'X', 'Y'], { X: 'A1', Y: 'A1' });
        expect(placed).toEqual({ X: 'A1', A1: 'A2', Y: 'A3' });
        expect(new Set(Object.values(placed)).size).toBe(3);
    });

    it('sizes the grid to what is placed, with room to grow', () => {
        expect(gridExtent({})).toEqual({ columns: 6, rows: 12 });
        expect(gridExtent({ X: 'H20' })).toEqual({ columns: 10, rows: 23 });
    });

    it('reads a saved layout as untrusted', () => {
        expect(readSavedLayout({ A: 'b2', B: 'nowhere', C: 3 })).toEqual({ A: 'B2' });
        for (const value of [null, 'A1', ['A1'], undefined]) expect(readSavedLayout(value)).toEqual({});
    });
});
