// js/gui/test.ts (新しいワード体系対応)

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

        this.showColoredInfo('Ajisai New File System Tests Starting...', 'info');
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
            this.showColoredInfo('All tests passed! File-based vector operations working perfectly.', 'success');
        }

        this.scrollToBottom();
    }

    private showColoredInfo(text: string, type: 'success' | 'error' | 'info' | 'code'): void {
        const outputElement = this.gui.elements.outputDisplay;
        
        const span = document.createElement('span');
        span.textContent = text + '\n';
        
        switch (type) {
            case 'success':
                span.style.color = '#28a745';
                span.style.fontWeight = 'bold';
                break;
            case 'error':
                span.style.color = '#dc3545';
                span.style.fontWeight = 'bold';
                break;
            case 'info':
                span.style.color = '#333';
                break;
            case 'code':
                span.style.color = '#6c757d';
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
            // ========== ファイル/ページ基本操作 ==========
            {
                name: "ファイル作成",
                code: "[ 1 2 3 4 5 ]",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 2, denominator: 1 } },
                        { type: 'number', value: { numerator: 3, denominator: 1 } },
                        { type: 'number', value: { numerator: 4, denominator: 1 } },
                        { type: 'number', value: { numerator: 5, denominator: 1 } }
                    ]
                }],
                category: "File Operations"
            },
            {
                name: "3ページ目を見る",
                code: "[ 1 2 3 4 5 ] 2 頁 取得",
                expectedWorkspace: [{ type: 'number', value: { numerator: 3, denominator: 1 } }],
                category: "Page Access"
            },
            {
                name: "ページ数を数える",
                code: "[ 1 2 3 4 5 ] 頁数 取得",
                expectedWorkspace: [{ type: 'number', value: { numerator: 5, denominator: 1 } }],
                category: "Page Count"
            },
            {
                name: "2ページ目に新しいページを挿入",
                code: "[ 1 2 3 4 5 ] 1 頁 9 挿入",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 9, denominator: 1 } },
                        { type: 'number', value: { numerator: 2, denominator: 1 } },
                        { type: 'number', value: { numerator: 3, denominator: 1 } },
                        { type: 'number', value: { numerator: 4, denominator: 1 } },
                        { type: 'number', value: { numerator: 5, denominator: 1 } }
                    ]
                }],
                category: "Page Modification"
            },
            {
                name: "2ページ目を置き換える",
                code: "[ 1 2 3 4 5 ] 1 頁 9 置換",
                expectedWorkspace: [
                    {
                        type: 'vector',
                        value: [
                            { type: 'number', value: { numerator: 1, denominator: 1 } },
                            { type: 'number', value: { numerator: 9, denominator: 1 } },
                            { type: 'number', value: { numerator: 3, denominator: 1 } },
                            { type: 'number', value: { numerator: 4, denominator: 1 } },
                            { type: 'number', value: { numerator: 5, denominator: 1 } }
                        ]
                    },
                    { type: 'number', value: { numerator: 2, denominator: 1 } }
                ],
                category: "Page Modification"
            },
            {
                name: "2ページ目を削除する",
                code: "[ 1 2 3 4 5 ] 1 頁 削除",
                expectedWorkspace: [
                    {
                        type: 'vector',
                        value: [
                            { type: 'number', value: { numerator: 1, denominator: 1 } },
                            { type: 'number', value: { numerator: 3, denominator: 1 } },
                            { type: 'number', value: { numerator: 4, denominator: 1 } },
                            { type: 'number', value: { numerator: 5, denominator: 1 } }
                        ]
                    },
                    { type: 'number', value: { numerator: 2, denominator: 1 } }
                ],
                category: "Page Modification"
            },
            {
                name: "2つのファイルを合併する",
                code: "[ 1 2 3 ] [ 4 5 ] 合併",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 2, denominator: 1 } },
                        { type: 'number', value: { numerator: 3, denominator: 1 } },
                        { type: 'number', value: { numerator: 4, denominator: 1 } },
                        { type: 'number', value: { numerator: 5, denominator: 1 } }
                    ]
                }],
                category: "File Operations"
            },
            {
                name: "ファイルを2ページ目で分離する",
                code: "[ 1 2 3 4 5 ] 2 頁 分離",
                expectedWorkspace: [
                    {
                        type: 'vector',
                        value: [
                            { type: 'number', value: { numerator: 1, denominator: 1 } },
                            { type: 'number', value: { numerator: 2, denominator: 1 } }
                        ]
                    },
                    {
                        type: 'vector',
                        value: [
                            { type: 'number', value: { numerator: 3, denominator: 1 } },
                            { type: 'number', value: { numerator: 4, denominator: 1 } },
                            { type: 'number', value: { numerator: 5, denominator: 1 } }
                        ]
                    }
                ],
                category: "File Operations"
            },

            // ========== カスタムワード定義 ==========
            {
                name: "ファイル複製ワード定義",
                code: "[ 0 頁 取得 ] \"ファイル先頭\" DEF [ 1 2 3 ] ファイル先頭",
                expectedWorkspace: [{ type: 'number', value: { numerator: 1, denominator: 1 } }],
                category: "Custom Words"
            },

            // ========== エラーケース ==========
            {
                name: "存在しないページアクセス",
                code: "[ 1 2 3 ] 10 頁 取得",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "空ファイルページアクセス",
                code: "[ ] 0 頁 取得",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "不十分なパラメータ（挿入）",
                code: "[ 1 2 3 ] 挿入",
                expectError: true,
                category: "Error Cases"
            }
        ];
    }
}
