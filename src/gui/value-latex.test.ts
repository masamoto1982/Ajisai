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

    test('big integers pass through verbatim', () => {
        const digits = '9'.repeat(40);
        expect(fractionToLatex(frac(digits, '7'))).toBe(`\\frac{${digits}}{7}`);
    });
});

describe('valueToLatex: scalars', () => {
    test('number renders as fraction', () => {
        expect(valueToLatex(num(1, 2))).toBe('\\frac{1}{2}');
    });

    test('approximate marker prefixes \\approx', () => {
        const item = { ...num(1414213562, 1000000000), semantics: { approximate: true } } as Value;
        expect(valueToLatex(item)).toBe('\\approx \\frac{1414213562}{1000000000}');
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
