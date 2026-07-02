// Sheet view cell addressing (docs/dev/ajisai-spreadsheet-app-redesign-plan.md §3).
//
// Pure A1-form address arithmetic: A1 text ⇔ (col, row) coordinates and
// range expansion. UI- and WASM-free so the whole module is unit-testable
// under vitest (plan §7).
//
// A token is treated as a cell reference only when it parses as A1-form AND
// falls inside the grid limits. The limits double as the collision policy
// boundary (plan §3.2 / §9): with the phase-1 grid of 26 columns × 1000 rows,
// only single-letter columns A–Z with rows 1–1000 are cell references, so a
// user word like TAX2024 (column "TAX", row 2024 — out of bounds) is never
// rewritten.

export interface CellCoord {
    /** 0-based column index (column A = 0). */
    col: number;
    /** 0-based row index (row 1 = 0). */
    row: number;
}

export interface SheetGridLimits {
    /** Number of rows; row indices are valid in [0, rows). */
    rows: number;
    /** Number of columns; column indices are valid in [0, cols). */
    cols: number;
}

/** Phase-1 grid size (plan §5: "固定 1000 行 × 26 列程度"). */
export const DEFAULT_GRID_LIMITS: SheetGridLimits = { rows: 1000, cols: 26 };

const LETTER_A_CODE = 'A'.charCodeAt(0);
const ALPHABET_SIZE = 26;

// A1-form: one or more ASCII letters, then a row number without leading
// zeros. Case-insensitive on input; canonical form is uppercase.
const CELL_REF_PATTERN = /^([A-Za-z]+)([1-9][0-9]*)$/;

/**
 * Bijective base-26 column letters → 0-based index: A→0 … Z→25, AA→26.
 * Returns null for anything that is not a pure ASCII-letter run.
 */
export function columnLettersToIndex(letters: string): number | null {
    if (!/^[A-Za-z]+$/.test(letters)) {
        return null;
    }
    let index = 0;
    for (const ch of letters.toUpperCase()) {
        index = index * ALPHABET_SIZE + (ch.charCodeAt(0) - LETTER_A_CODE + 1);
    }
    return index - 1;
}

/** 0-based column index → bijective base-26 letters: 0→A … 25→Z, 26→AA. */
export function columnIndexToLetters(index: number): string {
    if (!Number.isInteger(index) || index < 0) {
        throw new RangeError(`column index must be a non-negative integer, got ${index}`);
    }
    let remaining = index + 1;
    let letters = '';
    while (remaining > 0) {
        const digit = (remaining - 1) % ALPHABET_SIZE;
        letters = String.fromCharCode(LETTER_A_CODE + digit) + letters;
        remaining = Math.floor((remaining - 1) / ALPHABET_SIZE);
    }
    return letters;
}

/**
 * Parse an A1-form token into coordinates. Returns null when the token is
 * not A1-form (this is the common "just a word name" case, not an error).
 * Row numbers with leading zeros (A01) are not cell references.
 */
export function parseCellRef(token: string): CellCoord | null {
    const match = CELL_REF_PATTERN.exec(token);
    if (!match) {
        return null;
    }
    const col = columnLettersToIndex(match[1] as string);
    if (col === null) {
        return null;
    }
    return { col, row: Number(match[2]) - 1 };
}

/** Canonical (uppercase) A1-form text for a coordinate. */
export function formatCellRef(coord: CellCoord): string {
    return `${columnIndexToLetters(coord.col)}${coord.row + 1}`;
}

export function isWithinLimits(coord: CellCoord, limits: SheetGridLimits): boolean {
    return coord.col >= 0 && coord.col < limits.cols && coord.row >= 0 && coord.row < limits.rows;
}

export interface CellRange {
    /** Top-left corner (normalized: start.col ≤ end.col, start.row ≤ end.row). */
    start: CellCoord;
    /** Bottom-right corner. */
    end: CellCoord;
}

/**
 * Parse an `A1:B3`-form range token. Both endpoints must be A1-form; the
 * result is normalized so `start` is the top-left corner regardless of the
 * order the user wrote the endpoints in (spreadsheet convention).
 */
export function parseRangeRef(token: string): CellRange | null {
    const colonIndex = token.indexOf(':');
    if (colonIndex < 0 || token.indexOf(':', colonIndex + 1) >= 0) {
        return null;
    }
    const first = parseCellRef(token.slice(0, colonIndex));
    const second = parseCellRef(token.slice(colonIndex + 1));
    if (!first || !second) {
        return null;
    }
    return {
        start: { col: Math.min(first.col, second.col), row: Math.min(first.row, second.row) },
        end: { col: Math.max(first.col, second.col), row: Math.max(first.row, second.row) },
    };
}

/**
 * Expand a normalized range into row-major rows of coordinates:
 * `A1:B2` → [[A1, B1], [A2, B2]]. The caller decides whether a single
 * row/column flattens to one Vector (plan §3.1).
 */
export function expandRangeRows(range: CellRange): CellCoord[][] {
    const rows: CellCoord[][] = [];
    for (let row = range.start.row; row <= range.end.row; row++) {
        const cells: CellCoord[] = [];
        for (let col = range.start.col; col <= range.end.col; col++) {
            cells.push({ col, row });
        }
        rows.push(cells);
    }
    return rows;
}

/** Number of cells a range covers, for expansion-size guards. */
export function rangeCellCount(range: CellRange): number {
    return (range.end.row - range.start.row + 1) * (range.end.col - range.start.col + 1);
}
