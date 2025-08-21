// js/gui/test.ts

interface TestCase {
    name: string;
    code: string;
    expectedWorkspace?: any[];
    expectedOutput?: string;
    expectError?: boolean;
}

export class TestRunner {
    private gui: any; // GUI型の循環参照を避けるため any を使用

    constructor(gui: any) {
        this.gui = gui;
        console.log('TestRunner constructor called');
    }

    async runAllTests(): Promise<void> {
        console.log('runAllTests started');
        
        const testCases = this.getTestCases();
        console.log(`Running ${testCases.length} test cases`);
        
        let passed = 0;
        let failed = 0;

        this.gui.display.showInfo('🧪 Ajisai Tests Starting...\n');

        for (const testCase of testCases) {
            console.log(`Running test: ${testCase.name}`);
            try {
                const result = await this.runSingleTest(testCase);
                if (result) {
                    passed++;
                    console.log(`✅ ${testCase.name} PASSED`);
                    this.gui.display.showInfo(`✅ ${testCase.name}`, true);
                } else {
                    failed++;
                    console.log(`❌ ${testCase.name} FAILED`);
                    this.gui.display.showInfo(`❌ ${testCase.name}`, true);
                }
            } catch (error) {
                failed++;
                console.error(`💥 ${testCase.name} ERROR:`, error);
                this.gui.display.showInfo(`💥 ${testCase.name}: ${error}`, true);
            }
        }

        const summary = `\n📊 Results: ${passed} passed, ${failed} failed`;
        console.log(summary);
        this.gui.display.showInfo(summary, true);
        
        if (failed === 0) {
            this.gui.display.showInfo('🎉 All tests passed!', true);
        }
    }

    private async runSingleTest(testCase: TestCase): Promise<boolean> {
        console.log(`Testing: ${testCase.code}`);
        
        // テスト前にワークスペースをクリア
        window.ajisaiInterpreter.reset();

        try {
            const result = window.ajisaiInterpreter.execute(testCase.code);
            console.log('Test result:', result);
            
            if (testCase.expectError) {
                const success = result.status === 'ERROR';
                console.log(`Expected error, got ${result.status}: ${success}`);
                return success;
            }

            if (result.status === 'ERROR') {
                console.log(`Unexpected error: ${result.message}`);
                return false;
            }

            // ワークスペースの検証
            if (testCase.expectedWorkspace) {
                const workspace = window.ajisaiInterpreter.get_workspace();
                console.log('Actual workspace:', workspace);
                console.log('Expected workspace:', testCase.expectedWorkspace);
                const success = this.compareWorkspace(workspace, testCase.expectedWorkspace);
                console.log(`Workspace comparison: ${success}`);
                return success;
            }

            // 出力の検証
            if (testCase.expectedOutput) {
                const success = result.output === testCase.expectedOutput;
                console.log(`Output comparison: expected "${testCase.expectedOutput}", got "${result.output}": ${success}`);
                return success;
            }

            console.log('Test passed (no specific expectations)');
            return true;
        } catch (error) {
            console.error('Test execution error:', error);
            return testCase.expectError === true;
        }
    }

    private compareWorkspace(actual: any[], expected: any[]): boolean {
        console.log(`Comparing workspace lengths: actual ${actual.length}, expected ${expected.length}`);
        if (actual.length !== expected.length) return false;
        
        for (let i = 0; i < actual.length; i++) {
            if (!this.compareValue(actual[i], expected[i])) {
                console.log(`Value mismatch at index ${i}:`, actual[i], 'vs', expected[i]);
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

        return JSON.stringify(actual) === JSON.stringify(expected);
    }

    private getTestCases(): TestCase[] {
        return [
            {
                name: "基本加算",
                code: "3 4 +",
                expectedWorkspace: [{ type: 'number', value: { numerator: 7, denominator: 1 } }]
            },
            {
                name: "複製テスト",
                code: "5 複",
                expectedWorkspace: [
                    { type: 'number', value: { numerator: 5, denominator: 1 } },
                    { type: 'number', value: { numerator: 5, denominator: 1 } }
                ]
            },
            {
                name: "論理演算テスト",
                code: "true false 且",
                expectedWorkspace: [{ type: 'boolean', value: false }]
            },
            {
                name: "空ベクトルエラー",
                code: "[ ] 頭",
                expectError: true
            }
        ];
    }
}
