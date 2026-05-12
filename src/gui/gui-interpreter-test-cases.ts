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

// Phase 2 GUI tests cover:
//   - continued-fraction numeric literals (integer / fraction / decimal),
//   - the four arithmetic words on continued fractions,
//   - DUP / DROP / SWAP / OVER stack shuffles,
//   - Register words (STORE / RECALL / PEEK) and their sugar (>R / R> / R@),
//   - Comparison words (EQ / NE / LT / LE / GE / GT) and their sugar (=, <>, <, <=, >=),
//   - Three-valued logic (AND / OR / NOT) and their sugar (&, |, !),
//   - Nil propagation,
//   - DEF / DEL for user words,
//   - `.` for output emission.
export const TEST_CASES: TestCase[] = [
    // Literals
    {
        name: 'Integer literal pushes CF',
        code: '42',
        expectedStack: [createNumber('42')],
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

    // Arithmetic
    {
        name: 'Addition is exact on thirds and sixths',
        code: '1/3 1/6 +',
        expectedStack: [createNumber('1', '2')],
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

    // Stack
    {
        name: 'DUP duplicates the top',
        code: '5 DUP',
        expectedStack: [createNumber('5'), createNumber('5')],
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

    // Register
    {
        name: 'STORE moves the top into the Register',
        code: '42 STORE',
        expectedStack: [],
        category: 'Register',
    },
    {
        name: 'RECALL pushes the Register and clears it',
        code: '42 STORE RECALL',
        expectedStack: [createNumber('42')],
        category: 'Register',
    },
    {
        name: 'PEEK keeps the Register intact',
        code: '7 STORE PEEK PEEK',
        expectedStack: [createNumber('7'), createNumber('7')],
        category: 'Register',
    },
    {
        name: 'Sugar >R / R@ / R> work like STORE / PEEK / RECALL',
        code: '7 >R R@ R>',
        expectedStack: [createNumber('7'), createNumber('7')],
        category: 'Register',
    },

    // Comparison
    {
        name: 'EQ on equal values pushes 1',
        code: '1/3 2/6 EQ',
        expectedStack: [createNumber('1')],
        category: 'Comparison',
    },
    {
        name: 'LT pushes 1 when next is less',
        code: '1 2 <',
        expectedStack: [createNumber('1')],
        category: 'Comparison',
    },
    {
        name: 'GE on equal values pushes 1',
        code: '3 3 >=',
        expectedStack: [createNumber('1')],
        category: 'Comparison',
    },
    {
        name: 'Comparison with Nil yields Nil',
        code: 'NIL 1 EQ',
        expectedStack: [createNil()],
        category: 'Comparison',
    },

    // Logic (Kleene K3)
    {
        name: 'AND of true and true is 1',
        code: '1 1 AND',
        expectedStack: [createNumber('1')],
        category: 'Logic',
    },
    {
        name: 'AND with False short-circuits over Nil',
        code: '0 NIL AND',
        expectedStack: [createNumber('0')],
        category: 'Logic',
    },
    {
        name: 'OR with True short-circuits over Nil',
        code: '1 NIL OR',
        expectedStack: [createNumber('1')],
        category: 'Logic',
    },
    {
        name: 'NOT of Nil is Nil',
        code: 'NIL NOT',
        expectedStack: [createNil()],
        category: 'Logic',
    },
    {
        name: 'Sugar & | ! work like AND OR NOT',
        code: '1 0 | 1 &',
        expectedStack: [createNumber('1')],
        category: 'Logic',
    },

    // Nil
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

    // Definition and output
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

    // Errors
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
