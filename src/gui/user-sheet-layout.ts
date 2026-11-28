// Where the Sheet puts each User Word on its grid (spec/gui-semantics.md,
// Sheet). The position of a cell is the only thing the sheet holds of its
// own, and it is a view: the Words are the dictionary's, and a layout that
// names a Word the dictionary no longer holds simply forgets it.

/** A cell address as a spreadsheet writes it: a column of letters, a row from 1. */
export interface CellAddress {
    readonly column: number;
    readonly row: number;
}

/** Each placed Word's address, by name, as the session saves it. */
export type SheetLayout = Readonly<Record<string, string>>;

const MAX_COLUMN = 26 * 27 - 1; // ZZ
const MAX_ROW = 9999;

/** 0 is A, 25 is Z, 26 is AA. */
export const columnName = (column: number): string => {
    const letter = (n: number): string => String.fromCharCode(65 + n);
    return column < 26 ? letter(column) : letter(Math.floor(column / 26) - 1) + letter(column % 26);
};

export const formatAddress = ({ column, row }: CellAddress): string => `${columnName(column)}${row + 1}`;

/** `B3` as column 1, row 2; anything that is not an address answers null. */
export const parseAddress = (text: string): CellAddress | null => {
    const match = /^([A-Z]{1,2})([1-9][0-9]{0,3})$/.exec(text.toUpperCase());
    const letters = match?.[1];
    const digits = match?.[2];
    if (letters === undefined || digits === undefined) return null;
    const column = letters.length === 1
        ? letters.charCodeAt(0) - 65
        : (letters.charCodeAt(0) - 64) * 26 + letters.charCodeAt(1) - 65;
    const row = Number(digits) - 1;
    return column <= MAX_COLUMN && row < MAX_ROW ? { column, row } : null;
};

/**
 * Every Word given an address, no two sharing one. A Word keeps the address
 * the layout gave it; a Word named like an address (`B3`) stands there; any
 * other Word takes the first free row of column A. The answer is the layout
 * to keep, so a Word placed once stays where it was put.
 */
export const placeWords = (names: readonly string[], saved: SheetLayout): Record<string, string> => {
    const placed: Record<string, string> = {};
    const taken = new Set<string>();
    const claim = (name: string, address: string): boolean => {
        if (taken.has(address)) return false;
        taken.add(address);
        placed[name] = address;
        return true;
    };
    const unplaced = names.filter((name) => {
        const address = saved[name];
        return !(address !== undefined && parseAddress(address) && claim(name, address));
    });
    const rest = unplaced.filter((name) => !(parseAddress(name) && claim(name, name.toUpperCase())));
    let row = 0;
    for (const name of rest) {
        while (taken.has(formatAddress({ column: 0, row }))) row += 1;
        claim(name, formatAddress({ column: 0, row }));
    }
    return placed;
};

/** The grid's size: what is placed, a margin to grow into, and a floor. */
export const gridExtent = (layout: SheetLayout): { columns: number; rows: number } => {
    let columns = 0;
    let rows = 0;
    for (const address of Object.values(layout)) {
        const cell = parseAddress(address);
        if (!cell) continue;
        columns = Math.max(columns, cell.column + 1);
        rows = Math.max(rows, cell.row + 1);
    }
    return {
        columns: Math.min(Math.max(6, columns + 2), MAX_COLUMN + 1),
        rows: Math.min(Math.max(12, rows + 3), MAX_ROW)
    };
};

/** A saved layout read back as untrusted: only name-to-address strings survive. */
export const readSavedLayout = (value: unknown): Record<string, string> => {
    if (value === null || typeof value !== 'object' || Array.isArray(value)) return {};
    const layout: Record<string, string> = {};
    for (const [name, address] of Object.entries(value as Record<string, unknown>)) {
        if (typeof address === 'string' && parseAddress(address)) layout[name] = address.toUpperCase();
    }
    return layout;
};
