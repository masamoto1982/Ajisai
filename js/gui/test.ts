// js/gui/test.ts（構文表示・色分け対応版）

interface TestCase {
    name: string;
    code: string;
    expectedWorkspace?: any[];
    expectedOutput?: string;
    expectError?: boolean;
    category?: string;
}

export class TestRunner {
    private gui: any;

    constructor(gui: any) {
        this.gui = gui;
        console.log('TestRunner constructor called');
    }

    async runAllTests(): Promise<void> {
        console.log('runAllTests started');
        
        const testCases = this.getTestCases();
        const categories = [...new Set(testCases.map(t => t.category || 'Other'))];
        
        console.log(`Running ${testCases.length} test cases across ${categories.length} categories`);
        
        let totalPassed = 0;
        let totalFailed = 0;

        this.showColoredInfo('Ajisai Comprehensive Tests Starting...', 'info');
        this.showColoredInfo(`Total: ${testCases.length} tests across ${categories.length} categories\n`, 'info');

        for (const category of categories) {
            const categoryTests = testCases.filter(t => (t.category || 'Other') === category);
            let categoryPassed = 0;
            let categoryFailed = 0;

            this.showColoredInfo(`\n=== ${category} (${categoryTests.length} tests) ===`, 'info');

            for (const testCase of categoryTests) {
                try {
                    const result = await this.runSingleTest(testCase);
                    if (result) {
                        categoryPassed++;
                        totalPassed++;
                        this.showColoredInfo(`  PASS: ${testCase.name}`, 'success');
                        this.showCodeInfo(`        Code: ${testCase.code}`, 'code');
                    } else {
                        categoryFailed++;
                        totalFailed++;
                        this.showColoredInfo(`  FAIL: ${testCase.name}`, 'error');
                        this.showCodeInfo(`        Code: ${testCase.code}`, 'code');
                    }
                } catch (error) {
                    categoryFailed++;
                    totalFailed++;
                    this.showColoredInfo(`  ERROR: ${testCase.name}: ${error}`, 'error');
                    this.showCodeInfo(`         Code: ${testCase.code}`, 'code');
                }
            }

            this.showColoredInfo(`  Summary: ${categoryPassed} passed, ${categoryFailed} failed`, 'info');
        }

        this.showColoredInfo(`\n=== Final Results ===`, 'info');
        this.showColoredInfo(`Total Passed: ${totalPassed}`, 'success');
        
        if (totalFailed > 0) {
            this.showColoredInfo(`Total Failed: ${totalFailed}`, 'error');
            this.showColoredInfo('Review needed.', 'error');
        } else {
            this.showColoredInfo('All tests passed! Vector unified architecture fully operational.', 'success');
        }

        // 自動スクロール
        this.scrollToBottom();
    }

    private showColoredInfo(text: string, type: 'success' | 'error' | 'info' | 'code'): void {
        const outputElement = this.gui.elements.outputDisplay;
        
        const span = document.createElement('span');
        span.textContent = text + '\n';
        
        switch (type) {
            case 'success':
                span.style.color = '#28a745';  // 緑
                span.style.fontWeight = 'bold';
                break;
            case 'error':
                span.style.color = '#dc3545';  // 赤
                span.style.fontWeight = 'bold';
                break;
            case 'info':
                span.style.color = '#333';     // 通常
                break;
            case 'code':
                span.style.color = '#6c757d';  // グレー
                span.style.fontStyle = 'italic';
                span.style.fontSize = '0.9em';
                break;
        }
        
        outputElement.appendChild(span);
    }

    private showCodeInfo(text: string, type: 'code'): void {
        this.showColoredInfo(text, type);
    }

    private scrollToBottom(): void {
        const outputElement = this.gui.elements.outputDisplay;
        outputElement.scrollTop = outputElement.scrollHeight;
    }

