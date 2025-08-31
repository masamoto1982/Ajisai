// js/gui/test.ts (英語ワード対応)

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

        this.showColoredInfo('Ajisai English Word System Tests Starting...', 'info');
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
            this.showColoredInfo('All tests passed! English word system working perfectly.', 'success');
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
            // ========== 基本ベクトル操作 ==========
            {
                name: "ベクトル作成",
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
                category: "Vector Operations"
            },
            {
                name: "位置指定取得（0オリジン）",
                code: "[ 1 2 3 4 5 ] 2 NTH",
                expectedWorkspace: [{ type: 'number', value: { numerator: 3, denominator: 1 } }],
                category: "Position Access"
            },
            {
                name: "ベクトル長取得",
                code: "[ 1 2 3 4 5 ] LENGTH",
                expectedWorkspace: [{ type: 'number', value: { numerator: 5, denominator: 1 } }],
                category: "Vector Info"
            },
            {
                name: "要素挿入",
                code: "[ 1 2 3 4 5 ] 1 9 INSERT",
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
                category: "Vector Modification"
            },
            {
                name: "要素置換",
                code: "[ 1 2 3 4 5 ] 1 9 REPLACE",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 9, denominator: 1 } },
                        { type: 'number', value: { numerator: 3, denominator: 1 } },
                        { type: 'number', value: { numerator: 4, denominator: 1 } },
                        { type: 'number', value: { numerator: 5, denominator: 1 } }
                    ]
                }],
                category: "Vector Modification"
            },
            {
                name: "要素削除",
                code: "[ 1 2 3 4 5 ] 1 REMOVE",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 3, denominator: 1 } },
                        { type: 'number', value: { numerator: 4, denominator: 1 } },
                        { type: 'number', value: { numerator: 5, denominator: 1 } }
                    ]
                }],
                category: "Vector Modification"
            },
            {
                name: "ベクトル結合",
                code: "[ 1 2 3 ] [ 4 5 ] CONCAT",
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
                category: "Vector Operations"
            },

            // ========== EVAL テスト ==========
            {
                name: "単純なEVAL",
                code: "[ 1 2 + ] EVAL",
                expectedWorkspace: [{ type: 'number', value: { numerator: 3, denominator: 1 } }],
                category: "EVAL"
            },
            {
                name: "複雑なEVAL",
                code: "[ [ 1 2 3 ] 1 NTH 5 + ] EVAL",
                expectedWorkspace: [{ type: 'number', value: { numerator: 7, denominator: 1 } }],
                category: "EVAL"
            },

            // ========== カスタムワード定義 ==========
            {
                name: "カスタムワード定義と実行",
                code: "[ 1 2 + ] \"ADD_ONE_TWO\" DEF ADD_ONE_TWO",
                expectedWorkspace: [{ type: 'number', value: { numerator: 3, denominator: 1 } }],
                category: "Custom Words"
            },

            // ========== エラーケース ==========
            {
                name: "存在しない位置アクセス",
                code: "[ 1 2 3 ] 10 NTH",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "空ベクトルアクセス",
                code: "[ ] 0 NTH",
                expectError: true,
                category: "Error Cases"
            }
        ];
    }
}
