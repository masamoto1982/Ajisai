// js/gui/test-cases.ts - Test case definitions

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

// Test case definitions
export const TEST_CASES: TestCase[] = [
    // ============================================
    // Basic Types
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
    // Arithmetic
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
    // Comparison
    // 仕様: > と >= は提供しない。< と <= のみ使用する（セクション6.2）
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

    // ============================================
    // Logic
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
    // Vector Operations
    // ============================================
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
        // 仕様: INSERT は [index element] を単一ベクタとして受け取る
        name: "INSERT",
        code: "[ 1 3 ] [ 1 2 ] INSERT",
        expectedStack: [createVector([createNumber('1'), createNumber('2'), createNumber('3')])],
        category: "Vector Operations"
    },
    {
        // 仕様: REPLACE は [index new_element] を単一ベクタとして受け取る
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

    // ============================================
    // Tensor Operations
    // ============================================
    {
        // 仕様: SHAPE はデフォルトで入力を消費し、形状ベクタを返す
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
        // 仕様: RANK はデフォルトで入力を消費し、ランクのスカラーを返す
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

    // ============================================
    // Broadcasting
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
    // Higher-Order Functions
    // 仕様: DEF構文は : code ; 'NAME' DEF（セクション6.3）
    // ============================================
    {
        name: "MAP - double",
        code: ": [ 2 ] * ; 'DBL' DEF\n[ 1 2 3 ] 'DBL' MAP",
        expectedStack: [createVector([createNumber('2'), createNumber('4'), createNumber('6')])],
        category: "Higher-Order Functions"
    },
    {
        // 仕様: > は提供しない。<= NOT で代替する
        name: "FILTER - positive",
        code: ": [ 0 ] <= NOT ; 'POS' DEF\n[ -2 -1 0 1 2 ] 'POS' FILTER",
        expectedStack: [createVector([createNumber('1'), createNumber('2')])],
        category: "Higher-Order Functions"
    },
    {
        name: "FOLD - sum",
        code: "[ 1 2 3 4 ] [ 0 ] : + ; FOLD",
        expectedStack: [createVector([createNumber('10')])],
        category: "Higher-Order Functions"
    },

    // ============================================
    // Type Conversion
    // 仕様: STR は Map型ワードで、入力を消費して文字列を返す（セクション5.1）
    // ============================================
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
        // 仕様: NUM は文字列を数値にパースする（セクション2.2）
        // 入力は文字列（ベクタに包まない）
        name: "NUM - string to number",
        code: "'42' NUM",
        expectedStack: [createNumber('42')],
        category: "Type Conversion"
    },
    {
        // 仕様: BOOL はスカラー数値を真偽値に変換する
        // 入力はスカラー（ベクタに包まない）
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

    // ============================================
    // String Operations
    // 仕様: CHARS は文字列に対して操作する（ベクタに包まない）
    // ============================================
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
        // 仕様: JOIN は文字列ベクタを結合し、単一文字列を返す
        name: "JOIN - join strings",
        code: "[ 'h' 'e' 'l' 'l' 'o' ] JOIN",
        expectedStack: [createString('hello')],
        category: "String Operations"
    },

    // ============================================
    // Stack Mode (..)
    // ============================================
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

    // ============================================
    // Custom Word Definition
    // 仕様: DEF構文は : code ; 'NAME' DEF（セクション6.3）
    // ============================================
    {
        name: "DEF and call",
        code: ": [ 2 ] * ; 'DOUBLE' DEF\n[ 5 ] DOUBLE",
        expectedStack: [createVector([createNumber('10')])],
        category: "Custom Words"
    },
    {
        // 仕様: シェブロン分岐（ガード）を使用する（セクション4.2）
        name: "DEF with guard clause",
        code: ":\n>> [ 3 ] [ 1 ] <\n>> [ 99 ]\n>>> [ 0 ]\n; 'GUARD' DEF\nGUARD",
        expectedStack: [createVector([createNumber('0')])],
        category: "Custom Words"
    },
    {
        name: "DEL - delete custom word",
        code: ": [ 2 ] * ; 'TEMP' DEF\n'TEMP' DEL\nTEMP",
        expectError: true,
        category: "Custom Words"
    },

    // ============================================
    // Control Flow
    // ============================================
    {
        // 仕様: TIMES はコードブロックまたはカスタムワード名をN回繰り返す（セクション5.4）
        name: "TIMES - repeat",
        code: ": [ 1 ] + ; 'INC' DEF\n[ 0 ] 'INC' [ 5 ] TIMES",
        expectedStack: [createVector([createNumber('5')])],
        category: "Control Flow"
    },

    // ============================================
    // Tensor Generation
    // ============================================
    {
        // 仕様: FILL は [ shape... value ] を単一ベクタとして受け取る
        name: "FILL",
        code: "[ 3 7 ] FILL",
        expectedStack: [createVector([
            createNumber('7'),
            createNumber('7'),
            createNumber('7')
        ])],
        category: "Tensor Generation"
    },

    // ============================================
    // NIL Safety (セクション7)
    // ============================================
    {
        // 仕様: NIL Coalescing演算子 => （セクション4.4）
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

    // ============================================
    // Error Cases
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
        // 仕様: 互換性のない形状ではブロードキャストできずエラー（セクション2.5）
        name: "Error - incompatible shapes",
        code: "[ 1 2 3 ] [ 1 2 ] +",
        expectError: true,
        category: "Error Cases"
    },
    {
        // 仕様: 空ブラケットは禁止（セクション2.7）
        name: "Error - empty vector",
        code: "[ ]",
        expectError: true,
        category: "Error Cases"
    },
    {
        // 仕様: 「変化なしはエラー」原則（セクション2.6）
        name: "Error - no change (sort already sorted)",
        code: "[ 1 2 3 ] SORT",
        expectError: true,
        category: "Error Cases"
    }
];
