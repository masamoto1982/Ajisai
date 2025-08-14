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
    phase?: 'definition' | 'execution';
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
    phase: string;
}

export class TestRunner {
    private testCases: TestCase[] = [
        // ===== PHASE 1: カスタムワード定義テスト =====
        
        // === 基本四則演算の定義テスト（前置記法） ===
        {
            name: "定義_加算_前置記法",
            code: "+ 3 4",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置記法による加算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_減算_前置記法",
            code: "- 10 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置記法による減算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_乗算_前置記法",
            code: "* 6 7",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置記法による乗算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_除算_前置記法",
            code: "/ 15 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "前置記法による除算の自動ワード定義",
            phase: 'definition'
        },

        // === 基本四則演算の定義テスト（中置記法） ===
        {
            name: "定義_加算_中置記法",
            code: "3 + 4",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置記法による加算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_減算_中置記法",
            code: "10 - 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置記法による減算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_乗算_中置記法",
            code: "6 * 7",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置記法による乗算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_除算_中置記法",
            code: "15 / 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "中置記法による除算の自動ワード定義",
            phase: 'definition'
        },

        // === 基本四則演算の定義テスト（後置記法） ===
        {
            name: "定義_加算_後置記法",
            code: "3 4 +",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "後置記法による加算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_減算_後置記法",
            code: "10 3 -",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "後置記法による減算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_乗算_後置記法",
            code: "6 7 *",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "後置記法による乗算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_除算_後置記法",
            code: "15 3 /",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "後置記法による除算の自動ワード定義",
            phase: 'definition'
        },

        // === 比較演算の定義テスト ===
        {
            name: "定義_大なり比較",
            code: "5 > 3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "大なり比較の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_等価比較",
            code: "5 = 5",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "等価比較の自動ワード定義",
            phase: 'definition'
        },

        // === 論理演算の定義テスト ===
        {
            name: "定義_論理積",
            code: "true AND false",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "論理積の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_論理和",
            code: "true OR false",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "論理和の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_論理否定",
            code: "NOT true",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "論理否定の自動ワード定義",
            phase: 'definition'
        },

        // === 段階的二項演算の定義テスト ===
        {
            name: "定義_段階的演算_2段階",
            code: "1 2 + 3 *",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "2段階の二項演算の自動ワード定義: (1+2)*3",
            phase: 'definition'
        },
        {
            name: "定義_段階的演算_3段階",
            code: "1 2 + 3 * 4 +",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "3段階の二項演算の自動ワード定義: ((1+2)*3)+4",
            phase: 'definition'
        },

        // === 分数・小数の定義テスト ===
        {
            name: "定義_分数演算",
            code: "1/2 + 1/3",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "分数の加算の自動ワード定義",
            phase: 'definition'
        },
        {
            name: "定義_小数演算",
            code: "0.5 + 0.25",
            expectedResult: { autoNamed: true, stackLength: 0 },
            description: "小数の加算の自動ワード定義",
            phase: 'definition'
        },

        // === エラーケースの定義テスト ===
        {
            name: "定義_エラー_不完全な二項演算",
            code: "+ 2 3 5",
            expectedResult: { error: true },
            description: "不完全な二項演算によるエラー",
            phase: 'definition'
        },
        {
            name: "定義_エラー_演算子のみ",
            code: "+",
            expectedResult: { error: true },
            description: "演算子のみによるエラー",
            phase: 'definition'
        },
        {
            name: "定義_エラー_オペランド不足",
            code: "2 +",
            expectedResult: { error: true },
            description: "オペランド不足によるエラー",
            phase: 'definition'
        },

        // ===== PHASE 2: カスタムワード実行テスト =====
        
        // === 基本リテラルの実行テスト ===
        {
            name: "実行_数値リテラル",
            code: "42",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "数値リテラルの直接実行",
            phase: 'execution'
        },
        {
            name: "実行_分数リテラル",
            code: "1/2",
            expectedResult: { stackTop: { type: "number", value: { numerator: 1, denominator: 2 } } },
            description: "分数リテラルの直接実行",
            phase: 'execution'
        },
        {
            name: "実行_真偽値リテラル",
            code: "true",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "真偽値リテラルの直接実行",
            phase: 'execution'
        },

        // === ベクトルリテラルの実行テスト ===
        {
            name: "実行_ベクトルリテラル",
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
            description: "ベクトルリテラルの直接実行",
            phase: 'execution'
        },

        // === ビルトインワードの実行テスト ===
        {
            name: "実行_DUP",
            code: "42 DUP",
            expectedResult: { stackLength: 2 },
            description: "DUPワードの直接実行",
            phase: 'execution'
        },
        {
            name: "実行_LENGTH",
            code: "[ 1 2 3 4 5 ] LENGTH",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "LENGTHワードの直接実行",
            phase: 'execution'
        },

        // === 明示的カスタムワード定義と実行 ===
        {
            name: "実行_明示的ワード定義",
            code: "3 4 + \"SEVEN\" DEF",
            expectedResult: { output: "Defined: SEVEN" },
            description: "明示的なカスタムワード定義",
            phase: 'execution'
        },

        // === エラーケースの実行テスト ===
        {
            name: "実行_エラー_スタックアンダーフロー",
            code: "DUP",
            expectedResult: { error: true },
            description: "スタックアンダーフローエラー",
            phase: 'execution'
        },
        {
            name: "実行_エラー_未知のワード",
            code: "UNKNOWN_WORD",
            expectedResult: { error: true },
            description: "未知のワードエラー",
            phase: 'execution'
        }
    ];

