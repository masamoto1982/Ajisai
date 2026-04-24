// AQ-VER-004-B / AQ-VER-004-C: value-formatter MC/DC for QL-A boolean
// decisions in the result-comparison and stack-equality helpers.
//
// Trace: docs/quality/TRACEABILITY_MATRIX.md, requirement AQ-REQ-004.

import { describe, expect, test } from 'vitest';
import {
    compareStack,
    compareValue,
    formatFractionScientific,
} from './value-formatter';
import type { Fraction, Value } from '../wasm-interpreter-types';

function frac(numerator: string | number, denominator: string | number): Fraction {
    return {
        numerator: String(numerator),
        denominator: String(denominator),
    };
}

function num(numerator: string | number, denominator: string | number = 1): Value {
    return { type: 'number', value: frac(numerator, denominator) };
}

function str(s: string): Value {
    return { type: 'string', value: s };
}

function bool(b: boolean): Value {
    return { type: 'boolean', value: b };
}

function vec(items: Value[]): Value {
    return { type: 'vector', value: items };
}

// ---------------------------------------------------------------------------
// AQ-VER-004-B
// DUT: js/gui/value-formatter.ts:130-133 in `compareValue` (number arm)
//
//     return actualFrac.numerator === expectedFrac.numerator &&
//            actualFrac.denominator === expectedFrac.denominator;
//
// Conditions:
//   A = (actualFrac.numerator === expectedFrac.numerator)
//   B = (actualFrac.denominator === expectedFrac.denominator)
//
// MC/DC for A && B:
//   row 1: (A=T, B=T) -> true   (both fields match)
//   row 2: (A=F, B=T) -> false  (numerator differs)
//   row 3: (A=T, B=F) -> false  (denominator differs)
//   Pair (1, 2) with B held T: A flips T->F -> outcome flips.
//   Pair (1, 3) with A held T: B flips T->F -> outcome flips.
// ---------------------------------------------------------------------------
describe('AQ-VER-004-B compareValue number-arm equality conjunction', () => {
    test('row 1 (A=T, B=T) -> true: identical numerator and denominator', () => {
        expect(compareValue(num(2, 3), num(2, 3))).toBe(true);
    });

    test('row 2 (A=F, B=T) -> false: numerator differs', () => {
        // Pair (row 1, row 2) flips A with B held T.
        expect(compareValue(num(5, 3), num(2, 3))).toBe(false);
    });

    test('row 3 (A=T, B=F) -> false: denominator differs', () => {
        // Pair (row 1, row 3) flips B with A held T.
        expect(compareValue(num(2, 3), num(2, 7))).toBe(false);
    });

    test('cross-type guard short-circuits before number arm is reached', () => {
        // The decision at value-formatter.ts:124 (`actual.type !== expected.type`)
        // returns false before the number-arm equality runs. Documented here
        // to make the precondition for the MC/DC table explicit.
        expect(compareValue(num(2, 3), str('2/3'))).toBe(false);
    });
});

// ---------------------------------------------------------------------------
// AQ-VER-004-B (vector arm)
// DUT: js/gui/value-formatter.ts:136-139 in `compareValue`
//
//     if (!Array.isArray(actual.value) || !Array.isArray(expected.value)) {
//         return false;
//     }
//     return compareStack(actual.value, expected.value);
//
// Conditions for the early-return guard:
//   A = !Array.isArray(actual.value)
//   B = !Array.isArray(expected.value)
//
// The `vector` Value variant is typed as `value: Value[]`, but the helper is
// reachable via `JSON.stringify`-roundtripped inputs where one side may be a
// non-array (e.g., scalar smuggled in from a fixture). The guard exists to
// keep the recursive comparator total over malformed inputs.
//
// MC/DC for A || B:
//   row 1: (A=T, B=*)  -> false  (actual.value is not array)
//   row 2: (A=F, B=T)  -> false  (expected.value is not array)
//   row 3: (A=F, B=F)  -> recurse into compareStack -> true/false based on contents
//
// Pair (row 1, row 3) with B held F: A flips T->F -> guard skipped, recurse.
// Pair (row 2, row 3) with A held F: B flips T->F -> guard skipped, recurse.
// ---------------------------------------------------------------------------
describe('AQ-VER-004-B compareValue vector-arm array guard', () => {
    test('row 1 (A=T, B=F): actual.value non-array -> false', () => {
        const actual = { type: 'vector', value: 'not-an-array' as unknown as Value[] };
        const expected = vec([num(1)]);
        expect(compareValue(actual as Value, expected)).toBe(false);
    });

    test('row 2 (A=F, B=T): expected.value non-array -> false', () => {
        const actual = vec([num(1)]);
        const expected = { type: 'vector', value: 'not-an-array' as unknown as Value[] };
        expect(compareValue(actual, expected as Value)).toBe(false);
    });

    test('row 3 (A=F, B=F): both arrays -> recurse and compare contents', () => {
        // Pairs (1,3) and (2,3) prove A and B independent.
        expect(compareValue(vec([num(1), num(2)]), vec([num(1), num(2)]))).toBe(true);
        expect(compareValue(vec([num(1), num(2)]), vec([num(1), num(3)]))).toBe(false);
    });
});

