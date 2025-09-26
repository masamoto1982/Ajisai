import type { Value, Fraction } from '../wasm-types';

interface TestCase {
    name: string;
    code: string;
    expectedWorkspace?: Value[];
    expectedOutput?: string;
    expectError?: boolean;
    category?: string;
}

export class TestRunner {
    private gui: any;

    constructor(gui: any) {
        this.gui = gui;
    }

    async runAllTests(): Promise<void> {
        const testCases = this.getTestCases();
        let totalPassed = 0;
        let totalFailed = 0;
        const failedTests: string[] = [];

        this.showColoredInfo('=== Ajisai Comprehensive Test Suite ===', 'info');
        this.showColoredInfo(`Running ${testCases.length} test cases...`, 'info');

        // テスト開始前に一度だけリセット
        await this.resetInterpreter();

        // カテゴリ別にテストを実行
        const categories = [...new Set(testCases.map(t => t.category))].filter(Boolean);
        
        for (const category of categories) {
            this.showColoredInfo(`\n--- ${category} Tests ---`, 'info');
            const categoryTests = testCases.filter(t => t.category === category);
            
            for (const testCase of categoryTests) {
                try {
                    const result = await this.runSingleTestWithDetails(testCase);
                    if (result.passed) {
                        totalPassed++;
                        this.showTestResult(testCase, result, true);
                    } else {
                        totalFailed++;
                        failedTests.push(testCase.name);
                        this.showTestResult(testCase, result, false);
                    }
                } catch (error) {
                    totalFailed++;
                    failedTests.push(testCase.name);
                    this.showTestError(testCase, error);
                }
            }
        }

        this.showColoredInfo(`\n=== Final Results ===`, 'info');
        this.showColoredInfo(`Total Passed: ${totalPassed}`, 'success');
        
        if (totalFailed > 0) {
            this.showColoredInfo(`Total Failed: ${totalFailed}`, 'error');
            this.showColoredInfo(`Failed tests: ${failedTests.join(', ')}`, 'error');
        } else {
            this.showColoredInfo('🎉 All tests passed!', 'success');
        }
    }
    
    private async resetInterpreter(): Promise<void> {
        if (window.ajisaiInterpreter) {
            window.ajisaiInterpreter.restore_workspace([]);
        }
    }
    
    private async runSingleTestWithDetails(testCase: TestCase): Promise<{
        passed: boolean;
        actualWorkspace?: Value[];
        actualOutput?: string;
        errorMessage?: string;
        reason?: string;
    }> {
        // 各テスト前にworkspaceをクリア
        await this.resetInterpreter();
        
        const result = window.ajisaiInterpreter.execute(testCase.code);
        
        if (testCase.expectError) {
            return {
                passed: result.status === 'ERROR',
                errorMessage: result.message,
                reason: result.status === 'ERROR' ? 'Expected error occurred' : 'Expected error but execution succeeded'
            };
        }
        
        if (result.status === 'ERROR') {
            return {
                passed: false,
                errorMessage: result.message,
                reason: 'Unexpected error during execution'
            };
        }

        if (testCase.expectedWorkspace) {
            const workspace = window.ajisaiInterpreter.get_workspace();
            const matches = this.compareWorkspace(workspace, testCase.expectedWorkspace);
            return {
                passed: matches,
                actualWorkspace: workspace,
                reason: matches ? 'Workspace matches expected' : 'Workspace mismatch'
            };
        }
        
        if (testCase.expectedOutput) {
            const matches = result.output?.trim() === testCase.expectedOutput.trim();
            return {
                passed: matches,
                actualOutput: result.output,
                reason: matches ? 'Output matches expected' : 'Output mismatch'
            };
        }
        
        return {
            passed: true,
            reason: 'Test completed successfully'
        };
    }
    
