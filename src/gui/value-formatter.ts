

import type { Value, Fraction } from '../wasm-interpreter-types';


const SCIENTIFIC_THRESHOLD = 10;
const MANTISSA_PRECISION = 6;


function formatIntegerScientific(numStr: string): string {
    const isNegative = numStr.startsWith('-');
    const absNumStr = isNegative ? numStr.substring(1) : numStr;

    if (absNumStr.length < SCIENTIFIC_THRESHOLD) {
        return numStr;
    }

    const firstDigit = absNumStr[0];
    const remainingDigits = absNumStr.substring(1);
    const exponent = remainingDigits.length;

    let mantissa = firstDigit!;
    if (remainingDigits.length > 0) {
        const fractionalDigits = Math.min(MANTISSA_PRECISION - 1, remainingDigits.length);
        if (fractionalDigits > 0) {
            mantissa += '.' + remainingDigits.substring(0, fractionalDigits);
        }
    }

    mantissa = mantissa.replace(/\.?0+$/, '');
    if (isNegative) mantissa = '-' + mantissa;

    return `${mantissa}e${exponent}`;
}


export function formatFractionScientific(numerStr: string, denomStr: string): string {
    if (denomStr === '1') {
        return formatIntegerScientific(numerStr);
    }

    const numSci = formatIntegerScientific(numerStr);
    const denSci = formatIntegerScientific(denomStr);

    if (numSci.includes('e') && denSci.includes('e')) {
        const numMatch = numSci.match(/^([+-]?\d+\.?\d*)e([+-]?\d+)$/);
        const denMatch = denSci.match(/^([+-]?\d+\.?\d*)e([+-]?\d+)$/);

        if (numMatch && denMatch) {
            const numMantissa = parseFloat(numMatch[1]!);
            const numExponent = parseInt(numMatch[2]!);
            const denMantissa = parseFloat(denMatch[1]!);
            const denExponent = parseInt(denMatch[2]!);

            let resultMantissa = numMantissa / denMantissa;
            let resultExponent = numExponent - denExponent;

            // A zero (or non-finite) denominator mantissa makes resultMantissa
            // Infinity/NaN, and the normalization loops below would spin forever
            // (Infinity / 10 === Infinity). Fall back to the plain ratio form
            // rather than hang. (Canonical numbers never have a zero
            // denominator, but this helper is exported and may see malformed
            // or restored state.)
            if (!Number.isFinite(resultMantissa)) {
                return `${numSci}/${denSci}`;
            }

            while (Math.abs(resultMantissa) >= 10) {
                resultMantissa /= 10;
                resultExponent += 1;
            }
            while (Math.abs(resultMantissa) < 1 && resultMantissa !== 0) {
                resultMantissa *= 10;
                resultExponent -= 1;
            }

            const rounded = resultMantissa.toPrecision(MANTISSA_PRECISION);
            return resultExponent === 0 ? rounded : `${rounded}e${resultExponent}`;
        }
    }

    return `${numSci}/${denSci}`;
}


// Canonical numeric rendering: every number is a reduced
// numerator/denominator, integers included (`3` -> `3/1`).
export function formatFraction(frac: Fraction): string {
    return `${frac.numerator}/${frac.denominator}`;
}


export function compareValue(actual: Value, expected: Value): boolean {
    if (actual.type !== expected.type) {
        return false;
    }

    switch (actual.type) {
        case 'number': {
            // Guard against a malformed number node carrying a null/non-object
            // `value`: dereferencing `.numerator` on it would throw a TypeError
            // instead of reporting inequality.
            const actualFrac = actual.value as Fraction | null;
            const expectedFrac = expected.value as Fraction | null;
            if (!actualFrac || typeof actualFrac !== 'object' ||
                !expectedFrac || typeof expectedFrac !== 'object') {
                return actualFrac === expectedFrac;
            }
            return actualFrac.numerator === expectedFrac.numerator &&
                   actualFrac.denominator === expectedFrac.denominator;
        }
        case 'vector':
            if (!Array.isArray(actual.value) || !Array.isArray(expected.value)) {
                return false;
            }
            return compareStack(actual.value, expected.value);
        case 'string':
        case 'boolean':
            return JSON.stringify(actual.value) === JSON.stringify(expected.value);
        case 'nil':
            return true;
        default:
            return JSON.stringify(actual.value) === JSON.stringify(expected.value);
    }
}


export function compareStack(actual: Value[], expected: Value[]): boolean {
    if (actual.length !== expected.length) {
        return false;
    }

    for (let i = 0; i < actual.length; i++) {
        const a = actual[i];
        const e = expected[i];
        if (!a || !e || !compareValue(a, e)) {
            return false;
        }
    }

    return true;
}