// ---------------------------------------------------------------------------
// AQ-VER-004-C
// DUT: js/gui/value-formatter.ts:159 in `compareStack` (loop guard)
//
//     if (!a || !e || !compareValue(a, e)) { return false; }
//
// Three-condition disjunction over the per-index loop body. `a` and `e` are
// `actual[i]` and `expected[i]` respectively; `noUncheckedIndexedAccess` in
// tsconfig.json forces them through an `undefined` widening which is why the
// truthiness guards exist.
//
// Conditions:
//   X = !a                       (actual[i] is undefined / falsy)
//   Y = !e                       (expected[i] is undefined / falsy)
//   Z = !compareValue(a, e)      (recursive comparison disagrees)
//
// MC/DC for X || Y || Z:
//   row 1: (X=F, Y=F, Z=F) -> guard skipped, loop continues -> true at end
//   row 2: (X=T, Y=*, Z=*) -> short-circuit, return false
//   row 3: (X=F, Y=T, Z=*) -> short-circuit, return false
//   row 4: (X=F, Y=F, Z=T) -> third disjunct fires, return false
//
// Pair (1, 2) with Y, Z held F: X flips F->T -> outcome flips.
// Pair (1, 3) with X held F, Z held F: Y flips F->T -> outcome flips.
// Pair (1, 4) with X, Y held F: Z flips F->T -> outcome flips.
//
// Reaching X=T or Y=T from typed entry points requires either an
// out-of-bounds access (impossible inside the controlled loop) or a sparse
// array. The tests below construct a sparse expected array to exercise
// Y=T deterministically, mirroring how the type system's
// `noUncheckedIndexedAccess` treats `arr[i]` as `T | undefined`.
// ---------------------------------------------------------------------------
describe('AQ-VER-004-C compareStack per-index disjunction', () => {
    test('length mismatch is rejected before the loop runs', () => {
        // value-formatter.ts:152-153 short-circuits on length disagreement,
        // so the loop guard is only reached for equal-length arrays. This
        // is the implicit precondition for the X/Y/Z table below.
        expect(compareStack([num(1)], [num(1), num(2)])).toBe(false);
    });

    test('row 1 (X=F, Y=F, Z=F) -> all elements truthy and equal', () => {
        expect(compareStack([num(1), bool(true)], [num(1), bool(true)])).toBe(true);
    });

    test('row 2 (X=T, Y=F, Z=*) -> actual[i] falsy short-circuits', () => {
        // Sparse `actual` produces `actual[0] === undefined`. Pair (row 1,
        // row 2) flips X with Y, Z held F.
        const actual = new Array<Value>(1) as Value[]; // length 1 but [0] is undefined
        const expected = [num(1)];
        expect(compareStack(actual, expected)).toBe(false);
    });

    test('row 3 (X=F, Y=T, Z=*) -> expected[i] falsy short-circuits', () => {
        // Pair (row 1, row 3) flips Y with X, Z held F.
        const actual = [num(1)];
        const expected = new Array<Value>(1) as Value[];
        expect(compareStack(actual, expected)).toBe(false);
    });

    test('row 4 (X=F, Y=F, Z=T) -> compareValue disagreement triggers third disjunct', () => {
        // Pair (row 1, row 4) flips Z with X, Y held F.
        expect(compareStack([num(1)], [num(2)])).toBe(false);
    });
});

// ---------------------------------------------------------------------------
// AQ-VER-004-D
// DUT: js/gui/value-formatter.ts:45 in `formatFractionScientific`
//
//     if (numSci.includes('e') && denSci.includes('e')) { /* combine */ }
//
// Conditions:
//   A = numSci.includes('e')      (numerator was promoted to scientific form)
//   B = denSci.includes('e')      (denominator was promoted to scientific form)
//
// `formatIntegerScientific` only adds the 'e' suffix when the absolute digit
// count is >= SCIENTIFIC_THRESHOLD (= 10). This gives us a clean knob to flip
// each condition independently.
//
// MC/DC for A && B:
//   row 1: (A=T, B=T) -> combine into single mantissa/exponent form
//   row 2: (A=F, B=T) -> short-circuit, return `${numSci}/${denSci}` raw
//   row 3: (A=T, B=F) -> short-circuit, return raw
//
// Pair (1, 2) with B held T: A flips T->F.
// Pair (1, 3) with A held T: B flips T->F.
// ---------------------------------------------------------------------------
describe('AQ-VER-004-D formatFractionScientific scientific-form conjunction', () => {
    // Numerator under threshold: '12345' (5 digits) -> stays raw.
    // Numerator at threshold:   '1234567890' (10 digits) -> 'e9' form.
    // Same for denominator.

    test('row 1 (A=T, B=T) both scientific -> combined mantissa/exponent', () => {
        // Both 10-digit positives. Result must contain a single 'e' (combined).
        const out = formatFractionScientific('1234567890', '1111111111');
        // Expect form like "1.11111e0" (or similar combined). Loose check:
        // exactly one 'e' OR no 'e' (when exponents cancel and mantissa is in [1,10)).
        const eCount = (out.match(/e/g) ?? []).length;
        expect(eCount).toBeLessThanOrEqual(1);
        // The slash separator must NOT appear: that's the row-2/row-3 form.
        expect(out).not.toContain('/');
    });

    test('row 2 (A=F, B=T) numerator non-scientific, denominator scientific -> raw concat', () => {
        // numStr below threshold (5 digits), denStr at threshold (10 digits).
        // Pair (row 1, row 2) flips A with B held T -> output flips form.
        const out = formatFractionScientific('12345', '1234567890');
        expect(out).toContain('/');
    });

    test('row 3 (A=T, B=F) numerator scientific, denominator non-scientific -> raw concat', () => {
        // Pair (row 1, row 3) flips B with A held T.
        const out = formatFractionScientific('1234567890', '12345');
        expect(out).toContain('/');
    });

    test('denomStr === "1" short-circuits before the scientific conjunction', () => {
        // value-formatter.ts:38-40 returns formatIntegerScientific(numerStr)
        // before the AND is evaluated. Documented as the precondition that
        // bypasses the MC/DC table above.
        expect(formatFractionScientific('1234567890', '1')).toContain('e');
        expect(formatFractionScientific('5', '1')).toBe('5');
    });
});