    private showTestResult(testCase: TestCase, result: any, passed: boolean): void {
        const statusIcon = passed ? '✓' : '✗';
        const statusText = passed ? 'PASS' : 'FAIL';
        const statusColor = passed ? 'success' : 'error';
        
        this.showColoredInfo(`${statusIcon} ${statusText}: ${testCase.name}`, statusColor);
        this.showColoredInfo(`  Code: ${testCase.code}`, 'info');
        
        if (testCase.expectError) {
            this.showColoredInfo(`  Expected: Error should occur`, 'info');
            if (result.errorMessage) {
                this.showColoredInfo(`  Actual error: ${result.errorMessage}`, 'info');
            }
        } else if (testCase.expectedWorkspace) {
            this.showColoredInfo(`  Expected workspace: ${this.formatWorkspaceForDisplay(testCase.expectedWorkspace)}`, 'info');
            if (result.actualWorkspace) {
                this.showColoredInfo(`  Actual workspace: ${this.formatWorkspaceForDisplay(result.actualWorkspace)}`, 'info');
            }
        } else if (testCase.expectedOutput) {
            this.showColoredInfo(`  Expected output: "${testCase.expectedOutput}"`, 'info');
            if (result.actualOutput !== undefined) {
                this.showColoredInfo(`  Actual output: "${result.actualOutput}"`, 'info');
            }
        }
        
        if (result.reason) {
            this.showColoredInfo(`  Result: ${result.reason}`, passed ? 'success' : 'error');
        }

        if (!passed && result.errorMessage) {
            this.showColoredInfo(`  Error Message from Rust: ${result.errorMessage}`, 'error');
        }
        
        this.showColoredInfo('', 'info'); // 空行
    }
    
    private showTestError(testCase: TestCase, error: any): void {
        this.showColoredInfo(`✗ ERROR: ${testCase.name}`, 'error');
        this.showColoredInfo(`  Code: ${testCase.code}`, 'info');
        this.showColoredInfo(`  Error: ${error}`, 'error');
        this.showColoredInfo('', 'info'); // 空行
    }
    
    private formatWorkspaceForDisplay(workspace: Value[]): string {
        if (workspace.length === 0) {
            return '[]';
        }
        
        const formatted = workspace.map(value => this.formatValueForDisplay(value)).join(', ');
        return `[${formatted}]`;
    }
    
    private formatValueForDisplay(value: Value): string {
        switch (value.type) {
            case 'number':
                const frac = value.value as Fraction;
                if (frac.denominator === '1') {
                    return frac.numerator;
                } else {
                    return `${frac.numerator}/${frac.denominator}`;
                }
            case 'string':
                return `'${value.value}'`;
            case 'boolean':
                return value.value ? 'TRUE' : 'FALSE';
            case 'nil':
                return 'NIL';
            case 'vector':
                if (Array.isArray(value.value)) {
                    const elements = value.value.map(v => this.formatValueForDisplay(v)).join(' ');
                    const brackets = this.getBracketPair(value.bracketType);
                    return `${brackets[0]}${elements ? ' ' + elements + ' ' : ''}${brackets[1]}`;
                }
                return '[]';
            default:
                return JSON.stringify(value.value);
        }
    }
    
    private getBracketPair(bracketType?: string): [string, string] {
        switch (bracketType) {
            case 'curly': return ['{', '}'];
            case 'round': return ['(', ')'];
            default: return ['[', ']'];
        }
    }
    
    private compareWorkspace(actual: Value[], expected: Value[]): boolean {
        if (actual.length !== expected.length) {
            return false;
        }
        
        for (let i = 0; i < actual.length; i++) {
            const actualItem = actual[i];
            const expectedItem = expected[i];
            
            if (actualItem === undefined || expectedItem === undefined) {
                return false;
            }
            
            if (!this.compareValue(actualItem, expectedItem)) {
                return false;
            }
        }
        
        return true;
    }

    private compareValue(actual: Value, expected: Value): boolean {
        if (actual.type !== expected.type) {
            return false;
        }

        switch (actual.type) {
            case 'number':
                const actualFrac = actual.value as Fraction;
                const expectedFrac = expected.value as Fraction;
                return actualFrac.numerator === expectedFrac.numerator && 
                       actualFrac.denominator === expectedFrac.denominator;
            
            case 'vector':
                if (!Array.isArray(actual.value) || !Array.isArray(expected.value)) {
                    return false;
                }
                return this.compareWorkspace(actual.value, expected.value);
            
            case 'string':
            case 'boolean':
                return JSON.stringify(actual.value) === JSON.stringify(expected.value);
                
            case 'nil':
                return true;
                
            default:
                return JSON.stringify(actual.value) === JSON.stringify(expected.value);
        }
    }
    
    private showColoredInfo(text: string, type: 'success' | 'error' | 'info'): void {
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
        }
        
