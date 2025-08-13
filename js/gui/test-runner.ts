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

export class TestRunner {
    private testCases: TestCase[] = [
        // ========================================
        // 1. 基本的な算術演算 - 加算
        // ========================================
        {
            name: "加算_中置記法",
            code: "3 + 4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "中置記法による加算"
        },
        {
            name: "加算_前置記法",
            code: "+ 3 4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "前置記法による加算"
        },
        {
            name: "加算_後置記法",
            code: "3 4 +",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "後置記法（RPN）による加算"
        },
        {
            name: "加算_混合_中置前置",
            code: "2 + 3 + 4",
            expectedResult: { stackTop: { type: "number", value: { numerator: 9, denominator: 1 } } },
            description: "中置記法の連続"
        },
        {
            name: "加算_混合_後置中置",
            code: "1 2 + + 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 6, denominator: 1 } } },
            description: "後置と中置の混合"
        },

        // ========================================
        // 2. 基本的な算術演算 - 減算
        // ========================================
        {
            name: "減算_中置記法",
            code: "10 - 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "中置記法による減算"
        },
        {
            name: "減算_前置記法",
            code: "- 10 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "前置記法による減算"
        },
        {
            name: "減算_後置記法",
            code: "10 3 -",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "後置記法による減算"
        },
        {
            name: "減算_連続",
            code: "20 - 5 - 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 12, denominator: 1 } } },
            description: "連続した減算"
        },

        // ========================================
        // 3. 基本的な算術演算 - 乗算
        // ========================================
        {
            name: "乗算_中置記法",
            code: "6 * 7",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "中置記法による乗算"
        },
        {
            name: "乗算_前置記法",
            code: "* 6 7",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "前置記法による乗算"
        },
        {
            name: "乗算_後置記法",
            code: "6 7 *",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "後置記法による乗算"
        },

        // ========================================
        // 4. 基本的な算術演算 - 除算
        // ========================================
        {
            name: "除算_中置記法",
            code: "15 / 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "中置記法による除算"
        },
        {
            name: "除算_前置記法",
            code: "/ 15 3",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "前置記法による除算"
        },
        {
            name: "除算_後置記法",
            code: "15 3 /",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "後置記法による除算"
        },

        // ========================================
        // 5. ベクトル基本操作
        // ========================================
        {
            name: "ベクトル_リテラル",
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
            name: "ベクトル_LENGTH",
            code: "[ 1 2 3 4 5 ] LENGTH",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "ベクトルの長さ"
        },

        // その他のテストケース...
        // (簡略化のため一部のみ表示)

        // ========================================
        // エラーケース
        // ========================================
        {
            name: "エラー_ゼロ除算",
            code: "5 0 /",
            expectedResult: { error: true },
            description: "ゼロ除算エラー"
        },
        {
            name: "エラー_スタックアンダーフロー",
            code: "+",
            expectedResult: { error: true },
            description: "スタックアンダーフローエラー"
        },
        {
            name: "エラー_未知のワード",
            code: "UNKNOWN_WORD",
            expectedResult: { error: true },
            description: "未知のワードエラー"
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
                // 自動命名されたワードがある場合、それを実行する
                if (executeResult.autoNamed && executeResult.autoNamedWord) {
                    // 自動生成されたワードを実行
                    const secondResult = window.ajisaiInterpreter.execute(executeResult.autoNamedWord);
                    
                    if (secondResult.status === 'ERROR') {
                        result.actualValue = { 
                            error: true, 
                            message: secondResult.message || 'Unknown error' 
                        };
                        result.passed = testCase.expectedResult?.error === true;
                    } else {
                        // 成功した場合の検証
                        const stack = window.ajisaiInterpreter.get_stack();
                        const output = secondResult.output || "";
                        
                        result.actualValue = {
                            stackTop: stack.length > 0 ? stack[stack.length - 1] : null,
                            stackLength: stack.length,
                            output: output,
                            error: false
                        };
                        
                        // 期待値との比較
                        result.passed = this.compareResults(result.expectedValue, result.actualValue);
                    }
                } else {
                    // 自動命名されていない場合（単一ワードの実行など）
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
