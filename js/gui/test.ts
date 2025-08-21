// js/gui/test.ts（完全版）

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

        this.gui.display.showInfo('🧪 Ajisai Comprehensive Tests Starting...\n');

        for (const category of categories) {
            const categoryTests = testCases.filter(t => (t.category || 'Other') === category);
            let categoryPassed = 0;
            let categoryFailed = 0;

            this.gui.display.showInfo(`\n📁 ${category} (${categoryTests.length} tests)`, true);

            for (const testCase of categoryTests) {
                try {
                    const result = await this.runSingleTest(testCase);
                    if (result) {
                        categoryPassed++;
                        totalPassed++;
                        this.gui.display.showInfo(`  ✅ ${testCase.name}`, true);
                    } else {
                        categoryFailed++;
                        totalFailed++;
                        this.gui.display.showInfo(`  ❌ ${testCase.name}`, true);
                    }
                } catch (error) {
                    categoryFailed++;
                    totalFailed++;
                    this.gui.display.showInfo(`  💥 ${testCase.name}: ${error}`, true);
                }
            }

            this.gui.display.showInfo(`  📊 ${category}: ${categoryPassed}✅ ${categoryFailed}❌`, true);
        }

        const summary = `\n🏁 Final Results: ${totalPassed} passed, ${totalFailed} failed`;
        this.gui.display.showInfo(summary, true);
        
        if (totalFailed === 0) {
            this.gui.display.showInfo('🎉 All tests passed! Vector統一アーキテクチャ完全動作確認！', true);
        } else {
            this.gui.display.showInfo(`⚠️  ${totalFailed} tests failed. Review needed.`, true);
        }
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
                name: "以上比較",
                code: "5 5 >=",
                expectedWorkspace: [{ type: 'boolean', value: true }],
                category: "Comparison & Logic"
            },
            {
                name: "等価比較",
                code: "10 10 =",
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
                name: "無チェック（数値）",
                code: "5 無",
                expectedWorkspace: [{ type: 'boolean', value: false }],
                category: "Existence Check"
            },
            {
                name: "有チェック（数値）",
                code: "5 有",
                expectedWorkspace: [{ type: 'boolean', value: true }],
                category: "Existence Check"
            },
            {
                name: "有チェック（nil）",
                code: "nil 有",
                expectedWorkspace: [{ type: 'boolean', value: false }],
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
            {
                name: "Vector要素数",
                code: "[ 1 2 3 4 5 ] 数",
                expectedWorkspace: [
                    {
                        type: 'vector',
                        value: [
                            { type: 'number', value: { numerator: 1, denominator: 1 } },
                            { type: 'number', value: { numerator: 2, denominator: 1 } },
                            { type: 'number', value: { numerator: 3, denominator: 1 } },
                            { type: 'number', value: { numerator: 4, denominator: 1 } },
                            { type: 'number', value: { numerator: 5, denominator: 1 } }
                        ]
                    },
                    { type: 'number', value: { numerator: 5, denominator: 1 } }
                ],
                category: "Vector Basic"
            },

            // ========== 対称ペア操作 ==========
            {
                name: "接/離 対称性（完全）",
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
                name: "追/除 対称性（完全）",
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
            {
                name: "複数要素接続",
                code: "1 2 [ 3 4 ] 接 接",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 2, denominator: 1 } },
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 3, denominator: 1 } },
                        { type: 'number', value: { numerator: 4, denominator: 1 } }
                    ]
                }],
                category: "Symmetric Pairs"
            },

            // ========== ネストしたVector ==========
            {
                name: "ネストVector作成",
                code: "[ [ 1 2 ] [ 3 4 ] ]",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
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
                                { type: 'number', value: { numerator: 4, denominator: 1 } }
                            ]
                        }
                    ]
                }],
                category: "Nested Vectors"
            },
            {
                name: "ネストVector先頭",
                code: "[ [ 1 2 ] [ 3 4 ] ] 頭",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 2, denominator: 1 } }
                    ]
                }],
                category: "Nested Vectors"
            },
            {
                name: "3層ネスト",
                code: "[ [ [ 1 ] ] ]",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [{
                        type: 'vector',
                        value: [{
                            type: 'vector',
                            value: [{ type: 'number', value: { numerator: 1, denominator: 1 } }]
                        }]
                    }]
                }],
                category: "Nested Vectors"
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
            {
                name: "Vector複製",
                code: "[ 1 2 3 ] 複",
                expectedWorkspace: [
                    {
                        type: 'vector',
                        value: [
                            { type: 'number', value: { numerator: 1, denominator: 1 } },
                            { type: 'number', value: { numerator: 2, denominator: 1 } },
                            { type: 'number', value: { numerator: 3, denominator: 1 } }
                        ]
                    },
                    {
                        type: 'vector',
                        value: [
                            { type: 'number', value: { numerator: 1, denominator: 1 } },
                            { type: 'number', value: { numerator: 2, denominator: 1 } },
                            { type: 'number', value: { numerator: 3, denominator: 1 } }
                        ]
                    }
                ],
                category: "Clone Operations"
            },
            {
                name: "複数回複製",
                code: "3 複 複 + +",
                expectedWorkspace: [{ type: 'number', value: { numerator: 9, denominator: 1 } }],
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
            {
                name: "選択（nil）",
                code: "nil 10 20 選",
                expectedWorkspace: [{ type: 'number', value: { numerator: 20, denominator: 1 } }],
                category: "Selection"
            },
            {
                name: "選択（数値）",
                code: "5 10 20 選",
                expectedWorkspace: [{ type: 'number', value: { numerator: 10, denominator: 1 } }],
                category: "Selection"
            },
            {
                name: "Vector選択",
                code: "true [ 1 2 ] [ 3 4 ] 選",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'number', value: { numerator: 2, denominator: 1 } }
                    ]
                }],
                category: "Selection"
            },

            // ========== 位置アクセス ==========
            {
                name: "Vector位置アクセス（0番目）",
                code: "0 [ 10 20 30 ] 在",
                expectedWorkspace: [{ type: 'number', value: { numerator: 10, denominator: 1 } }],
                category: "Position Access"
            },
            {
                name: "Vector位置アクセス（1番目）",
                code: "1 [ 10 20 30 ] 在",
                expectedWorkspace: [{ type: 'number', value: { numerator: 20, denominator: 1 } }],
                category: "Position Access"
            },
            {
                name: "Vector位置アクセス（負のインデックス）",
                code: "-1 [ 10 20 30 ] 在",
                expectedWorkspace: [{ type: 'number', value: { numerator: 30, denominator: 1 } }],
                category: "Position Access"
            },
            {
                name: "ワークスペース位置アクセス",
                code: "10 20 30 1 在",
                expectedWorkspace: [
                    { type: 'number', value: { numerator: 10, denominator: 1 } },
                    { type: 'number', value: { numerator: 20, denominator: 1 } },
                    { type: 'number', value: { numerator: 30, denominator: 1 } },
                    { type: 'number', value: { numerator: 20, denominator: 1 } }
                ],
                category: "Position Access"
            },

            // ========== 実行操作 ==========
            {
                name: "値表示",
                code: "42 行",
                expectedOutput: "42",
                category: "Execute Operations"
            },
            {
                name: "Vector実行",
                code: "[ 3 4 + ] 行",
                expectedWorkspace: [{ type: 'number', value: { numerator: 7, denominator: 1 } }],
                category: "Execute Operations"
            },

            // ========== ワード定義・削除 ==========
            {
                name: "ワード定義と実行",
                code: "[ 複 * ] \"平方\" 定 5 平方",
                expectedWorkspace: [{ type: 'number', value: { numerator: 25, denominator: 1 } }],
                category: "Word Definition"
            },
            {
                name: "複雑なワード定義",
                code: "[ 複 複 + * ] \"三乗\" 定 3 三乗",
                expectedWorkspace: [{ type: 'number', value: { numerator: 27, denominator: 1 } }],
                category: "Word Definition"
            },

            // ========== 漢字・英語互換性 ==========
            {
                name: "漢字英語混在（論理演算）",
                code: "true false AND",
                expectedWorkspace: [{ type: 'boolean', value: false }],
                category: "Kanji-English Compatibility"
            },
            {
                name: "漢字英語混在（Vector操作）",
                code: "[ 1 2 3 ] HEAD",
                expectedWorkspace: [{ type: 'number', value: { numerator: 1, denominator: 1 } }],
                category: "Kanji-English Compatibility"
            },

            // ========== 複雑な組み合わせ ==========
            {
                name: "複雑なVector処理",
                code: "[ 1 2 3 ] 複 数 * 頭 +",
                expectedWorkspace: [{ type: 'number', value: { numerator: 4, denominator: 1 } }],
                category: "Complex Operations"
            },
            {
                name: "ネストVector操作",
                code: "[ [ 1 2 ] [ 3 4 ] ] 頭 尾 頭",
                expectedWorkspace: [{ type: 'number', value: { numerator: 2, denominator: 1 } }],
                category: "Complex Operations"
            },
            {
                name: "条件付きVector操作",
                code: "[ 1 2 3 ] 数 3 = [ 10 ] [ 20 ] 選 頭",
                expectedWorkspace: [{ type: 'number', value: { numerator: 10, denominator: 1 } }],
                category: "Complex Operations"
            },
            {
                name: "多段階処理",
                code: "5 複 + 複 * 複 /",
                expectedWorkspace: [{ type: 'number', value: { numerator: 100, denominator: 1 } }],
                category: "Complex Operations"
            },

            // ========== 実用的なプログラム例 ==========
            {
                name: "範囲チェック",
                code: "7 1 >= 7 10 <= 且",
                expectedWorkspace: [{ type: 'boolean', value: true }],
                category: "Practical Examples"
            },
            {
                name: "Vector最大値風（2要素）",
                code: "[ 5 8 ] 複 頭 SWAP 尾 頭 複 > 選",
                expectedWorkspace: [{ type: 'number', value: { numerator: 8, denominator: 1 } }],
                category: "Practical Examples"
            },
            {
                name: "データパイプライン",
                code: "[ 1 2 3 ] 複 複 数 SWAP 頭 +",
                expectedWorkspace: [{ type: 'number', value: { numerator: 4, denominator: 1 } }],
                category: "Practical Examples"
            },

            // ========== エラーケース ==========
            {
                name: "空Vector先頭エラー",
                code: "[ ] 頭",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "空Vector末尾エラー",
                code: "[ ] 尾",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "空Vector分離エラー",
                code: "[ ] 離",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "空Vector除去エラー",
                code: "[ ] 除",
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
            },
            {
                name: "位置アクセス範囲外",
                code: "10 [ 1 2 3 ] 在",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "未定義ワードエラー",
                code: "存在しないワード",
                expectError: true,
                category: "Error Cases"
            },

            // ========== 境界値テスト ==========
            {
                name: "単一要素Vector",
                code: "[ 42 ] 頭",
                expectedWorkspace: [{ type: 'number', value: { numerator: 42, denominator: 1 } }],
                category: "Boundary Tests"
            },
            {
                name: "単一要素Vector末尾",
                code: "[ 42 ] 尾",
                expectedWorkspace: [{ type: 'vector', value: [] }],
                category: "Boundary Tests"
            },
            {
                name: "負の分数",
                code: "-3/4 1/4 +",
                expectedWorkspace: [{ type: 'number', value: { numerator: -1, denominator: 2 } }],
                category: "Boundary Tests"
            },

            // ========== 型混在テスト ==========
            {
                name: "混在型Vector",
                code: "[ 1 true \"text\" nil ]",
                expectedWorkspace: [{
                    type: 'vector',
                    value: [
                        { type: 'number', value: { numerator: 1, denominator: 1 } },
                        { type: 'boolean', value: true },
                        { type: 'string', value: "text" },
                        { type: 'nil' }
                    ]
                }],
                category: "Mixed Types"
            },
            {
                name: "混在型Vector先頭",
                code: "[ 1 true \"text\" ] 頭",
                expectedWorkspace: [{ type: 'number', value: { numerator: 1, denominator: 1 } }],
                category: "Mixed Types"
            }
        ];
    }
}
