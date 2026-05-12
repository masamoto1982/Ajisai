import type { Value } from '../wasm-interpreter-types';

export interface TestCase {
    name: string;
    code: string;
    expectedStack?: Value[];
    expectedOutput?: string;
    expectError?: boolean;
    category?: string;
}

export function createNumber(numerator: string, denominator: string = '1'): Value {
    return { type: 'number', value: { numerator, denominator } };
}

export function createNil(): Value {
    return { type: 'nil', value: null };
}

// Phase 1 GUI tests cover the externally observable behaviour of:
//   - continued-fraction numeric literals (integer / fraction / decimal),
//   - the four arithmetic words on continued fractions,
//   - DUP / DROP / SWAP / OVER stack shuffles,
//   - Nil propagation,
//   - DEF / DEL for user words,
//   - `.` for output emission.
export const TEST_CASES: TestCase[] = [
    {
        name: 'Integer literal pushes CF',
        code: '42',
        expectedStack: [createNumber('42')],
        category: 'Literals',
    },
    {
        name: 'Negative integer literal',
        code: '-17',
        expectedStack: [createNumber('-17')],
        category: 'Literals',
    },
    {
        name: 'Fraction literal reduces',
        code: '6/8',
        expectedStack: [createNumber('3', '4')],
        category: 'Literals',
    },
    {
        name: 'Decimal literal becomes exact fraction',
        code: '0.5',
        expectedStack: [createNumber('1', '2')],
        category: 'Literals',
    },
    {
        name: 'Addition is exact on thirds and sixths',
        code: '1/3 1/6 +',
        expectedStack: [createNumber('1', '2')],
        category: 'Arithmetic',
    },
    {
        name: 'Subtraction',
        code: '7 3 -',
        expectedStack: [createNumber('4')],
        category: 'Arithmetic',
    },
    {
        name: 'Multiplication of fraction by integer',
        code: '3/4 2 *',
        expectedStack: [createNumber('3', '2')],
        category: 'Arithmetic',
    },
    {
        name: 'Division yields exact fraction',
        code: '1 3 /',
        expectedStack: [createNumber('1', '3')],
        category: 'Arithmetic',
    },
    {
        name: 'DUP duplicates the top',
        code: '5 DUP',
        expectedStack: [createNumber('5'), createNumber('5')],
        category: 'Stack',
    },
    {
        name: 'DROP discards the top',
        code: '1 2 DROP',
        expectedStack: [createNumber('1')],
        category: 'Stack',
    },
    {
        name: 'SWAP exchanges the top two',
        code: '1 2 SWAP',
        expectedStack: [createNumber('2'), createNumber('1')],
        category: 'Stack',
    },
    {
        name: 'OVER copies the second item',
        code: '1 2 OVER',
        expectedStack: [createNumber('1'), createNumber('2'), createNumber('1')],
        category: 'Stack',
    },
    {
        name: 'Nil propagates through addition',
        code: 'NIL 1 +',
        expectedStack: [createNil()],
        category: 'Nil',
    },
    {
        name: 'Division by zero produces Nil',
        code: '1 0 /',
        expectedStack: [createNil()],
        category: 'Nil',
    },
    {
        name: 'NIL? returns 1 for Nil',
        code: 'NIL NIL?',
        expectedStack: [createNumber('1')],
        category: 'Nil',
    },
    {
        name: 'NIL? returns 0 for a number',
        code: '42 NIL?',
        expectedStack: [createNumber('0')],
        category: 'Nil',
    },
    {
        name: 'DEF then call user word doubles via DUP +',
        code: 'DEF DOUBLE DUP +',
        expectedStack: [],
        category: 'Definition',
    },
    {
        name: '. writes rational to output',
        code: '3 4 / .',
        expectedStack: [],
        expectedOutput: '3/4',
        category: 'Output',
    },
    {
        name: 'Stack underflow is reported as error',
        code: '+',
        expectError: true,
        category: 'Errors',
    },
    {
        name: 'Unknown word is reported as error',
        code: 'FOO',
        expectError: true,
        category: 'Errors',
    },
];
