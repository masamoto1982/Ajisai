// Sheet view formula preprocessing (docs/dev/ajisai-spreadsheet-app-redesign-plan.md §3).
//
// Host-side rewriting only — the language's resolution ladder is untouched:
//   - Cell text starting with `=` is a formula; anything else is a literal
//     (a number when it parses as an Ajisai number literal, a string
//     otherwise). The `=` marker never reaches the interpreter.
//   - Inside a formula, a bare in-bounds A1-form token is rewritten to the
//     qualified word `SHEET@A1`, and a range token `A1:A3` expands to a
//     Vector of qualified references. Everything else — strings, comments,
//     numbers, word names, already-qualified references — passes through
//     unchanged.
//
// The scanner mirrors the tokenizer's lexical rules (rust/src/tokenizer.rs):
// `'...'` string literals (closed by a quote followed by a delimiter),
// `#` line comments, and the special characters that terminate a token.
// Only bare word tokens are candidates for rewriting.

import {
    CellCoord,
    CellRange,
    DEFAULT_GRID_LIMITS,
    SheetGridLimits,
    expandRangeRows,
    formatCellRef,
    isWithinLimits,
    parseCellRef,
    parseRangeRef,
    rangeCellCount,
} from './cell-address';

/** Classification of raw cell text (plan §3.1). */
export type CellContent =
    | { kind: 'empty' }
    | { kind: 'formula'; formulaSource: string }
    | { kind: 'number'; literal: string }
    | { kind: 'text'; text: string };

/**
 * Classify what the user typed into a cell. `=` opens a formula (universal
 * spreadsheet convention); otherwise the text is a number literal when the
 * Ajisai tokenizer would read it as one, else a plain string.
 */
export function classifyCellText(rawText: string): CellContent {
    const trimmed = rawText.trim();
    if (trimmed === '') {
        return { kind: 'empty' };
    }
    if (trimmed.startsWith('=')) {
        return { kind: 'formula', formulaSource: trimmed.slice(1) };
    }
    if (isAjisaiNumberLiteral(trimmed)) {
        return { kind: 'number', literal: trimmed };
    }
    return { kind: 'text', text: trimmed };
}