    // カスタムワード実行テスト（定義→実行の2段階テスト）
    private customWordExecutionTests: Array<{
        definitionCode: string;
        executionCode: string;
        expectedResult: any;
        description: string;
    }> = [
        {
            definitionCode: "3 + 4",
            executionCode: "3_4_ADD",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "加算ワード(3_4_ADD)の実行: 3+4=7"
        },
        {
            definitionCode: "10 - 3",
            executionCode: "10_3_SUB",
            expectedResult: { stackTop: { type: "number", value: { numerator: 7, denominator: 1 } } },
            description: "減算ワード(10_3_SUB)の実行: 10-3=7"
        },
        {
            definitionCode: "6 * 7",
            executionCode: "6_7_MUL",
            expectedResult: { stackTop: { type: "number", value: { numerator: 42, denominator: 1 } } },
            description: "乗算ワード(6_7_MUL)の実行: 6*7=42"
        },
        {
            definitionCode: "15 / 3",
            executionCode: "15_3_DIV",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 1 } } },
            description: "除算ワード(15_3_DIV)の実行: 15/3=5"
        },
        {
            definitionCode: "5 > 3",
            executionCode: "5_3_GT",
            expectedResult: { stackTop: { type: "boolean", value: true } },
            description: "比較ワード(5_3_GT)の実行: 5>3=true"
        },
        {
            definitionCode: "true AND false",
            executionCode: "TRUE_FALSE_AND",
            expectedResult: { stackTop: { type: "boolean", value: false } },
            description: "論理積ワード(TRUE_FALSE_AND)の実行: true AND false=false"
        },
        {
            definitionCode: "1/2 + 1/3",
            executionCode: "1D2_1D3_ADD",
            expectedResult: { stackTop: { type: "number", value: { numerator: 5, denominator: 6 } } },
            description: "分数加算ワード(1D2_1D3_ADD)の実行: 1/2+1/3=5/6"
        },
        {
            definitionCode: "1 2 + 3 *",
            executionCode: "1_2_ADD_3_MUL",
            expectedResult: { stackTop: { type: "number", value: { numerator: 9, denominator: 1 } } },
            description: "段階的演算ワード(1_2_ADD_3_MUL)の実行: (1+2)*3=9"
        }
    ];

    // 既存のフォーマットメソッド
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
        
        // Phase 1: 基本テストケース
        for (const testCase of this.testCases) {
            const result = await this.runSingleTest(testCase);
            results.push(result);
        }
        
        // Phase 2: カスタムワード実行テスト（定義→実行）
        for (const customTest of this.customWordExecutionTests) {
            const result = await this.runCustomWordExecutionTest(customTest);
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
                expectedValue: testCase.expectedResult || {},
                phase: testCase.phase || 'unknown'
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
                expectedValue: testCase.expectedResult || {},
                phase: testCase.phase || 'unknown'
            };
        }
    }

    async runCustomWordExecutionTest(customTest: {
        definitionCode: string;
        executionCode: string;
        expectedResult: any;
        description: string;
    }): Promise<TestResult> {
        try {
            window.ajisaiInterpreter.reset();
            
            // Step 1: ワード定義
            const defineResult = window.ajisaiInterpreter.execute(customTest.definitionCode);
            if (defineResult.status !== 'OK') {
                throw new Error(`Definition failed: ${defineResult.message}`);
            }
            
            // Step 2: ワード実行
            const executeResult = window.ajisaiInterpreter.execute(customTest.executionCode);
            
            const result: TestResult = {
                name: `実行_カスタムワード_${customTest.executionCode}`,
                description: customTest.description,
                code: `${customTest.definitionCode} → ${customTest.executionCode}`,
                passed: false,
                error: null,
                actual: "",
                expected: "",
                actualValue: {},
                expectedValue: customTest.expectedResult,
                phase: 'custom_execution'
            };

            if (executeResult.status === 'ERROR' || executeResult.error === true) {
                result.actualValue = { 
                    error: true, 
                    message: executeResult.message || 'Unknown error' 
                };
                result.passed = customTest.expectedResult?.error === true;
            } else if (executeResult.status === 'OK') {
                const stack = window.ajisaiInterpreter.get_stack();
                const output = executeResult.output || "";
                
                result.actualValue = {
                    stackTop: stack.length > 0 ? stack[stack.length - 1] : null,
                    stackLength: stack.length,
                    output: output,
                    error: false
                };

                result.passed = this.compareResults(result.expectedValue, result.actualValue);
            }

            result.actual = this.formatTestValue(result.actualValue);
            result.expected = this.formatTestValue(result.expectedValue);

            return result;
        } catch (error) {
            return {
                name: `実行_カスタムワード_${customTest.executionCode}`,
                description: customTest.description,
                code: `${customTest.definitionCode} → ${customTest.executionCode}`,
                passed: false,
                error: error as Error,
                actual: `予期しないエラー: ${(error as Error).message}`,
                expected: this.formatTestValue(customTest.expectedResult),
                actualValue: { error: true, message: (error as Error).message },
                expectedValue: customTest.expectedResult,
                phase: 'custom_execution'
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