        outputElement.appendChild(span);
    }
    
    private createVector(elements: Value[], bracketType: 'square' | 'curly' | 'round' = 'square'): Value {
        return {
            type: 'vector',
            value: elements,
            bracketType: bracketType
        };
    }
    
    private createNumber(numerator: string, denominator: string = '1'): Value {
        return {
            type: 'number',
            value: { numerator, denominator }
        };
    }
    
    private createString(value: string): Value {
        return {
            type: 'string',
            value: value
        };
    }
    
    private createBoolean(value: boolean): Value {
        return {
            type: 'boolean',
            value: value
        };
    }
    
    private createNil(): Value {
        return {
            type: 'nil',
            value: null
        };
    }
    
    private getTestCases(): TestCase[] {
        return [
            // === 基本データ型 ===
            {
                name: "整数リテラル",
                code: "[ 42 ]",
                expectedWorkspace: [this.createVector([this.createNumber('42')])],
                category: "Basic Data Types"
            },
            {
                name: "負の整数",
                code: "[ -15 ]",
                expectedWorkspace: [this.createVector([this.createNumber('-15')])],
                category: "Basic Data Types"
            },
            {
                name: "小数",
                code: "[ 3.14 ]",
                expectedWorkspace: [this.createVector([this.createNumber('157', '50')])],
                category: "Basic Data Types"
            },
            {
                name: "分数",
                code: "[ 3/4 ]",
                expectedWorkspace: [this.createVector([this.createNumber('3', '4')])],
                category: "Basic Data Types"
            },
            {
                name: "文字列リテラル",
                code: "[ 'Hello World' ]",
                expectedWorkspace: [this.createVector([this.createString('Hello World')])],
                category: "Basic Data Types"
            },
            {
                name: "真偽値true",
                code: "[ TRUE ]",
                expectedWorkspace: [this.createVector([this.createBoolean(true)])],
                category: "Basic Data Types"
            },
            {
                name: "真偽値false",
                code: "[ FALSE ]",
                expectedWorkspace: [this.createVector([this.createBoolean(false)])],
                category: "Basic Data Types"
            },
            {
                name: "Nil値",
                code: "[ NIL ]",
                expectedWorkspace: [this.createVector([this.createNil()])],
                category: "Basic Data Types"
            },
            {
                name: "空のベクトル",
                code: "[ ]",
                expectedWorkspace: [this.createVector([])],
                category: "Basic Data Types"
            },
            {
                name: "複数要素のベクトル",
                code: "[ 1 2 3 ]",
                expectedWorkspace: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('2'),
                    this.createNumber('3')
                ])],
                category: "Basic Data Types"
            },

            // === 算術演算 ===
            {
                name: "整数の加算",
                code: "[ 5 ] [ 3 ] +",
                expectedWorkspace: [this.createVector([this.createNumber('8')])],
                category: "Arithmetic"
            },
            {
                name: "整数の減算",
                code: "[ 10 ] [ 3 ] -",
                expectedWorkspace: [this.createVector([this.createNumber('7')])],
                category: "Arithmetic"
            },
            {
                name: "整数の乗算",
                code: "[ 4 ] [ 7 ] *",
                expectedWorkspace: [this.createVector([this.createNumber('28')])],
                category: "Arithmetic"
            },
            {
                name: "整数の除算",
                code: "[ 15 ] [ 3 ] /",
                expectedWorkspace: [this.createVector([this.createNumber('5')])],
                category: "Arithmetic"
            },
            {
                name: "分数の加算",
                code: "[ 1/2 ] [ 1/3 ] +",
                expectedWorkspace: [this.createVector([this.createNumber('5', '6')])],
                category: "Arithmetic"
            },
            {
                name: "分数の減算",
                code: "[ 3/4 ] [ 1/4 ] -",
                expectedWorkspace: [this.createVector([this.createNumber('1', '2')])],
                category: "Arithmetic"
            },
            {
                name: "分数の乗算",
                code: "[ 2/3 ] [ 3/4 ] *",
                expectedWorkspace: [this.createVector([this.createNumber('1', '2')])],
                category: "Arithmetic"
            },
            {
                name: "分数の除算",
                code: "[ 2/3 ] [ 1/2 ] /",
                expectedWorkspace: [this.createVector([this.createNumber('4', '3')])],
                category: "Arithmetic"
            },

            // === 比較演算 ===
            {
                name: "等価判定（真）",
                code: "[ 5 ] [ 5 ] =",
                expectedWorkspace: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "等価判定（偽）",
                code: "[ 5 ] [ 3 ] =",
                expectedWorkspace: [this.createVector([this.createBoolean(false)])],
                category: "Comparison"
            },
            {
                name: "より小さい（真）",
                code: "[ 3 ] [ 5 ] <",
                expectedWorkspace: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "より小さい（偽）",
                code: "[ 5 ] [ 3 ] <",
                expectedWorkspace: [this.createVector([this.createBoolean(false)])],
                category: "Comparison"
            },
            {
                name: "以下（真）",
                code: "[ 5 ] [ 5 ] <=",
                expectedWorkspace: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "より大きい（真）",
                code: "[ 7 ] [ 3 ] >",
                expectedWorkspace: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "以上（真）",
                code: "[ 5 ] [ 5 ] >=",
                expectedWorkspace: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },

            // === 論理演算 ===
            {
                name: "論理AND（真）",
                code: "[ TRUE ] [ TRUE ] AND",
                expectedWorkspace: [this.createVector([this.createBoolean(true)])],
                category: "Logic"
            },
            {
                name: "論理AND（偽）",
                code: "[ TRUE ] [ FALSE ] AND",
                expectedWorkspace: [this.createVector([this.createBoolean(false)])],
                category: "Logic"
            },
            {
                name: "論理OR（真）",
                code: "[ TRUE ] [ FALSE ] OR",
                expectedWorkspace: [this.createVector([this.createBoolean(true)])],
                category: "Logic"
            },
            {
                name: "論理OR（偽）",
                code: "[ FALSE ] [ FALSE ] OR",
                expectedWorkspace: [this.createVector([this.createBoolean(false)])],
                category: "Logic"
            },
            {
                name: "論理NOT（真→偽）",
                code: "[ TRUE ] NOT",
                expectedWorkspace: [this.createVector([this.createBoolean(false)])],
                category: "Logic"
            },
            {
                name: "論理NOT（偽→真）",
                code: "[ FALSE ] NOT",
                expectedWorkspace: [this.createVector([this.createBoolean(true)])],
                category: "Logic"
            },

            // === ワークスペース操作 ===
            {
                name: "DUP - 複製",
                code: "[ 42 ] DUP",
                expectedWorkspace: [
                    this.createVector([this.createNumber('42')]),
                    this.createVector([this.createNumber('42')])
                ],
                category: "Workspace"
            },
            {
                name: "SWAP - 交換",
                code: "[ 1 ] [ 2 ] SWAP",
                expectedWorkspace: [
                    this.createVector([this.createNumber('2')]),
                    this.createVector([this.createNumber('1')])
                ],
                category: "Workspace"
            },
            {
                name: "ROT - 回転",
                code: "[ 1 ] [ 2 ] [ 3 ] ROT",
                expectedWorkspace: [
                    this.createVector([this.createNumber('2')]),
                    this.createVector([this.createNumber('3')]),
                    this.createVector([this.createNumber('1')])
                ],
                category: "Workspace"
            },

            // === ベクトル操作 - 位置指定（0オリジン） ===
            {
                name: "GET - 正のインデックス",
                code: "[ 10 20 30 ] [ 1 ] GET",
                expectedWorkspace: [this.createVector([this.createNumber('20')])],
                category: "Vector Operations"
            },
            {
                name: "GET - 負のインデックス",
                code: "[ 10 20 30 ] [ -1 ] GET",
                expectedWorkspace: [this.createVector([this.createNumber('30')])],
                category: "Vector Operations"
            },
            {
                name: "INSERT - 要素挿入",
                code: "[ 1 3 ] [ 1 ] [ 2 ] INSERT",
                expectedWorkspace: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('2'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },
            {
                name: "REPLACE - 要素置換",
                code: "[ 1 2 3 ] [ 1 ] [ 5 ] REPLACE",
                expectedWorkspace: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('5'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },
            {
                name: "REMOVE - 要素削除",
                code: "[ 1 2 3 ] [ 1 ] REMOVE",
                expectedWorkspace: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },

            // === ベクトル操作 - 量指定（1オリジン） ===
            {
                name: "LENGTH - 長さ取得",
                code: "[ 1 2 3 4 5 ] LENGTH",
                expectedWorkspace: [this.createVector([this.createNumber('5')])],
                category: "Vector Operations"
            },
            {
                name: "TAKE - 先頭から取得",
                code: "[ 1 2 3 4 5 ] [ 3 ] TAKE",
                expectedWorkspace: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('2'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },
            {
                name: "TAKE - 負の数で末尾から",
                code: "[ 1 2 3 4 5 ] [ -2 ] TAKE",
                expectedWorkspace: [this.createVector([
                    this.createNumber('4'),
                    this.createNumber('5')
                ])],
                category: "Vector Operations"
            },
            {
                name: "DROP - 先頭から削除",
                code: "[ 1 2 3 4 5 ] [ 2 ] DROP",
                expectedWorkspace: [this.createVector([
                    this.createNumber('3'),
                    this.createNumber('4'),
                    this.createNumber('5')
                ])],
                category: "Vector Operations"
            },
            {
                name: "SPLIT - 分割",
                code: "[ 1 2 3 4 5 6 ] [ 2 ] [ 3 ] [ 1 ] SPLIT",
                expectedWorkspace: [
                    this.createVector([
                        this.createNumber('1'),
                        this.createNumber('2')
                    ]),
                    this.createVector([
                        this.createNumber('3'),
                        this.createNumber('4'),
                        this.createNumber('5')
                    ]),
                    this.createVector([
                        this.createNumber('6')
                    ])
                ],
                category: "Vector Operations"
            },

            // === ベクトル構造操作 ===
            {
                name: "CONCAT - 連結",
                code: "[ 1 2 ] [ 3 4 ] CONCAT",
                expectedWorkspace: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('2'),
                    this.createNumber('3'),
                    this.createNumber('4')
                ])],
                category: "Vector Operations"
            },
            {
                name: "REVERSE - 反転",
                code: "[ 1 2 3 4 ] REVERSE",
                expectedWorkspace: [this.createVector([
                    this.createNumber('4'),
                    this.createNumber('3'),
                    this.createNumber('2'),
                    this.createNumber('1')
                ])],
                category: "Vector Operations"
            },

            // === BigInt対応 ===
            {
                name: "巨大整数作成",
                code: "[ 10000000000000000000000000000000000000000000000000000 ]",
                expectedWorkspace: [this.createVector([
                    this.createNumber('10000000000000000000000000000000000000000000000000000')
                ])],
                category: "BigInt"
            },
            {
                name: "巨大整数の加算",
                code: "[ 9007199254740991 ] [ 9007199254740991 ] +",
                expectedWorkspace: [this.createVector([
                    this.createNumber('18014398509481982')
                ])],
                category: "BigInt"
            },
            {
                name: "巨大分数の計算",
                code: "[ 999999999999999999999/1000000000000000000000 ] [ 1/1000000000000000000000 ] +",
                expectedWorkspace: [this.createVector([
                    this.createNumber('1', '1')
                ])],
                category: "BigInt"
            },

            // === 入出力 ===
            {
                name: "PRINT - 数値出力",
                code: "[ 42 ] PRINT",
                expectedOutput: "[42] ",
                category: "I/O"
            },
            {
                name: "PRINT - 文字列出力",
                code: "[ 'Hello' ] PRINT",
                expectedOutput: "['Hello'] ",
                category: "I/O"
            },

            // === 科学的記数法 ===
            {
                name: "科学的記数法 - 正の指数",
                code: "[ 1.5e3 ]",
                expectedWorkspace: [this.createVector([this.createNumber('1500')])],
                category: "Scientific Notation"
            },
            {
                name: "科学的記数法 - 負の指数",
                code: "[ 2.5e-2 ]",
                expectedWorkspace: [this.createVector([this.createNumber('1', '40')])],
                category: "Scientific Notation"
            },

            // === 複雑な計算 ===
            {
                name: "連続計算",
                code: "[ 2 ] [ 3 ] + [ 4 ] *",
                expectedWorkspace: [this.createVector([this.createNumber('20')])],
                category: "Complex Calculations"
            },
            {
                name: "ベクトルの算術連鎖",
                code: "[ 1 ] [ 2 ] + [ 3 ] + [ 4 ] +",
                expectedWorkspace: [this.createVector([this.createNumber('10')])],
                category: "Complex Calculations"
            },

            // === ネストしたベクトル ===
            {
                name: "ネストしたベクトル",
                code: "[ [ 1 2 ] [ 3 4 ] ]",
                expectedWorkspace: [this.createVector([
                    this.createVector([this.createNumber('1'), this.createNumber('2')]),
                    this.createVector([this.createNumber('3'), this.createNumber('4')])
                ])],
                category: "Nested Vectors"
            },

            // === エラーケース ===
            {
                name: "ゼロ除算エラー",
                code: "[ 5 ] [ 0 ] /",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "範囲外インデックス",
                code: "[ 1 2 3 ] [ 10 ] GET",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "空ベクトルへのアクセス",
                code: "[ ] [ 0 ] GET",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "ワークスペース不足エラー",
                code: "+",
                expectError: true,
                category: "Error Cases"
            }
        ];
    }
}
