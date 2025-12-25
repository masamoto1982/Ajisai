// js/gui/test-cases.ts - テストケース定義

import type { Value } from '../wasm-types';

export interface TestCase {
    name: string;
    code: string;
    expectedStack?: Value[];
    expectedOutput?: string;
    expectError?: boolean;
    category?: string;
}

// ヘルパー関数：値の生成
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

// テストケース定義
export const TEST_CASES: TestCase[] = [
    // ============================================
    // Basic Types - 基本型
    // ============================================
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

    // ============================================
    // Arithmetic - 算術演算
    // ============================================
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

    // ============================================
    // Comparison - 比較演算
    // ============================================
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
        name: "Greater than",
        code: "[ 5 ] [ 3 ] >",
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
        name: "Greater than or equal",
        code: "[ 3 ] [ 3 ] >=",
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
        name: "Equal - strings",
        code: "[ 'abc' ] [ 'abc' ] =",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Comparison"
    },

    // ============================================
    // Logic - 論理演算
    // ============================================
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

    // ============================================
    // Vector Operations - ベクタ操作
    // ============================================
    {
        name: "LENGTH",
        code: "[ 1 2 3 4 5 ] LENGTH",
        expectedStack: [
            createVector([createNumber('1'), createNumber('2'), createNumber('3'), createNumber('4'), createNumber('5')]),
            createVector([createNumber('5')])
        ],
        category: "Vector Operations"
    },
    {
        name: "GET - first element",
        code: "[ 10 20 30 ] [ 0 ] GET",
        expectedStack: [
            createVector([createNumber('10'), createNumber('20'), createNumber('30')]),
            createVector([createNumber('10')])
        ],
        category: "Vector Operations"
    },
    {
        name: "GET - negative index",
        code: "[ 10 20 30 ] [ -1 ] GET",
        expectedStack: [
            createVector([createNumber('10'), createNumber('20'), createNumber('30')]),
            createVector([createNumber('30')])
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
        code: "[ 1 3 ] [ 1 ] [ 2 ] INSERT",
        expectedStack: [createVector([createNumber('1'), createNumber('2'), createNumber('3')])],
        category: "Vector Operations"
    },
    {
        name: "REPLACE",
        code: "[ 1 2 3 ] [ 1 ] [ 9 ] REPLACE",
        expectedStack: [createVector([createNumber('1'), createNumber('9'), createNumber('3')])],
        category: "Vector Operations"
    },
    {
        name: "REMOVE",
        code: "[ 1 2 3 ] [ 1 ] REMOVE",
        expectedStack: [createVector([createNumber('1'), createNumber('3')])],
        category: "Vector Operations"
    },

    // ============================================
    // Tensor Operations - テンソル操作
    // ============================================
    {
        name: "SHAPE - 1D",
        code: "[ 1 2 3 ] SHAPE",
        expectedStack: [
            createVector([createNumber('1'), createNumber('2'), createNumber('3')]),
            createVector([createNumber('3')])
        ],
        category: "Tensor Operations"
    },
    {
        name: "SHAPE - 2D",
        code: "[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE",
        expectedStack: [
            createVector([
                createVector([createNumber('1'), createNumber('2'), createNumber('3')]),
                createVector([createNumber('4'), createNumber('5'), createNumber('6')])
            ]),
            createVector([createNumber('2'), createNumber('3')])
        ],
        category: "Tensor Operations"
    },
    {
        name: "RANK - 1D",
        code: "[ 1 2 3 ] RANK",
        expectedStack: [
            createVector([createNumber('1'), createNumber('2'), createNumber('3')]),
            createVector([createNumber('1')])
        ],
        category: "Tensor Operations"
    },
    {
        name: "RANK - 2D",
        code: "[ [ 1 2 ] [ 3 4 ] ] RANK",
        expectedStack: [
            createVector([
                createVector([createNumber('1'), createNumber('2')]),
                createVector([createNumber('3'), createNumber('4')])
            ]),
            createVector([createNumber('2')])
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

    // ============================================
    // Broadcasting - ブロードキャスト
    // ============================================
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

    // ============================================
    // Higher-Order Functions - 高階関数
    // ============================================
    {
        name: "MAP - double",
        code: "[ ': [ 2 ] *' ] 'DBL' DEF\n[ 1 2 3 ] 'DBL' MAP",
        expectedStack: [createVector([createNumber('2'), createNumber('4'), createNumber('6')])],
        category: "Higher-Order Functions"
    },
    {
        name: "FILTER - positive",
        code: "[ ': [ 0 ] >' ] 'POS' DEF\n[ -2 -1 0 1 2 ] 'POS' FILTER",
        expectedStack: [createVector([createNumber('1'), createNumber('2')])],
        category: "Higher-Order Functions"
    },
    {
        name: "FOLD - sum",
        code: "[ 1 2 3 4 ] [ 0 ] '+' FOLD",
        expectedStack: [createVector([createNumber('10')])],
        category: "Higher-Order Functions"
    },

    // ============================================
    // Type Conversion - 型変換
    // ============================================
    {
        name: "STR - number to string",
        code: "[ 42 ] STR",
        expectedStack: [createVector([createString('42')])],
        category: "Type Conversion"
    },
    {
        name: "STR - fraction to string",
        code: "[ 3/4 ] STR",
        expectedStack: [createVector([createString('3/4')])],
        category: "Type Conversion"
    },
    {
        name: "NUM - string to number",
        code: "[ '42' ] NUM",
        expectedStack: [createVector([createNumber('42')])],
        category: "Type Conversion"
    },
    {
        name: "BOOL - 1 to true",
        code: "[ 1 ] BOOL",
        expectedStack: [createVector([createBoolean(true)])],
        category: "Type Conversion"
    },
    {
        name: "BOOL - 0 to false",
        code: "[ 0 ] BOOL",
        expectedStack: [createVector([createBoolean(false)])],
        category: "Type Conversion"
    },

    // ============================================
    // String Operations - 文字列操作
    // ============================================
    {
        name: "CHARS - split string",
        code: "[ 'hello' ] CHARS",
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
        expectedStack: [createVector([createString('hello')])],
        category: "String Operations"
    },

    // ============================================
    // Stack Mode (..) - スタックモード
    // ============================================
    {
        name: "Stack mode - LENGTH",
        code: "[ 1 ] [ 2 ] [ 3 ] .. LENGTH",
        expectedStack: [
            createVector([createNumber('1')]),
            createVector([createNumber('2')]),
            createVector([createNumber('3')]),
            createVector([createNumber('3')])
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

    // ============================================
    // Custom Word Definition - カスタムワード定義
    // ============================================
    {
        name: "DEF and call",
        code: "[ ': [ 2 ] *' ] 'DOUBLE' DEF\n[ 5 ] DOUBLE",
        expectedStack: [createVector([createNumber('10')])],
        category: "Custom Words"
    },
    {
        name: "DEF with guard clause",
        code: "[ ': [ 0 ] >\n: [ 1 ]\n: [ 0 ]' ] 'SIGN' DEF\n[ 5 ] SIGN",
        expectedStack: [createVector([createNumber('1')])],
        category: "Custom Words"
    },
    {
        name: "DEL - delete custom word",
        code: "[ ': [ 2 ] *' ] 'TEMP' DEF\n'TEMP' DEL\nTEMP",
        expectError: true,
        category: "Custom Words"
    },

    // ============================================
    // Control Flow - 制御フロー
    // ============================================
    {
        name: "TIMES - repeat",
        code: "[ ': [ 1 ] +' ] 'INC' DEF\n[ 0 ] 'INC' [ 5 ] TIMES",
        expectedStack: [createVector([createNumber('5')])],
        category: "Control Flow"
    },

    // ============================================
    // Tensor Generation - テンソル生成
    // ============================================
    {
        name: "FILL",
        code: "[ 3 ] [ 7 ] FILL",
        expectedStack: [createVector([
            createNumber('7'),
            createNumber('7'),
            createNumber('7')
        ])],
        category: "Tensor Generation"
    },

    // ============================================
    // Error Cases - エラーケース
    // ============================================
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
        name: "Error - type mismatch in arithmetic",
        code: "[ 'hello' ] [ 1 ] +",
        expectError: true,
        category: "Error Cases"
    }
];
