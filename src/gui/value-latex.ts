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

// Digit count at which a numerator or denominator stops being readable as
// a digit string and the math view switches to scientific notation. Same
// threshold as the text renderer's `formatFractionScientific`.
const SCIENTIFIC_DIGIT_THRESHOLD = 10;
const MANTISSA_DIGITS = 6;

const checkFractionShape = (value: unknown): Fraction | null => {
    if (!value || typeof value !== 'object') return null;
    const candidate = value as { numerator?: unknown; denominator?: unknown };
    const numerator = String(candidate.numerator ?? '');
    const denominator = String(candidate.denominator ?? '');
    if (!INTEGER_PATTERN.test(numerator) || !INTEGER_PATTERN.test(denominator)) return null;
    // A zero denominator is not a faithful rational (it is NIL occupancy /
    // malformed state, never a canonical number). Reject it here so the math
    // view falls back to the canonical text rendering instead of dividing by
    // zero inside `scientificLatex`. Matches INTEGER_PATTERN-allowed forms like
    // "0", "-0" and "0000000000".
    if (/^-?0+$/.test(denominator)) return null;
    return { numerator, denominator };
};

// Scientific reading of a huge ratio: mantissa times a power of ten,
// computed exactly with BigInt long division and prefixed with \approx
// whenever any precision is dropped — the math view never presents a
// truncated value as exact.
const scientificLatex = (numeratorStr: string, denominatorStr: string): string => {
    let numerator = BigInt(numeratorStr);
    let denominator = BigInt(denominatorStr);
    // Defensive: a zero denominator would divide by zero below. Internal callers
    // are pre-filtered by `checkFractionShape`, but `fractionToLatex` is exported
    // and may be called directly, so keep this primitive total.
    if (denominator === 0n) return '\\mathrm{NIL}';
    if (denominator < 0n) {
        denominator = -denominator;
        numerator = -numerator;
    }
    const negative = numerator < 0n;
    if (negative) numerator = -numerator;
    if (numerator === 0n) return '0';

    // Scale so the quotient carries one digit beyond the mantissa, then
    // read mantissa and exponent off the quotient's decimal digits.
    const digitGap = String(numerator).length - String(denominator).length;
    const scale = MANTISSA_DIGITS + 1 - digitGap;
    const scaled = scale >= 0
        ? (numerator * 10n ** BigInt(scale)) / denominator
        : numerator / (denominator * 10n ** BigInt(-scale));
    const dividesExactly = scale >= 0
        ? (numerator * 10n ** BigInt(scale)) % denominator === 0n
        : numerator % (denominator * 10n ** BigInt(-scale)) === 0n;

    const digits = String(scaled);
    const exponent = digits.length - 1 - scale;
    const kept = digits.slice(0, MANTISSA_DIGITS);
    const dropped = digits.slice(MANTISSA_DIGITS);
    const exact = dividesExactly && /^0*$/.test(dropped);

    let significand = kept;
    let exponentOut = exponent;
    if (!exact && dropped.length > 0 && dropped[0]! >= '5') {
        // Round half-up on the first dropped digit; a carry out of the top
        // digit (9.99999... -> 10) bumps the exponent instead.
        const rounded = String(BigInt(kept) + 1n);
        if (rounded.length > kept.length) {
            significand = '1';
            exponentOut = exponent + 1;
        } else {
            significand = rounded;
        }
    }
    significand = significand.replace(/0+$/, '') || '0';

    // Huge components do not imply a huge value (a best rational
    // approximation of sqrt(2) has ten-digit components and the value 1.41…),
    // so a human-scale exponent renders as a plain decimal and only a
    // genuinely large or tiny value gets the power of ten.
    const sign = negative ? '-' : '';
    let body: string;
    if (exponentOut >= 0 && exponentOut <= 5) {
        const integerLength = exponentOut + 1;
        const padded = significand.padEnd(integerLength, '0');
        const integerPart = padded.slice(0, integerLength);
        const fractionalPart = padded.slice(integerLength);
        body = `${sign}${integerPart}${fractionalPart ? `.${fractionalPart}` : ''}`;
    } else if (exponentOut < 0 && exponentOut >= -4) {
        body = `${sign}0.${'0'.repeat(-exponentOut - 1)}${significand}`;
    } else {
        const mantissa = significand.length > 1
            ? `${significand[0]}.${significand.slice(1)}`
            : significand;
        body = mantissa === '1'
            ? `${sign}10^{${exponentOut}}`
            : `${sign}${mantissa} \\times 10^{${exponentOut}}`;
    }
    return exact ? body : `\\approx ${body}`;
};

const checkHugeDigits = (frac: Fraction): boolean => {
    const numeratorDigits = frac.numerator.replace('-', '').length;
    const denominatorDigits = frac.denominator.replace('-', '').length;
    return numeratorDigits >= SCIENTIFIC_DIGIT_THRESHOLD
        || denominatorDigits >= SCIENTIFIC_DIGIT_THRESHOLD;
};

// `3/1` reads as the integer 3; `-3/4` keeps its sign outside the bar.
// Huge components switch to scientific notation so the rendering stays
// inside the Stack area instead of running off its right edge.
export const fractionToLatex = (frac: Fraction): string => {
    if (checkHugeDigits(frac)) return scientificLatex(frac.numerator, frac.denominator);
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
            // lossy role (SPEC §2.3): make the approximation visible. The
            // scientific form may already carry its own \approx.
            const approximate = (item.semantics as { approximate?: boolean } | undefined)?.approximate === true;
            return approximate && !tex.startsWith('\\approx') ? `\\approx ${tex}` : tex;
        }
        case 'tensor':
            return tensorToLatex(item.value);
        case 'vector':
            return Array.isArray(item.value) ? vectorToLatex(item.value as Value[]) : null;
        default:
            return null;
    }
};
