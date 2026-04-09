

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

export function createVector(elements: Value[]): Value {
    return { type: 'vector', value: elements };
}

export function createString(value: string): Value {
    return { type: 'string', value };
}

export function createBoolean(value: boolean): Value {
    return { type: 'boolean', value };
}

export function createNil(): Value {
    return { type: 'nil', value: null };
}


export const TEST_CASES: TestCase[] = [



    {
        name: "Number - integer",
        code: "[ 42 ]",
        expectedStack: [createVector([createNumber('42')])],
        category: "Basic Types"
    },
    {
        name: "Number - negative",
        code: "[ -17 ]",
        expectedStack: [createVector([createNumber('-17')])],
        category: "Basic Types"
    },
    {
        name: "Number - fraction",
        code: "[ 3/4 ]",
        expectedStack: [createVector([createNumber('3', '4')])],
        category: "Basic Types"
    },
    {
        name: "Number - decimal converts to fraction",
        code: "[ 0.5 ]",
        expectedStack: [createVector([createNumber('1', '2')])],
        category: "Basic Types"
    },
    {
        name: "String - simple",
        code: "[ 'hello' ]",
        expectedStack: [createVector([createString('hello')])],
        category: "Basic Types"
    },
    {
        name: "String - with spaces",
        code: "[ 'hello world' ]",
        expectedStack: [createVector([createString('hello world')])],
        category: "Basic Types"
    },
    {
        name: "Boolean - TRUE",
        code: "[ TRUE ]",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Basic Types"
    },
    {
        name: "Boolean - FALSE",
        code: "[ FALSE ]",
        expectedStack: [createVector([createBoolean(false)])],
        category: "Basic Types"
    },
    {
        name: "NIL",
        code: "[ NIL ]",
        expectedStack: [createVector([createNil()])],
        category: "Basic Types"
    },




    {
        name: "Addition - integers",
        code: "[ 2 ] [ 3 ] +",
        expectedStack: [createVector([createNumber('5')])],
        category: "Arithmetic"
    },
    {
        name: "Addition - fractions",
        code: "[ 1/2 ] [ 1/3 ] +",
        expectedStack: [createVector([createNumber('5', '6')])],
        category: "Arithmetic"
    },
    {
        name: "Subtraction",
        code: "[ 10 ] [ 3 ] -",
        expectedStack: [createVector([createNumber('7')])],
        category: "Arithmetic"
    },
    {
        name: "Multiplication",
        code: "[ 4 ] [ 5 ] *",
        expectedStack: [createVector([createNumber('20')])],
        category: "Arithmetic"
    },
    {
        name: "Division",
        code: "[ 10 ] [ 4 ] /",
        expectedStack: [createVector([createNumber('5', '2')])],
        category: "Arithmetic"
    },
    {
        name: "Division by zero - error",
        code: "[ 1 ] [ 0 ] /",
        expectError: true,
        category: "Arithmetic"
    },
    {
        name: "Modulo",
        code: "[ 7 ] [ 3 ] MOD",
        expectedStack: [createVector([createNumber('1')])],
        category: "Arithmetic"
    },
    {
        name: "Floor",
        code: "[ 7/3 ] FLOOR",
        expectedStack: [createVector([createNumber('2')])],
        category: "Arithmetic"
    },
    {
        name: "Ceil",
        code: "[ 7/3 ] CEIL",
        expectedStack: [createVector([createNumber('3')])],
        category: "Arithmetic"
    },
    {
        name: "Round",
        code: "[ 5/2 ] ROUND",
        expectedStack: [createVector([createNumber('3')])],
        category: "Arithmetic"
    },





    {
        name: "Less than - true",
        code: "[ 3 ] [ 5 ] <",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Comparison"
    },
    {
        name: "Less than - false",
        code: "[ 5 ] [ 3 ] <",
        expectedStack: [createVector([createBoolean(false)])],
        category: "Comparison"
    },
    {
        name: "Greater than (via <= NOT)",
        code: "[ 5 ] [ 3 ] <= NOT",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Comparison"
    },
    {
        name: "Less than or equal",
        code: "[ 3 ] [ 3 ] <=",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Comparison"
    },
    {
        name: "Greater than or equal (via < NOT)",
        code: "[ 3 ] [ 3 ] < NOT",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Comparison"
    },
    {
        name: "Equal - numbers",
        code: "[ 5 ] [ 5 ] =",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Comparison"
    },
    {
        name: "Equal - fraction auto-reduction",
        code: "[ 1/2 ] [ 2/4 ] =",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Comparison"
    },




    {
        name: "AND - true && true",
        code: "[ TRUE ] [ TRUE ] AND",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Logic"
    },
    {
        name: "AND - true && false",
        code: "[ TRUE ] [ FALSE ] AND",
        expectedStack: [createVector([createBoolean(false)])],
        category: "Logic"
    },
    {
        name: "OR - false || true",
        code: "[ FALSE ] [ TRUE ] OR",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Logic"
    },
    {
        name: "NOT - true",
        code: "[ TRUE ] NOT",
        expectedStack: [createVector([createBoolean(false)])],
        category: "Logic"
    },
    {
        name: "NOT - false",
        code: "[ FALSE ] NOT",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Logic"
    },




    {
        name: "COND - basic branch",
        code: "[ -1 ] { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND",
        expectedStack: [createString('negative')],
        category: "Conditional"
    },
    {
        name: "COND - else branch",
        code: "[ 7 ] { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND",
        expectedStack: [createString('positive')],
        category: "Conditional"
    },
    {
        name: "COND - exhausted error",
        code: "[ 7 ] { [ 0 ] < } { 'negative' } COND",
        expectError: true,
        category: "Conditional"
    },




    {
        name: "LENGTH",
        code: "[ 1 2 3 4 5 ] LENGTH",
        expectedStack: [
            createVector([createNumber('1'), createNumber('2'), createNumber('3'), createNumber('4'), createNumber('5')]),
            createNumber('5')
        ],
        category: "Vector Operations"
    },
    {
        name: "GET - first element",
        code: "[ 10 20 30 ] [ 0 ] GET",
        expectedStack: [
            createVector([createNumber('10'), createNumber('20'), createNumber('30')]),
            createNumber('10')
        ],
        category: "Vector Operations"
    },
    {
        name: "GET - negative index",
        code: "[ 10 20 30 ] [ -1 ] GET",
        expectedStack: [
            createVector([createNumber('10'), createNumber('20'), createNumber('30')]),
            createNumber('30')
        ],
        category: "Vector Operations"
    },
    {
        name: "TAKE - positive",
        code: "[ 1 2 3 4 5 ] [ 3 ] TAKE",
        expectedStack: [createVector([createNumber('1'), createNumber('2'), createNumber('3')])],
        category: "Vector Operations"
    },
    {
        name: "TAKE - negative",
        code: "[ 1 2 3 4 5 ] [ -2 ] TAKE",
        expectedStack: [createVector([createNumber('4'), createNumber('5')])],
        category: "Vector Operations"
    },
    {
        name: "REVERSE",
        code: "[ 1 2 3 ] REVERSE",
        expectedStack: [createVector([createNumber('3'), createNumber('2'), createNumber('1')])],
        category: "Vector Operations"
    },
    {
        name: "CONCAT",
        code: "[ 1 2 ] [ 3 4 ] CONCAT",
        expectedStack: [createVector([createNumber('1'), createNumber('2'), createNumber('3'), createNumber('4')])],
        category: "Vector Operations"
    },
    {

        name: "INSERT",
        code: "[ 1 3 ] [ 1 2 ] INSERT",
        expectedStack: [createVector([createNumber('1'), createNumber('2'), createNumber('3')])],
        category: "Vector Operations"
    },
    {

        name: "REPLACE",
        code: "[ 1 2 3 ] [ 1 9 ] REPLACE",
        expectedStack: [createVector([createNumber('1'), createNumber('9'), createNumber('3')])],
        category: "Vector Operations"
    },
    {
        name: "REMOVE",
        code: "[ 1 2 3 ] [ 1 ] REMOVE",
        expectedStack: [createVector([createNumber('1'), createNumber('3')])],
        category: "Vector Operations"
    },




    {

        name: "SHAPE - 1D",
        code: "[ 1 2 3 ] SHAPE",
        expectedStack: [
            createVector([createNumber('3')])
        ],
        category: "Tensor Operations"
    },
    {
        name: "SHAPE - 2D",
        code: "[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE",
        expectedStack: [
            createVector([createNumber('2'), createNumber('3')])
        ],
        category: "Tensor Operations"
    },
    {

        name: "RANK - 1D",
        code: "[ 1 2 3 ] RANK",
        expectedStack: [
            createNumber('1')
        ],
        category: "Tensor Operations"
    },
    {
        name: "RANK - 2D",
        code: "[ [ 1 2 ] [ 3 4 ] ] RANK",
        expectedStack: [
            createNumber('2')
        ],
        category: "Tensor Operations"
    },
    {
        name: "TRANSPOSE",
        code: "[ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE",
        expectedStack: [
            createVector([
                createVector([createNumber('1'), createNumber('4')]),
                createVector([createNumber('2'), createNumber('5')]),
                createVector([createNumber('3'), createNumber('6')])
            ])
        ],
        category: "Tensor Operations"
    },
    {
        name: "RESHAPE",
        code: "[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE",
        expectedStack: [
            createVector([
                createVector([createNumber('1'), createNumber('2'), createNumber('3')]),
                createVector([createNumber('4'), createNumber('5'), createNumber('6')])
            ])
        ],
        category: "Tensor Operations"
    },




    {
        name: "Broadcast - scalar + vector",
        code: "[ 10 ] [ 1 2 3 ] +",
        expectedStack: [createVector([createNumber('11'), createNumber('12'), createNumber('13')])],
        category: "Broadcasting"
    },
    {
        name: "Broadcast - vector * scalar",
        code: "[ 1 2 3 ] [ 2 ] *",
        expectedStack: [createVector([createNumber('2'), createNumber('4'), createNumber('6')])],
        category: "Broadcasting"
    },
    {
        name: "Broadcast - vector + vector (same length)",
        code: "[ 1 2 3 ] [ 10 20 30 ] +",
        expectedStack: [createVector([createNumber('11'), createNumber('22'), createNumber('33')])],
        category: "Broadcasting"
    },





    {
        name: "MAP - double",
        code: "{ [ 2 ] * } 'DBL' DEF\n[ 1 2 3 ] 'DBL' MAP",
        expectedStack: [createVector([createNumber('2'), createNumber('4'), createNumber('6')])],
        category: "Higher-Order Functions"
    },
    {

        name: "FILTER - positive",
        code: "{ [ 0 ] <= NOT } 'POS' DEF\n[ -2 -1 0 1 2 ] 'POS' FILTER",
        expectedStack: [createVector([createNumber('1'), createNumber('2')])],
        category: "Higher-Order Functions"
    },
    {
        name: "FOLD - sum",
        code: "[ 1 2 3 4 ] [ 0 ] '+' FOLD",
        expectedStack: [createVector([createNumber('10')])],
        category: "Higher-Order Functions"
    },
    {
        name: "UNFOLD - basic",
        code: "[ 1 ] { { [ 1 ] = } { [ 1 2 ] } { [ 2 ] = } { [ 2 3 ] } { [ 3 ] = } { [ 3 NIL ] } { IDLE } { NIL } COND } UNFOLD",
        expectedStack: [createVector([createNumber('1'), createNumber('2'), createNumber('3'), createNumber('4')])],
        category: "Higher-Order Functions"
    },
    {
        name: "ANY - basic",
        code: "[ 1 3 5 8 ] { [ 2 ] MOD [ 0 ] = } ANY",
        expectedStack: [createBoolean(true)],
        category: "Higher-Order Functions"
    },
    {
        name: "ALL - basic",
        code: "[ 2 4 6 8 ] { [ 2 ] MOD [ 0 ] = } ALL",
        expectedStack: [createBoolean(true)],
        category: "Higher-Order Functions"
    },
    {
        name: "COUNT - basic",
        code: "[ 1 2 3 4 5 6 ] { [ 2 ] MOD [ 0 ] = } COUNT",
        expectedStack: [createVector([createNumber('3')])],
        category: "Higher-Order Functions"
    },
    {
        name: "SCAN - prefix sum",
        code: "[ 1 2 3 4 ] [ 0 ] '+' SCAN",
        expectedStack: [createVector([createNumber('1'), createNumber('3'), createNumber('6'), createNumber('10')])],
        category: "Higher-Order Functions"
    },





    {
        name: "STR - number to string",
        code: "[ 42 ] STR",
        expectedStack: [createString('42')],
        category: "Type Conversion"
    },
    {
        name: "STR - fraction to string",
        code: "[ 3/4 ] STR",
        expectedStack: [createString('3/4')],
        category: "Type Conversion"
    },
    {


        name: "NUM - string to number",
        code: "'42' NUM",
        expectedStack: [createNumber('42')],
        category: "Type Conversion"
    },
    {


        name: "BOOL - 1 to true",
        code: "1 BOOL",
        expectedStack: [createBoolean(true)],
        category: "Type Conversion"
    },
    {
        name: "BOOL - 0 to false",
        code: "0 BOOL",
        expectedStack: [createBoolean(false)],
        category: "Type Conversion"
    },





    {
        name: "CHARS - split string",
        code: "'hello' CHARS",
        expectedStack: [createVector([
            createString('h'),
            createString('e'),
            createString('l'),
            createString('l'),
            createString('o')
        ])],
        category: "String Operations"
    },
    {

        name: "JOIN - join strings",
        code: "[ 'h' 'e' 'l' 'l' 'o' ] JOIN",
        expectedStack: [createString('hello')],
        category: "String Operations"
    },




    {
        name: "Stack mode - LENGTH",
        code: "[ 1 ] [ 2 ] [ 3 ] .. LENGTH",
        expectedStack: [
            createVector([createNumber('1')]),
            createVector([createNumber('2')]),
            createVector([createNumber('3')]),
            createNumber('3')
        ],
        category: "Stack Mode"
    },
    {
        name: "Stack mode - GET",
        code: "[ 'a' ] [ 'b' ] [ 'c' ] [ 1 ] .. GET",
        expectedStack: [
            createVector([createString('a')]),
            createVector([createString('b')]),
            createVector([createString('c')]),
            createVector([createString('b')])
        ],
        category: "Stack Mode"
    },
    {
        name: "Stack mode - REVERSE",
        code: "[ 1 ] [ 2 ] [ 3 ] .. REVERSE",
        expectedStack: [
            createVector([createNumber('3')]),
            createVector([createNumber('2')]),
            createVector([createNumber('1')])
        ],
        category: "Stack Mode"
    },





    {
        name: "DEF and call",
        code: "{ [ 2 ] * } 'DOUBLE' DEF\n[ 5 ] DOUBLE",
        expectedStack: [createVector([createNumber('10')])],
        category: "User Words"
    },
    {
        name: "DEL - delete user word",
        code: "{ [ 2 ] * } 'TEMP' DEF\n'TEMP' DEL\nTEMP",
        expectError: true,
        category: "User Words"
    },




    {

        name: "FILL",
        code: "[ 3 7 ] FILL",
        expectedStack: [createVector([
            createNumber('7'),
            createNumber('7'),
            createNumber('7')
        ])],
        category: "Tensor Generation"
    },




    {

        name: "Nil Coalescing - NIL case",
        code: "NIL => [ 0 ]",
        expectedStack: [createVector([createNumber('0')])],
        category: "NIL Safety"
    },
    {
        name: "Nil Coalescing - non-NIL case",
        code: "[ 42 ] => [ 0 ]",
        expectedStack: [createVector([createNumber('42')])],
        category: "NIL Safety"
    },




    {
        name: "Error - stack underflow",
        code: "+",
        expectError: true,
        category: "Error Cases"
    },
    {
        name: "Error - unknown word",
        code: "UNKNOWNWORD",
        expectError: true,
        category: "Error Cases"
    },
    {
        name: "Error - index out of bounds",
        code: "[ 1 2 3 ] [ 10 ] GET",
        expectError: true,
        category: "Error Cases"
    },
    {

        name: "Error - incompatible shapes",
        code: "[ 1 2 3 ] [ 1 2 ] +",
        expectError: true,
        category: "Error Cases"
    },
    {

        name: "Error - empty vector",
        code: "[ ]",
        expectError: true,
        category: "Error Cases"
    },
    {

        name: "Error - no change (sort already sorted)",
        code: "[ 1 2 3 ] SORT",
        expectError: true,
        category: "Error Cases"
    }
];