    private async runSingleTest(testCase: TestCase): Promise<boolean> {
        // テスト前にリセット
        window.ajisaiInterpreter.reset();

        try {
            const result = window.ajisaiInterpreter.execute(testCase.code);
            
            if (testCase.expectError) {
                return result.status === 'ERROR';
            }

            if (result.status === 'ERROR') {
                console.log(`Unexpected error in ${testCase.name}: ${result.message}`);
                return false;
            }

            if (testCase.expectedWorkspace) {
                const workspace = window.ajisaiInterpreter.get_workspace();
                return this.compareWorkspace(workspace, testCase.expectedWorkspace);
            }

            if (testCase.expectedOutput) {
                return result.output === testCase.expectedOutput;
            }

            return true;
        } catch (error) {
            return testCase.expectError === true;
        }
    }

    private compareWorkspace(actual: any[], expected: any[]): boolean {
        if (actual.length !== expected.length) return false;
        
        for (let i = 0; i < actual.length; i++) {
            if (!this.compareValue(actual[i], expected[i])) {
                return false;
            }
        }
        return true;
    }

    private compareValue(actual: any, expected: any): boolean {
        if (expected.type === 'number' && actual.type === 'number') {
            const actualFrac = actual.value;
            const expectedFrac = expected.value;
            return actualFrac.numerator === expectedFrac.numerator && 
                   actualFrac.denominator === expectedFrac.denominator;
        }
        
        if (expected.type === 'vector' && actual.type === 'vector') {
            return this.compareWorkspace(actual.value, expected.value);
        }

        if (expected.type === 'boolean' && actual.type === 'boolean') {
            return actual.value === expected.value;
        }

        if (expected.type === 'string' && actual.type === 'string') {
            return actual.value === expected.value;
        }

        if (expected.type === 'nil' && actual.type === 'nil') {
            return true;
        }

        return JSON.stringify(actual) === JSON.stringify(expected);
    }

