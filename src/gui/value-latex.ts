// Math view for the Stack area: derives a LaTeX reading of a stack value
// from its structured protocol form (never by parsing display strings).
//
// This module is presentation only. The canonical display strings
// (`3/1`, `[ 1/1 2/1 ]`, the nested continued-fraction form) remain the
// observable semantics the conformance suite checks; the LaTeX produced
// here is an alternate GUI rendering of the same structured `Value`.
// Values without a faithful math reading return `null`, and the caller
// falls back to the canonical text rendering.

import type { Value, Fraction } from '../wasm-interpreter-types';

// Beyond this many numeric lanes a matrix stops being readable and the
// bracket text form is the better surface.
const MAX_MATH_LANES = 64;

const INTEGER_PATTERN = /^-?\d+$/;

const checkFractionShape = (value: unknown): Fraction | null => {
    if (!value || typeof value !== 'object') return null;
    const candidate = value as { numerator?: unknown; denominator?: unknown };
    const numerator = String(candidate.numerator ?? '');
    const denominator = String(candidate.denominator ?? '');
    if (!INTEGER_PATTERN.test(numerator) || !INTEGER_PATTERN.test(denominator)) return null;
    return { numerator, denominator };
};

// `3/1` reads as the integer 3; `-3/4` keeps its sign outside the bar.
export const fractionToLatex = (frac: Fraction): string => {
    if (frac.denominator === '1') return frac.numerator;
    const negative = frac.numerator.startsWith('-');
    const magnitude = negative ? frac.numerator.slice(1) : frac.numerator;
    const body = `\\frac{${magnitude}}{${frac.denominator}}`;
    return negative ? `-${body}` : body;
};

const laneToLatex = (lane: unknown): string => {
    const frac = checkFractionShape(lane);
    // An invalid dense lane is NIL occupancy (SPEC §4.3.1).
    return frac === null ? '\\mathrm{NIL}' : fractionToLatex(frac);
};

const rowsToMatrixLatex = (rows: string[][]): string => {
    const body = rows.map(row => row.join(' & ')).join(' \\\\ ');
    return `\\begin{bmatrix} ${body} \\end{bmatrix}`;
};

const tensorToLatex = (value: unknown): string | null => {
    if (!value || typeof value !== 'object') return null;
    const tensor = value as { shape?: unknown; data?: unknown; displayHint?: unknown };
    if (!Array.isArray(tensor.shape) || !Array.isArray(tensor.data)) return null;
    // Text-hinted byte tensors are strings, not mathematics.
    if (String(tensor.displayHint ?? '').toLowerCase() === 'text') return null;

    const shape = tensor.shape as number[];
    const data = tensor.data as unknown[];
    if (data.length === 0 || data.length > MAX_MATH_LANES) return null;

    if (shape.length === 0) return laneToLatex(data[0]);
    if (shape.length === 1) {
        return rowsToMatrixLatex([data.map(laneToLatex)]);
    }
    if (shape.length === 2) {
        const [rowCount, colCount] = [shape[0] ?? 0, shape[1] ?? 0];
        if (rowCount * colCount !== data.length || colCount === 0) return null;
        const rows: string[][] = [];
        for (let r = 0; r < rowCount; r++) {
            rows.push(data.slice(r * colCount, (r + 1) * colCount).map(laneToLatex));
        }
        return rowsToMatrixLatex(rows);
    }
    // Rank >= 3 has no flat matrix reading.
    return null;
};

const numberElementToLatex = (item: Value): string | null => {
    if (item.type !== 'number') return null;
    const frac = checkFractionShape(item.value);
    return frac === null ? null : fractionToLatex(frac);
};

const vectorToLatex = (elements: Value[]): string | null => {
    if (elements.length === 0 || elements.length > MAX_MATH_LANES) return null;

    // Homogeneous numeric vector: a one-row matrix.
    const scalarRow = elements.map(numberElementToLatex);
    if (scalarRow.every((tex): tex is string => tex !== null)) {
        return rowsToMatrixLatex([scalarRow]);
    }

    // Rectangular vector-of-numeric-vectors: a rank-2 matrix.
    const rows: string[][] = [];
    let width: number | null = null;
    for (const element of elements) {
        if (element.type !== 'vector' || !Array.isArray(element.value)) return null;
        const row = (element.value as Value[]).map(numberElementToLatex);
        if (!row.every((tex): tex is string => tex !== null)) return null;
        if (width === null) width = row.length;
        if (row.length !== width || width === 0) return null;
        rows.push(row);
    }
    if (rows.reduce((total, row) => total + row.length, 0) > MAX_MATH_LANES) return null;
    return rowsToMatrixLatex(rows);
};

// The LaTeX reading of a stack value, or `null` when the canonical text
// rendering is the only faithful surface.
export const valueToLatex = (item: Value): string | null => {
    if (!item || !item.type) return null;

    switch (item.type) {
        case 'number': {
            const frac = checkFractionShape(item.value);
            if (frac === null) return null;
            const tex = fractionToLatex(frac);
            // Best rational approximation of an exact irrational under a
            // lossy role (SPEC §2.3): make the approximation visible.
            const approximate = (item.semantics as { approximate?: boolean } | undefined)?.approximate === true;
            return approximate ? `\\approx ${tex}` : tex;
        }
        case 'tensor':
            return tensorToLatex(item.value);
        case 'vector':
            return Array.isArray(item.value) ? vectorToLatex(item.value as Value[]) : null;
        default:
            return null;
    }
};
