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
            name: "分数演算",
            code: "1/2 1/3 +",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 6 } } },
            description: "分数の加算"
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
            name: "スタック操作_DUP",
            code: "42 DUP",
            expectedResult: { stackLength: 2 },
            description: "スタックトップの複製"
        },
        {
            name: "比較演算_大なり",
            code: "5 3 >",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "大なり比較"
        },
        {
            name: "論理演算_AND",
            code: "true false AND",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "論理積演算"
        },
        {
            name: "カスタムワード定義",
            code: "3 4 + \"SEVEN\" DEF",
            expectedResult: { output: "Defined: SEVEN" },
            description: "カスタムワードの明示的定義"
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
        }
    ];

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
            
            // コードを実行
            const executeResult = window.ajisaiInterpreter.execute(testCase.code);
            
            const result: TestResult = {
                name: testCase.name,
                description: testCase.description || "",
                passed: false,
                error: null,
                actual: {},
                expected: testCase.expectedResult || {}
            };

            if (executeResult.status === 'OK') {
                // 成功した場合の検証
                const stack = window.ajisaiInterpreter.get_stack();
                const output = executeResult.output || "";
                
                result.actual = {
                    stackTop: stack.length > 0 ? stack[stack.length - 1] : null,
                    stackLength: stack.length,
                    output: output,
                    error: false
                };

                // 期待値との比較
                result.passed = this.compareResults(result.expected, result.actual);
            } else {
                // エラーが発生した場合
                result.actual = { error: true, message: executeResult.message };
                result.passed = testCase.expectedResult?.error === true;
            }

            return result;
        } catch (error) {
            return {
                name: testCase.name,
                description: testCase.description || "",
                passed: testCase.expectedResult?.error === true,
                error: error as Error,
                actual: { error: true, message: (error as Error).message },
                expected: testCase.expectedResult || {}
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
    passed: boolean;
    error: Error | null;
    actual: any;
    expected: any;
}