    private getTestCases(): TestCase[] {
        return [
            // ========== 基本算術演算 ==========
            {
                name: "基本加算",
                code: "3 4 +",
                expectedWorkspace: [{ type: 'number', value: { numerator: 7, denominator: 1 } }],
                category: "Basic Arithmetic"
            },
            {
                name: "基本減算",
                code: "10 3 -",
                expectedWorkspace: [{ type: 'number', value: { numerator: 7, denominator: 1 } }],
                category: "Basic Arithmetic"
            },
            {
                name: "基本乗算",
                code: "6 7 *",
                expectedWorkspace: [{ type: 'number', value: { numerator: 42, denominator: 1 } }],
                category: "Basic Arithmetic"
            },
            {
                name: "基本除算",
                code: "15 3 /",
                expectedWorkspace: [{ type: 'number', value: { numerator: 5, denominator: 1 } }],
                category: "Basic Arithmetic"
            },
            {
                name: "分数演算",
                code: "1/2 1/3 +",
                expectedWorkspace: [{ type: 'number', value: { numerator: 5, denominator: 6 } }],
                category: "Basic Arithmetic"
            },
            {
                name: "複合演算",
                code: "2 3 + 4 *",
                expectedWorkspace: [{ type: 'number', value: { numerator: 20, denominator: 1 } }],
                category: "Basic Arithmetic"
            },

            // ========== 比較・論理演算 ==========
            {
                name: "大なり比較",
                code: "5 3 >",
                expectedWorkspace: [{ type: 'boolean', value: true }],
                category: "Comparison & Logic"
            },
            {
                name: "論理否定（漢字）",
                code: "true 否",
                expectedWorkspace: [{ type: 'boolean', value: false }],
                category: "Comparison & Logic"
            },
            {
                name: "論理積（漢字）",
                code: "true false 且",
                expectedWorkspace: [{ type: 'boolean', value: false }],
                category: "Comparison & Logic"
            },
            {
                name: "論理和（漢字）",
                code: "true false 或",
                expectedWorkspace: [{ type: 'boolean', value: true }],
                category: "Comparison & Logic"
            },

            // ========== 存在チェック ==========
            {
                name: "無チェック（nil）",
                code: "nil 無",
                expectedWorkspace: [{ type: 'boolean', value: true }],
                category: "Existence Check"
            },
            {
                name: "有チェック（数値）",
                code: "5 有",
                expectedWorkspace: [{ type: 'boolean', value: true }],
                category: "Existence Check"
            },

            // ========== Vector基本操作 ==========
            {
                name: "Vectorリテラル",
                code: "[ 1 2 3 ]",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 2, denominator: 1 } },
                        { type: 'number', value: { numerator: 3, denominator: 1 } }
                    ]
                }],
                category: "Vector Basic"
            },
            {
                name: "Vector先頭取得",
                code: "[ 10 20 30 ] 頭",
                expectedWorkspace: [{ type: 'number', value: { numerator: 10, denominator: 1 } }],
                category: "Vector Basic"
            },
            {
                name: "Vector末尾取得",
                code: "[ 10 20 30 ] 尾",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 20, denominator: 1 } },
                        { type: 'number', value: { numerator: 30, denominator: 1 } }
                    ]
                }],
                category: "Vector Basic"
            },

            // ========== 対称ペア操作 ==========
            {
                name: "接/離 対称性",
                code: "5 [ 1 2 3 ] 接 離",
                expectedWorkspace: [
                    { type: 'number', value: { numerator: 5, denominator: 1 } },
                    {
                        type: 'vector',
                        value: [
                            { type: 'number', value: { numerator: 1, denominator: 1 } },
                            { type: 'number', value: { numerator: 2, denominator: 1 } },
                            { type: 'number', value: { numerator: 3, denominator: 1 } }
                        ]
                    }
                ],
                category: "Symmetric Pairs"
            },
            {
                name: "追/除 対称性",
                code: "[ 1 2 ] 3 追 除",
                expectedWorkspace: [
                    {
                        type: 'vector',
                        value: [
                            { type: 'number', value: { numerator: 1, denominator: 1 } },
                            { type: 'number', value: { numerator: 2, denominator: 1 } }
                        ]
                    },
                    { type: 'number', value: { numerator: 3, denominator: 1 } }
                ],
                category: "Symmetric Pairs"
            },

            // ========== 複製機能 ==========
            {
                name: "基本複製",
                code: "5 複",
                expectedWorkspace: [
                    { type: 'number', value: { numerator: 5, denominator: 1 } },
                    { type: 'number', value: { numerator: 5, denominator: 1 } }
                ],
                category: "Clone Operations"
            },
            {
                name: "複製して自乗",
                code: "7 複 *",
                expectedWorkspace: [{ type: 'number', value: { numerator: 49, denominator: 1 } }],
                category: "Clone Operations"
            },

            // ========== 条件選択 ==========
            {
                name: "選択（真）",
                code: "true 10 20 選",
                expectedWorkspace: [{ type: 'number', value: { numerator: 10, denominator: 1 } }],
                category: "Selection"
            },
            {
                name: "選択（偽）",
                code: "false 10 20 選",
                expectedWorkspace: [{ type: 'number', value: { numerator: 20, denominator: 1 } }],
                category: "Selection"
            },

            // ========== 位置アクセス ==========
            {
                name: "Vector位置アクセス",
                code: "1 [ 10 20 30 ] 在",
                expectedWorkspace: [{ type: 'number', value: { numerator: 20, denominator: 1 } }],
                category: "Position Access"
            },

            // ========== ワード定義 ==========
            {
                name: "ワード定義と実行",
                code: "[ 複 * ] \"平方\" 定 5 平方",
                expectedWorkspace: [{ type: 'number', value: { numerator: 25, denominator: 1 } }],
                category: "Word Definition"
            },

            // ========== 複雑な組み合わせ ==========
            {
                name: "複雑なVector処理",
                code: "[ 1 2 3 ] 複 数 * 頭 +",
                expectedWorkspace: [{ type: 'number', value: { numerator: 4, denominator: 1 } }],
                category: "Complex Operations"
            },

            // ========== エラーケース ==========
            {
                name: "空Vector先頭エラー",
                code: "[ ] 頭",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "ワークスペース不足エラー",
                code: "+",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "ゼロ除算エラー",
                code: "5 0 /",
                expectError: true,
                category: "Error Cases"
            }
        ];
    }
}