// Mirror of rust/src/tokenizer.rs parse_number_from_string: optional sign,
// then either an integer fraction `a/b` (no exponent), or a decimal with an
// optional exponent. Leading-dot (`.5`) and trailing-dot (`5.`) decimals are
// numbers; a bare sign or dot is not.
const NUMBER_LITERAL_PATTERN =
    /^[+-]?(?:[0-9]+\/[0-9]+|(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?)$/;

export function isAjisaiNumberLiteral(text: string): boolean {
    return NUMBER_LITERAL_PATTERN.test(text);
}

// Special characters from the tokenizer's is_special_char: they terminate a
// bare token. `;` (modifier sugar) also splits, and whitespace always splits.
const TOKEN_BOUNDARY_CHARS = new Set([
    '[', ']', '{', '}', '(', ')', '#', "'", '>', '=', '|', '~', '^', ';',
]);

function isTokenBoundary(ch: string): boolean {
    return TOKEN_BOUNDARY_CHARS.has(ch) || /\s/.test(ch);
}

// The tokenizer closes a `'...'` literal at a quote followed by end-of-input
// or a close delimiter (whitespace, or a special char other than `'`).
function isStringCloseDelimiter(ch: string): boolean {
    return /\s/.test(ch) || (TOKEN_BOUNDARY_CHARS.has(ch) && ch !== "'" && ch !== ';');
}

export interface FormulaPreprocessOptions {
    /** Dictionary name of the sheet the formula lives in (e.g. 'SHEET'). */
    sheetName: string;
    /** Grid bounds; tokens outside them are word names, not cell refs. */
    limits?: SheetGridLimits;
}

/**
 * Guard against pathological range expansions (`A1:Z1000`); a formula that
 * needs this much data should read whole columns via a word instead.
 */
export const MAX_RANGE_EXPANSION_CELLS = 10_000;

export interface PreprocessedFormula {
    /** The rewritten Ajisai source, ready for the interpreter. */
    source: string;
    /**
     * Fully-qualified cell words this formula reads (`SHEET@A1`), deduped in
     * first-appearance order. This is the host-side dependency-graph input:
     * it is textual, so a reference to a still-empty cell is recorded even
     * though the interpreter cannot resolve (and so cannot index) it yet.
     */
    references: string[];
    /** First preprocessing error, if any (e.g. an oversized range). */
    error: string | null;
}

/**
 * Rewrite a formula body (the text after `=`) for the interpreter:
 * bare in-bounds `A1` → `SHEET@A1`, bare `A1:B2` → Vector of qualified
 * references (row-major nesting for rectangles, flat for a single row or
 * column). Qualified tokens (`SHEET2@B3`, `SHEET2@A1:A3`) resolve against
 * their own dictionary for cross-sheet references.
 */
export function preprocessFormula(
    formulaSource: string,
    options: FormulaPreprocessOptions,
): PreprocessedFormula {
    const limits = options.limits ?? DEFAULT_GRID_LIMITS;
    const sheetName = options.sheetName.toUpperCase();
    const references: string[] = [];
    const seen = new Set<string>();
    let error: string | null = null;

    const addReference = (fqName: string): void => {
        if (!seen.has(fqName)) {
            seen.add(fqName);
            references.push(fqName);
        }
    };

    const qualify = (dictionary: string, coord: CellCoord): string =>
        `${dictionary}@${formatCellRef(coord)}`;

    const expandRange = (dictionary: string, range: CellRange): string | null => {
        if (rangeCellCount(range) > MAX_RANGE_EXPANSION_CELLS) {
            if (error === null) {
                error =
                    `Range expands to ${rangeCellCount(range)} cells ` +
                    `(limit ${MAX_RANGE_EXPANSION_CELLS})`;
            }
            return null;
        }
        const rows = expandRangeRows(range);
        const rowTexts = rows.map((rowCells) => {
            const refs = rowCells.map((coord) => {
                const fqName = qualify(dictionary, coord);
                addReference(fqName);
                return fqName;
            });
            return refs.join(' ');
        });
        // A single row or column is one flat Vector; a rectangle nests one
        // Vector per row (plan §3.1: 矩形範囲は入れ子 Vector).
        if (rows.length === 1) {
            return `[ ${rowTexts[0]} ]`;
        }
        if ((rows[0] as CellCoord[]).length === 1) {
            return `[ ${rowTexts.join(' ')} ]`;
        }
        return `[ ${rowTexts.map((text) => `[ ${text} ]`).join(' ')} ]`;
    };

    const rewriteToken = (token: string): string => {
        const atIndex = token.indexOf('@');
        if (atIndex > 0 && token.indexOf('@', atIndex + 1) < 0) {
            // Qualified token: `DICT@A1` stays as written (it is already a
            // valid word call) but is recorded as a dependency; a qualified
            // range `DICT@A1:B2` expands against that dictionary.
            const dictionary = token.slice(0, atIndex).toUpperCase();
            const rest = token.slice(atIndex + 1);
            if (!/^[A-Za-z][A-Za-z0-9_-]*$/.test(dictionary)) {
                return token;
            }
            const cell = parseCellRef(rest);
            if (cell && isWithinLimits(cell, limits)) {
                addReference(qualify(dictionary, cell));
                return token;
            }
            const range = parseRangeRef(rest);
            if (
                range &&
                isWithinLimits(range.start, limits) &&
                isWithinLimits(range.end, limits)
            ) {
                return expandRange(dictionary, range) ?? token;
            }
            return token;
        }

        const cell = parseCellRef(token);
        if (cell && isWithinLimits(cell, limits)) {
            const fqName = qualify(sheetName, cell);
            addReference(fqName);
            return fqName;
        }

        const range = parseRangeRef(token);
        if (range && isWithinLimits(range.start, limits) && isWithinLimits(range.end, limits)) {
            return expandRange(sheetName, range) ?? token;
        }

        return token;
    };

    let output = '';
    let tokenStart = -1;
    let i = 0;
    const flushToken = (end: number): void => {
        if (tokenStart >= 0) {
            output += rewriteToken(formulaSource.slice(tokenStart, end));
            tokenStart = -1;
        }
    };

    while (i < formulaSource.length) {
        const ch = formulaSource[i] as string;

        if (tokenStart < 0 && ch === "'") {
            // String literal: copy verbatim until the tokenizer's close rule
            // fires (quote + delimiter/end). Unclosed strings are copied
            // through; the interpreter reports them with its own diagnosis.
            let j = i + 1;
            while (j < formulaSource.length) {
                if (
                    formulaSource[j] === "'" &&
                    (j + 1 >= formulaSource.length ||
                        isStringCloseDelimiter(formulaSource[j + 1] as string))
                ) {
                    j++;
                    break;
                }
                j++;
            }
            output += formulaSource.slice(i, j);
            i = j;
            continue;
        }

        if (tokenStart < 0 && ch === '#') {
            // Line comment: copy verbatim to end of line.
            let j = i;
            while (j < formulaSource.length && formulaSource[j] !== '\n') {
                j++;
            }
            output += formulaSource.slice(i, j);
            i = j;
            continue;
        }

        if (isTokenBoundary(ch)) {
            flushToken(i);
            output += ch;
            i++;
            continue;
        }

        if (tokenStart < 0) {
            tokenStart = i;
        }
        i++;
    }
    flushToken(formulaSource.length);

    return { source: output, references, error };
}

/**
 * Render cell text as an Ajisai string literal, or report why it cannot be
 * (plan §2.1 keeps cell values exact — no escaping mechanism exists, so a
 * quote followed by a delimiter cannot round-trip through `'...'`).
 */
export function formatTextCellLiteral(
    text: string,
): { literal: string; error: null } | { literal: null; error: string } {
    for (let i = 0; i < text.length; i++) {
        if (text[i] === "'") {
            const next = text[i + 1];
            if (next !== undefined && isStringCloseDelimiter(next)) {
                return {
                    literal: null,
                    error:
                        "Text containing a quote followed by a delimiter cannot be " +
                        "represented as an Ajisai string literal",
                };
            }
        }
    }
    return { literal: `'${text}'`, error: null };
}

/**
 * Reconstruct a cell's editable text from its word body — the inverse of
 * the engine's body generation, used on restore until Phase 4 persists the
 * sheet document: `[ 42 ]` → `42`, `'hello'` → `hello`, anything else →
 * `= <body>`. Restored formula bodies carry qualified references
 * (`TABLE1@A1`) — stable through re-preprocessing, just more explicit than
 * what was originally typed. Ambiguous strings (text that would classify
 * back to a number or formula) stay explicit formulas.
 */
export function reconstructCellText(bodySource: string): string {
    const trimmed = bodySource.trim();
    const numberMatch = /^\[\s*(\S+)\s*\]$/.exec(trimmed);
    if (numberMatch && isAjisaiNumberLiteral(numberMatch[1] as string)) {
        return numberMatch[1] as string;
    }
    const stringMatch = /^'([^]*)'$/.exec(trimmed);
    if (stringMatch !== null) {
        const text = stringMatch[1] as string;
        if (!text.startsWith('=') && !text.includes("'") && !isAjisaiNumberLiteral(text)) {
            return text;
        }
    }
    return `= ${trimmed}`;
}

/**
 * Sheet-view protection (plan §3.2): an in-bounds A1-form name is reserved
 * for cells; the host refuses to DEF it for anything else.
 */
export function isCellShapedName(
    name: string,
    limits: SheetGridLimits = DEFAULT_GRID_LIMITS,
): boolean {
    const cell = parseCellRef(name);
    return cell !== null && isWithinLimits(cell, limits);
}
