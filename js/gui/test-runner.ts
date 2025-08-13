// js/gui/test-runner.ts

export interface TestCase {
    name: string;
    code: string;
    expectedResult?: {
        stackTop?: any;
        stackLength?: number;
        output?: string;
        error?: boolean;
        autoNamed?: boolean;
        autoNamedWord?: string;
    };
    description?: string;
}

export interface TestResult {
    name: string;
    description: string;
    code: string;
    passed: boolean;
    error: Error | null;
    actual: string;
    expected: string;
    actualValue: any;
    expectedValue: any;
}

export class TestRunner {
    private testCases: TestCase[] = [
        // === 加算（+）- 全6パターン ===
        {
            name: "加算_前置記法_自動定義",
            code: "+ 3 4",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置記法による加算の自動ワード定義: + 3 4"
        },
        {
            name: "加算_中置記法_自動定義",
            code: "3 + 4", 
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置記法による加算の自動ワード定義: 3 + 4"
        },
        {
            name: "加算_後置記法_自動定義",
            code: "3 4 +",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "後置記法による加算の自動ワード定義: 3 4 +"
        },
        {
            name: "加算_混合パターン1_自動定義",
            code: "+ 2 3 5 +",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置と後置の混合による加算の自動ワード定義: + 2 3 5 +"
        },
        {
            name: "加算_混合パターン2_自動定義",
            code: "1 + 2 3 +",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置と後置の混合による加算の自動ワード定義: 1 + 2 3 +"
        },
        {
            name: "加算_混合パターン3_自動定義",
            code: "+ 1 2 3 + 4",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置と中置の混合による加算の自動ワード定義: + 1 2 3 + 4"
        },

        // === 減算（-）- 全6パターン ===
        {
            name: "減算_前置記法_自動定義",
            code: "- 10 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置記法による減算の自動ワード定義: - 10 3"
        },
        {
            name: "減算_中置記法_自動定義",
            code: "10 - 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置記法による減算の自動ワード定義: 10 - 3"
        },
        {
            name: "減算_後置記法_自動定義",
            code: "10 3 -",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "後置記法による減算の自動ワード定義: 10 3 -"
        },
        {
            name: "減算_混合パターン1_自動定義",
            code: "- 15 5 2 -",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置と後置の混合による減算の自動ワード定義: - 15 5 2 -"
        },
        {
            name: "減算_混合パターン2_自動定義",
            code: "20 - 5 3 -",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置と後置の混合による減算の自動ワード定義: 20 - 5 3 -"
        },
        {
            name: "減算_混合パターン3_自動定義",
            code: "- 20 5 10 - 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置と中置の混合による減算の自動ワード定義: - 20 5 10 - 3"
        },

        // === 乗算（*）- 全6パターン ===
        {
            name: "乗算_前置記法_自動定義",
            code: "* 6 7",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置記法による乗算の自動ワード定義: * 6 7"
        },
        {
            name: "乗算_中置記法_自動定義",
            code: "6 * 7",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置記法による乗算の自動ワード定義: 6 * 7"
        },
        {
            name: "乗算_後置記法_自動定義",
            code: "6 7 *",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "後置記法による乗算の自動ワード定義: 6 7 *"
        },
        {
            name: "乗算_混合パターン1_自動定義",
            code: "* 2 3 4 *",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置と後置の混合による乗算の自動ワード定義: * 2 3 4 *"
        },
        {
            name: "乗算_混合パターン2_自動定義",
            code: "2 * 3 4 *",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置と後置の混合による乗算の自動ワード定義: 2 * 3 4 *"
        },
        {
            name: "乗算_混合パターン3_自動定義",
            code: "* 2 3 4 * 5",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置と中置の混合による乗算の自動ワード定義: * 2 3 4 * 5"
        },

        // === 除算（/）- 全6パターン ===
        {
            name: "除算_前置記法_自動定義",
            code: "/ 15 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置記法による除算の自動ワード定義: / 15 3"
        },
        {
            name: "除算_中置記法_自動定義",
            code: "15 / 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置記法による除算の自動ワード定義: 15 / 3"
        },
        {
            name: "除算_後置記法_自動定義",
            code: "15 3 /",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "後置記法による除算の自動ワード定義: 15 3 /"
        },
        {
            name: "除算_混合パターン1_自動定義",
            code: "/ 20 4 2 /",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置と後置の混合による除算の自動ワード定義: / 20 4 2 /"
        },
        {
            name: "除算_混合パターン2_自動定義",
            code: "20 / 4 2 /",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置と後置の混合による除算の自動ワード定義: 20 / 4 2 /"
        },
        {
            name: "除算_混合パターン3_自動定義",
            code: "/ 60 3 15 / 5",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置と中置の混合による除算の自動ワード定義: / 60 3 15 / 5"
        },

        // === 分数での記法テスト（自動定義） ===
        {
            name: "分数_前置記法_自動定義",
            code: "+ 1/2 1/3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "分数の前置加算の自動ワード定義: + 1/2 1/3"
        },
        {
            name: "分数_中置記法_自動定義",
            code: "1/2 + 1/3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "分数の中置加算の自動ワード定義: 1/2 + 1/3"
        },
        {
            name: "分数_後置記法_自動定義",
            code: "1/2 1/3 +",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "分数の後置加算の自動ワード定義: 1/2 1/3 +"
        },

        // === 小数点記法での演算テスト（自動定義） ===
        {
            name: "小数_前置記法_自動定義",
            code: "+ 0.5 0.25",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "小数の前置加算の自動ワード定義: + 0.5 0.25"
        },
        {
            name: "小数_中置記法_自動定義",
            code: "0.5 + 0.25",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "小数の中置加算の自動ワード定義: 0.5 + 0.25"
        },
        {
            name: "小数_後置記法_自動定義",
            code: "0.5 0.25 +",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "小数の後置加算の自動ワード定義: 0.5 0.25 +"
        },

        // === 単一値の直接実行（リテラル） ===
        {
            name: "数値リテラル",
            code: "42",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "数値リテラルの直接実行"
        },
        {
            name: "分数リテラル",
            code: "1/2",
            expectedResult: { stackTop: { type: "number", value: { numerator: 1, denominator: 2 } } },
            description: "分数リテラルの直接実行"
        },
        {
            name: "小数リテラル",
            code: "0.75",
            expectedResult: { stackTop: { type: "number", value: { numerator: 3, denominator: 4 } } },
            description: "小数リテラルの直接実行"
        },
        {
            name: "真偽値リテラル_true",
            code: "true",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "真偽値リテラル（true）の直接実行"
        },
        {
            name: "真偽値リテラル_false",
            code: "false",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "真偽値リテラル（false）の直接実行"
        },
        {
            name: "nil_リテラル",
            code: "nil",
            expectedResult: { stackTop: { type: "nil", value: null } },
            description: "nilリテラルの直接実行"
        },

        // === ベクトルリテラルの直接実行 ===
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
            description: "ベクトルリテラルの直接実行"
        },
        {
            name: "空ベクトルリテラル",
            code: "[ ]",
            expectedResult: { stackTop: { type: "vector", value: [] } },
            description: "空ベクトルリテラルの直接実行"
        },

