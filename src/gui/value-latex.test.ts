// Math-view LaTeX derivation: the alternate KaTeX stack rendering must be
// generated from the structured protocol form and refuse (return null) any
// value without a faithful flat math reading, so the canonical text
// rendering remains the fallback.

import { describe, expect, test } from 'vitest';
import { fractionToLatex, valueToLatex } from './value-latex';
import type { Fraction, Value } from '../wasm-interpreter-types';

function frac(numerator: string | number, denominator: string | number = 1): Fraction {
    return {
        numerator: String(numerator),
        denominator: String(denominator),
    };
}

function num(numerator: string | number, denominator: string | number = 1): Value {
    return { type: 'number', value: frac(numerator, denominator) };
}

function vec(...elements: Value[]): Value {
    return { type: 'vector', value: elements };
}

function tensor(shape: number[], data: unknown[], displayHint?: string): Value {
    return { type: 'tensor', value: { shape, data, displayHint } };
}

describe('fractionToLatex', () => {
    test('integer collapses the denominator', () => {
        expect(fractionToLatex(frac(3))).toBe('3');
    });

    test('proper fraction renders as \\frac', () => {
        expect(fractionToLatex(frac(3, 4))).toBe('\\frac{3}{4}');
    });

    test('negative sign stays outside the bar', () => {
        expect(fractionToLatex(frac(-3, 4))).toBe('-\\frac{3}{4}');
    });

    test('nine-digit components stay exact', () => {
        expect(fractionToLatex(frac('999999999', '7'))).toBe('\\frac{999999999}{7}');
    });
});

describe('fractionToLatex: huge components switch to scientific notation', () => {
    test('huge integer rounds to a six-digit mantissa with \\approx', () => {
        expect(fractionToLatex(frac('12345678901'))).toBe('\\approx 1.23457 \\times 10^{10}');
    });

    test('exact power of ten needs no mantissa and no \\approx', () => {
        expect(fractionToLatex(frac('1' + '0'.repeat(40)))).toBe('10^{40}');
    });

    test('exact short mantissa keeps no \\approx', () => {
        expect(fractionToLatex(frac('5' + '0'.repeat(12)))).toBe('5 \\times 10^{12}');
    });

    test('huge ratio collapses to one scientific number', () => {
        const digits = '9'.repeat(40);
        expect(fractionToLatex(frac(digits, '7'))).toBe('\\approx 1.42857 \\times 10^{39}');
    });

    test('rounding can carry into the exponent', () => {
        expect(fractionToLatex(frac('999999999999'))).toBe('\\approx 10^{12}');
    });

    test('negative huge value keeps its sign', () => {
        expect(fractionToLatex(frac('-12345678901'))).toBe('\\approx -1.23457 \\times 10^{10}');
    });

    test('tiny ratio gets a negative exponent', () => {
        expect(fractionToLatex(frac('1', '1' + '0'.repeat(12)))).toBe('10^{-12}');
    });

    test('human-scale value with huge components renders as a decimal', () => {
        expect(fractionToLatex(frac('1414213562', '1000000000'))).toBe('\\approx 1.41421');
    });

    test('mid-scale value places the decimal point, not a power of ten', () => {
        expect(fractionToLatex(frac('31415926535', '100000000'))).toBe('\\approx 314.159');
    });

    test('near-zero value uses leading zeros down to 10^-4', () => {
        expect(fractionToLatex(frac('1234567891', '10000000000000'))).toBe('\\approx 0.000123457');
    });

    test('exact human-scale value carries no \\approx', () => {
        expect(fractionToLatex(frac('1500000000', '1000000000'))).toBe('1.5');
    });
});

describe('valueToLatex: scalars', () => {
    test('number renders as fraction', () => {
        expect(valueToLatex(num(1, 2))).toBe('\\frac{1}{2}');
    });

    test('approximate sqrt(2) renders as a decimal with a single \\approx', () => {
        const item = { ...num(1414213562, 1000000000), semantics: { approximate: true } } as Value;
        expect(valueToLatex(item)).toBe('\\approx 1.41421');
    });

    test('malformed numerator is refused (no TeX injection)', () => {
        const item: Value = { type: 'number', value: { numerator: '\\dangerous', denominator: '1' } };
        expect(valueToLatex(item)).toBeNull();
    });

    test('non-math types are refused', () => {
        expect(valueToLatex({ type: 'string', value: 'hello' })).toBeNull();
        expect(valueToLatex({ type: 'nil', value: null })).toBeNull();
        expect(valueToLatex({ type: 'boolean', value: true })).toBeNull();
    });
});

