// js/gui/test-runner.ts

export interface TestCase {
    name: string;
    code: string;
    expectedResult?: {
        stackTop?: any;
        stackLength?: number;
        output?: string;
        error?: boolean;
    };
    description?: string;
}

export class TestRunner {
    private testCases: TestCase[] = [
        // 基本的な算術演算
        {
            name: "中置記法_加算",
            code: "3 + 4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "中置記法による加算"
        },
        {
            name: "後置記法_加算", 
            code: "3 4 +",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "後置記法（RPN）による加算"
        },
        {
            name: "前置記法_加算",
            code: "+ 3 4", 
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "前置記法による加算"
        },
        {
            name: "中置記法_減算",
            code: "10 - 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "中置記法による減算"
        },
        {
            name: "中置記法_乗算",
            code: "6 * 7",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "中置記法による乗算"
        },
        {
            name: "中置記法_除算",
            code: "15 / 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "中置記法による除算"
        },
        {
            name: "分数演算",
            code: "1/2 1/3 +",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 6 } } },
            description: "分数の加算"
        },
        {
            name: "小数点記法",
            code: "0.5 0.25 +",
            expectedResult: { stackTop: { type: "number", value: { numerator: 3, denominator: 4 } } },
            description: "小数点記法での加算"
        },
        {
            name: "ベクトルリテラル",
            code: "[ 1 2 3 ]",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 1, denominator: 1 } },
                        { type: "number", value: { numerator: 2, denominator: 1 } },
                        { type: "number", value: { numerator: 3, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルリテラルの作成"
        },
        {
            name: "ベクトル長さ",
            code: "[ 1 2 3 4 5 ] LENGTH",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "ベクトルの長さを取得"
        },
        {
            name: "ベクトル先頭要素",
            code: "[ 10 20 30 ] HEAD",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "ベクトルの先頭要素を取得"
        },
        {
            name: "ベクトル暗黙反復",
            code: "[ 1 2 3 ] 2 *",
            expectedResult: { 
                stackTop: { 
                    type: "vector", 
                    value: [
                        { type: "number", value: { numerator: 2, denominator: 1 } },
                        { type: "number", value: { numerator: 4, denominator: 1 } },
                        { type: "number", value: { numerator: 6, denominator: 1 } }
                    ]
                }
            },
            description: "ベクトルに対する暗黙の反復"
        },
        {
            name: "スタック操作_DUP",
            code: "42 DUP",
            expectedResult: { stackLength: 2 },
            description: "スタックトップの複製"
        },
        {
            name: "スタック操作_SWAP",
            code: "10 20 SWAP",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "スタック上位2つの交換"
        },
        {
            name: "レジスタ操作",
            code: "42 >R R@",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "レジスタへの移動と取得"
        },
        {
            name: "比較演算_大なり",
            code: "5 3 >",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "大なり比較"
        },
        {
            name: "比較演算_等価",
            code: "5 5 =",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "等価比較"
        },
        {
            name: "論理演算_AND",
            code: "true false AND",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "論理積演算"
        },
        {
            name: "論理演算_OR",
            code: "true false OR",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "論理和演算"
        },
        {
            name: "論理演算_NOT",
            code: "true NOT",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "論理否定演算"
        },
        {
            name: "条件選択_true",
            code: "true 10 20 ?",
            expectedResult: { stackTop: { type: "number", value: { numerator: 10, denominator: 1 } } },
            description: "条件選択（true の場合）"
        },
        {
            name: "条件選択_false",
            code: "false 10 20 ?",
            expectedResult: { stackTop: { type: "number", value: { numerator: 20, denominator: 1 } } },
            description: "条件選択（false の場合）"
        },
        {
            name: "Nil関連_NIL判定",
            code: "nil NIL?",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "NIL判定"
        },
        {
            name: "Nil関連_DEFAULT",
            code: "nil 42 DEFAULT",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "デフォルト値の適用"
        },
        {
            name: "カスタムワード定義",
            code: "3 4 + \"SEVEN\" DEF",
            expectedResult: { output: "Defined: SEVEN" },
            description: "カスタムワードの明示的定義"
        },
        {
            name: "自動ワード生成",
            code: "2 3 +",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "自動ワード生成による式の評価"
        },
        {
            name: "除算ゼロエラー",
            code: "5 0 /",
            expectedResult: { error: true },
            description: "ゼロ除算エラーのテスト"
        },
        {
            name: "スタックアンダーフロー",
            code: "+",
            expectedResult: { error: true },
            description: "スタックアンダーフローエラーのテスト"
        },
        {
            name: "未知のワード",
            code: "UNKNOWN_WORD",
            expectedResult: { error: true },
            description: "未知のワードエラーのテスト"
        }
    ];

    // Ajisai言語の値をフォーマットする関数
    private formatValue(value: any): string {
        if (!value) return 'undefined';
        
        switch (value.type) {
            case 'number':
                if (typeof value.value === 'object' && value.value !== null && 'numerator' in value.value && 'denominator' in value.value) {
                    const frac = value.value as { numerator: number; denominator: number };
                    if (frac.denominator === 1) {
                        return frac.numerator.toString();
                    } else {
                        return `${frac.numerator}/${frac.denominator}`;
                    }
                }
                return typeof value.value === 'string' ? value.value : String(value.value);
            case 'string':
                return `"${value.value}"`;
            case 'symbol':
                return String(value.value);
            case 'boolean':
                return value.value ? 'true' : 'false';
            case 'vector':
                if (Array.isArray(value.value)) {
                    return `[ ${value.value.map((v: any) => this.formatValue(v)).join(' ')} ]`;
                }
                return '[ ]';
            case 'nil':
                return 'nil';
            default:
                return String(value.value);
        }
    }

    // テスト結果の表示用フォーマット
    private formatTestValue(value: any): string {
        if (value === null || value === undefined) {
            return '(empty)';
        }
        
        if (typeof value === 'object') {
            if (value.error === true) {
                return `エラー: ${value.message || 'Unknown error'}`;
            }
            
            if (value.stackTop !== undefined) {
                return `スタックトップ: ${this.formatValue(value.stackTop)}`;
            }
            
            if (value.stackLength !== undefined) {
                return `スタック長: ${value.stackLength}`;
            }
            
            if (value.output !== undefined) {
                return `出力: "${value.output}"`;
            }
            
            // Ajisai値の場合
            if (value.type) {
                return this.formatValue(value);
            }
        }
        
        return String(value);
    }

    async runAllTests(): Promise<TestResult[]> {
        const results: TestResult[] = [];
        
        for (const testCase of this.testCases) {
            const result = await this.runSingleTest(testCase);
            results.push(result);
        }
        
        return results;
    }

    async runSingleTest(testCase: TestCase): Promise<TestResult> {
    try {
        // インタープリターをリセット
        window.ajisaiInterpreter.reset();
        
        // コードを実行（常にオブジェクトが返される）
        const executeResult = window.ajisaiInterpreter.execute(testCase.code);
        
        const result: TestResult = {
            name: testCase.name,
            description: testCase.description || "",
            code: testCase.code,
            passed: false,
            error: null,
            actual: "",
            expected: "",
            actualValue: {},
            expectedValue: testCase.expectedResult || {}
        };

        // エラーステータスのチェック
        if (executeResult.status === 'ERROR' || executeResult.error === true) {
            // エラーが発生した場合
            result.actualValue = { 
                error: true, 
                message: executeResult.message || 'Unknown error' 
            };
            result.passed = testCase.expectedResult?.error === true;
        } else if (executeResult.status === 'OK') {
            // 成功した場合の検証
            const stack = window.ajisaiInterpreter.get_stack();
            const output = executeResult.output || "";
            
            result.actualValue = {
                stackTop: stack.length > 0 ? stack[stack.length - 1] : null,
                stackLength: stack.length,
                output: output,
                error: false
            };

            // 期待値との比較
            result.passed = this.compareResults(result.expectedValue, result.actualValue);
        }

        // フォーマットされた文字列を設定
        result.actual = this.formatTestValue(result.actualValue);
        result.expected = this.formatTestValue(result.expectedValue);

        return result;
    } catch (error) {
        // 予期しない例外が発生した場合のフォールバック
        return {
            name: testCase.name,
            description: testCase.description || "",
            code: testCase.code,
            passed: testCase.expectedResult?.error === true,
            error: error as Error,
            actual: `予期しないエラー: ${(error as Error).message}`,
            expected: this.formatTestValue(testCase.expectedResult || {}),
            actualValue: { error: true, message: (error as Error).message },
            expectedValue: testCase.expectedResult || {}
        };
    }
}

    private compareResults(expected: any, actual: any): boolean {
        if (expected.error !== undefined) {
            return expected.error === actual.error;
        }
        
        if (expected.stackLength !== undefined) {
            if (expected.stackLength !== actual.stackLength) return false;
        }
        
        if (expected.stackTop !== undefined) {
            return this.deepEqual(expected.stackTop, actual.stackTop);
        }
        
        if (expected.output !== undefined) {
            return actual.output.includes(expected.output);
        }
        
        return true;
    }

    private deepEqual(a: any, b: any): boolean {
        if (a === b) return true;
        if (a == null || b == null) return false;
        if (typeof a !== typeof b) return false;
        
        if (typeof a === 'object') {
            const keysA = Object.keys(a);
            const keysB = Object.keys(b);
            if (keysA.length !== keysB.length) return false;
            
            for (const key of keysA) {
                if (!keysB.includes(key)) return false;
                if (!this.deepEqual(a[key], b[key])) return false;
            }
            return true;
        }
        
        return false;
    }
}

export interface TestResult {
    name: string;
    description: string;
    code: string;
    passed: boolean;
    error: Error | null;
    actual: string;      // フォーマットされた表示用文字列
    expected: string;    // フォーマットされた表示用文字列
    actualValue: any;    // 実際の値（比較用）
    expectedValue: any;  // 期待値（比較用）
}