        // === 単一ビルトインワードの直接実行 ===
        {
            name: "DUP_単体実行",
            code: "42 DUP",
            expectedResult: { stackLength: 2 },
            description: "DUPワードの直接実行"
        },
        {
            name: "LENGTH_単体実行",
            code: "[ 1 2 3 4 5 ] LENGTH",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "LENGTHワードの直接実行"
        },

        // === 明示的カスタムワード定義 ===
        {
            name: "明示的ワード定義",
            code: "3 4 + \"SEVEN\" DEF",
            expectedResult: { output: "Defined: SEVEN" },
            description: "明示的なカスタムワード定義"
        },

        // === 複雑な自動ワード定義 ===
        {
            name: "複雑な式_自動定義",
            code: "2 + 3 4 *",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "複雑な式の自動ワード定義: 2 + 3 4 *"
        },
        {
            name: "四則混合_自動定義",
            code: "/ 20 - 10 6 + 2 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "四則混合演算の自動ワード定義: / 20 - 10 6 + 2 3"
        },

        // === エラーケースのテスト ===
        {
            name: "スタックアンダーフロー_DUP",
            code: "DUP",
            expectedResult: { error: true },
            description: "スタックアンダーフローエラー（DUP）"
        },
        {
            name: "未知のワード",
            code: "UNKNOWN_WORD",
            expectedResult: { error: true },
            description: "未知のワードエラー"
        },
        {
            name: "ベクトル空エラー_HEAD", 
            code: "[ ] HEAD",
            expectedResult: { error: true },
            description: "空ベクトルのHEADエラー"
        }
    ];

    // 既存のメソッドは変更なし
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

    private formatTestValue(value: any): string {
        if (value === null || value === undefined) {
            return '(empty)';
        }
        
        if (typeof value === 'object') {
            if (value.error === true) {
                return `エラー: ${value.message || 'Unknown error'}`;
            }
            
            if (value.autoNamed === true) {
                return `自動ワード定義: ${value.autoNamedWord || '生成済み'}`;
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
            window.ajisaiInterpreter.reset();
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

            if (executeResult.status === 'ERROR' || executeResult.error === true) {
                result.actualValue = { 
                    error: true, 
                    message: executeResult.message || 'Unknown error' 
                };
                result.passed = testCase.expectedResult?.error === true;
            } else if (executeResult.status === 'OK') {
                const stack = window.ajisaiInterpreter.get_stack();
                const output = executeResult.output || "";
                
                result.actualValue = {
                    stackTop: stack.length > 0 ? stack[stack.length - 1] : null,
                    stackLength: stack.length,
                    output: output,
                    error: false,
                    autoNamed: executeResult.autoNamed || false,
                    autoNamedWord: executeResult.autoNamedWord || null
                };

                result.passed = this.compareResults(result.expectedValue, result.actualValue);
            }

            result.actual = this.formatTestValue(result.actualValue);
            result.expected = this.formatTestValue(result.expectedValue);

            return result;
        } catch (error) {
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
        
        if (expected.autoNamed !== undefined) {
            if (expected.autoNamed !== actual.autoNamed) return false;
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
