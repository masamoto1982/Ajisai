// js/gui/test.ts

import type { GUI } from './main';

interface TestCase {
    name: string;
    code: string;
    expectedWorkspace?: any[];
    expectedOutput?: string;
    expectError?: boolean;
}

export class TestRunner {
    private gui: GUI;

    constructor(gui: GUI) {
        this.gui = gui;
    }

    async runAllTests(): Promise<void> {
        const testCases = this.getTestCases();
        let passed = 0;
        let failed = 0;

        this.gui.display.showInfo('🧪 Ajisai Tests Starting...\n');

        for (const testCase of testCases) {
            try {
                const result = await this.runSingleTest(testCase);
                if (result) {
                    passed++;
                    this.gui.display.showInfo(`✅ ${testCase.name}`, true);
                } else {
                    failed++;
                    this.gui.display.showInfo(`❌ ${testCase.name}`, true);
                }
            } catch (error) {
                failed++;
                this.gui.display.showInfo(`💥 ${testCase.name}: ${error}`, true);
            }
        }

        this.gui.display.showInfo(`\n📊 Results: ${passed} passed, ${failed} failed`, true);
        
        if (failed === 0) {
            this.gui.display.showInfo('🎉 All tests passed!', true);
        }
    }

    private async runSingleTest(testCase: TestCase): Promise<boolean> {
        // テスト前にワークスペースをクリア
        window.ajisaiInterpreter.reset();

        try {
            const result = window.ajisaiInterpreter.execute(testCase.code);
            
            if (testCase.expectError) {
                return result.status === 'ERROR';
            }

            if (result.status === 'ERROR') {
                return false;
            }

            // ワークスペースの検証
            if (testCase.expectedWorkspace) {
                const workspace = window.ajisaiInterpreter.get_workspace();
                return this.compareWorkspace(workspace, testCase.expectedWorkspace);
            }

            // 出力の検証
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

        return JSON.stringify(actual) === JSON.stringify(expected);
    }

    private getTestCases(): TestCase[] {
        return [
            // 基本算術テスト
            {
                name: "基本加算",
                code: "3 4 +",
                expectedWorkspace: [{ type: 'number', value: { numerator: 7, denominator: 1 } }]
            },
            {
                name: "漢字論理演算",
                code: "true false 且",
                expectedWorkspace: [{ type: 'boolean', value: false }]
            },
            {
                name: "漢字論理演算（或）",
                code: "true false 或",
                expectedWorkspace: [{ type: 'boolean', value: true }]
            },

            // 対称ペアテスト
            {
                name: "接/離 対称性",
                code: "5 [ 1 2 3 ] 接 離",
                expectedWorkspace: [
                    { type: 'number', value: { numerator: 5, denominator: 1 } },
                    { type: 'vector', value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 2, denominator: 1 } },
                        { type: 'number', value: { numerator: 3, denominator: 1 } }
                    ]}
                ]
            },
            {
                name: "追/除 対称性",
                code: "[ 1 2 ] 3 追 除",
                expectedWorkspace: [
                    { type: 'vector', value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 2, denominator: 1 } }
                    ]},
                    { type: 'number', value: { numerator: 3, denominator: 1 } }
                ]
            },

            // 新機能テスト
            {
                name: "複製（複）",
                code: "5 複 *",
                expectedWorkspace: [{ type: 'number', value: { numerator: 25, denominator: 1 } }]
            },
            {
                name: "選択（選）- true",
                code: "true 10 20 選",
                expectedWorkspace: [{ type: 'number', value: { numerator: 10, denominator: 1 } }]
            },
            {
                name: "選択（選）- false", 
                code: "false 10 20 選",
                expectedWorkspace: [{ type: 'number', value: { numerator: 20, denominator: 1 } }]
            },

            // 統一操作テスト
            {
                name: "要素数（数）",
                code: "[ 1 2 3 4 5 ] 数",
                expectedWorkspace: [
                    { type: 'vector', value: Array(5) },
                    { type: 'number', value: { numerator: 5, denominator: 1 } }
                ]
            },
            {
                name: "位置アクセス（在）",
                code: "1 [ 10 20 30 ] 在",
                expectedWorkspace: [{ type: 'number', value: { numerator: 20, denominator: 1 } }]
            },

            // 存在チェックテスト
            {
                name: "無チェック",
                code: "nil 無",
                expectedWorkspace: [{ type: 'boolean', value: true }]
            },
            {
                name: "有チェック",
                code: "5 有",
                expectedWorkspace: [{ type: 'boolean', value: true }]
            },

            // エラーテスト
            {
                name: "空ベクトルエラー",
                code: "[ ] 頭",
                expectError: true
            },

            // ワード定義テスト
            {
                name: "ワード定義",
                code: "[ 複 * ] \"平方\" 定 5 平方",
                expectedWorkspace: [{ type: 'number', value: { numerator: 25, denominator: 1 } }]
            }
        ];
    }
}