describe('valueToLatex: vectors', () => {
    test('numeric vector renders as a one-row matrix', () => {
        expect(valueToLatex(vec(num(1), num(2), num(3)))).toBe(
            '\\begin{bmatrix} 1 & 2 & 3 \\end{bmatrix}'
        );
    });

    test('rectangular nested vector renders as a rank-2 matrix', () => {
        expect(valueToLatex(vec(vec(num(1), num(2)), vec(num(3), num(4))))).toBe(
            '\\begin{bmatrix} 1 & 2 \\\\ 3 & 4 \\end{bmatrix}'
        );
    });

    test('ragged nested vector is refused', () => {
        expect(valueToLatex(vec(vec(num(1), num(2)), vec(num(3))))).toBeNull();
    });

    test('mixed-type vector is refused', () => {
        expect(valueToLatex(vec(num(1), { type: 'string', value: 'x' }))).toBeNull();
    });

    test('empty vector is refused (bracket text is the surface)', () => {
        expect(valueToLatex(vec())).toBeNull();
    });

    test('oversized vector falls back to text', () => {
        const elements = Array.from({ length: 65 }, (_, i) => num(i));
        expect(valueToLatex(vec(...elements))).toBeNull();
    });
});

describe('valueToLatex: tensors', () => {
    test('rank-1 tensor renders as a one-row matrix', () => {
        expect(valueToLatex(tensor([2], [frac(1, 2), frac(3)]))).toBe(
            '\\begin{bmatrix} \\frac{1}{2} & 3 \\end{bmatrix}'
        );
    });

    test('rank-2 tensor renders rows split by shape', () => {
        expect(valueToLatex(tensor([2, 2], [frac(1), frac(2), frac(3), frac(4)]))).toBe(
            '\\begin{bmatrix} 1 & 2 \\\\ 3 & 4 \\end{bmatrix}'
        );
    });

    test('invalid lane renders as NIL occupancy', () => {
        expect(valueToLatex(tensor([2], [frac(1), null]))).toBe(
            '\\begin{bmatrix} 1 & \\mathrm{NIL} \\end{bmatrix}'
        );
    });

    test('rank-0 tensor renders its single lane', () => {
        expect(valueToLatex(tensor([], [frac(5, 6)]))).toBe('\\frac{5}{6}');
    });

    test('rank-3 tensor is refused', () => {
        expect(valueToLatex(tensor([1, 1, 2], [frac(1), frac(2)]))).toBeNull();
    });

    test('text-hinted byte tensor is refused (it is a string)', () => {
        expect(valueToLatex(tensor([2], [frac(72), frac(105)], 'text'))).toBeNull();
    });

    test('shape/data mismatch for rank-2 is refused', () => {
        expect(valueToLatex(tensor([2, 2], [frac(1), frac(2), frac(3)]))).toBeNull();
    });
});

// Adversarial robustness (fuzzing regression): the math view must never throw.
// A number value whose denominator is zero is malformed / NIL occupancy (it
// never arises from a canonical number, but can reach the renderer via restored
// or injected state). `scientificLatex` used to divide by zero on a >=10-digit
// zero denominator, throwing a RangeError out of the live Stack render.
describe('valueToLatex zero-denominator robustness', () => {
    for (const denom of ['0', '-0', '00', '0000000000', '-0000000000']) {
        for (const numer of ['1', '1234567890', '12345678901', '99999999999999999999']) {
            test(`returns null (text fallback) for ${numer}/${denom}`, () => {
                expect(valueToLatex(num(numer, denom))).toBeNull();
            });
        }
    }

    test('fractionToLatex does not throw on a huge zero denominator', () => {
        expect(() => fractionToLatex(frac('12345678901', '0000000000'))).not.toThrow();
        expect(() => fractionToLatex(frac('12345678901', '0'))).not.toThrow();
    });
});
