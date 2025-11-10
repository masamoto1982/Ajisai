import type { Value, Fraction } from '../wasm-types';

interface TestCase {
    name: string;
    code: string;
    expectedStack?: Value[];
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

        // 出力エリアをクリア
        this.gui.elements.outputDisplay.innerHTML = '';

        this.showColoredInfo('=== Ajisai Comprehensive Test Suite ===', 'info');
        this.showColoredInfo(`Running ${testCases.length} test cases...`, 'info');

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
            this.showColoredInfo('All tests passed!', 'success');
        }
    }

    private async resetInterpreter(): Promise<void> {
        if (window.ajisaiInterpreter) {
            // リセットを実行するが、出力は保存して復元する
            const currentOutput = this.gui.elements.outputDisplay.innerHTML;
            await window.ajisaiInterpreter.reset();
            // リセットメッセージを除去して元の出力を復元
            this.gui.elements.outputDisplay.innerHTML = currentOutput;
        }
    }
    
    private async runSingleTestWithDetails(testCase: TestCase): Promise<{
        passed: boolean;
        actualStack?: Value[];
        actualOutput?: string;
        errorMessage?: string;
        reason?: string;
    }> {
        // 各テスト前に完全リセット
        await this.resetInterpreter();
        
        // DEFを含む場合、定義と実行を分離
        if (testCase.code.includes(' DEF')) {
            const lines = testCase.code.split('\n');
            
            // 最後のDEF行のインデックスを見つける（後ろから探索）
            let defEndIndex = -1;
            for (let i = lines.length - 1; i >= 0; i--) {
                const line = lines[i];
                if (line && line.trim().includes(' DEF')) {
                    defEndIndex = i;
                    break;
                }
            }
            
            if (defEndIndex >= 0) {
                // DEFまでの部分を実行（定義）
                const defPart = lines.slice(0, defEndIndex + 1).join('\n');
                const defResult = await window.ajisaiInterpreter.execute(defPart);
                
                if (defResult.status === 'ERROR') {
                    return {
                        passed: testCase.expectError === true,
                        errorMessage: defResult.message,
                        reason: 'Error during word definition'
                    };
                }
                
                // DEF後の部分があれば実行
                if (defEndIndex + 1 < lines.length) {
                    const execPart = lines.slice(defEndIndex + 1)
                        .map((line: string) => line.trim())
                        .filter((line: string) => line.length > 0)
                        .join('\n');
                        
                    if (execPart) {
                        const execResult = await window.ajisaiInterpreter.execute(execPart);
                        
                        if (testCase.expectError) {
                            return {
                                passed: execResult.status === 'ERROR',
                                errorMessage: execResult.message,
                                reason: execResult.status === 'ERROR' ? 'Expected error occurred' : 'Expected error but execution succeeded'
                            };
                        }
                        
                        if (execResult.status === 'ERROR') {
                            return {
                                passed: false,
                                errorMessage: execResult.message,
                                reason: 'Unexpected error during execution'
                            };
                        }
                    }
                }
            }
        } else {
            // DEFを含まない通常のテスト
            const result = await window.ajisaiInterpreter.execute(testCase.code);
            
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
        }

        // スタックまたは出力のチェック
        if (testCase.expectedStack) {
            const stack = window.ajisaiInterpreter.get_stack();
            const matches = this.compareStack(stack, testCase.expectedStack);
            return {
                passed: matches,
                actualStack: stack,
                reason: matches ? 'Stack matches expected' : 'Stack mismatch'
            };
        }
        
        if (testCase.expectedOutput) {
            // 出力チェックの場合は再実行が必要
            await this.resetInterpreter();
            const result = await window.ajisaiInterpreter.execute(testCase.code);
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
        
        // 必ず表示されるようにログにも出力
        console.log(`${statusIcon} ${statusText}: ${testCase.name}`);
        
        this.showColoredInfo(`${statusIcon} ${statusText}: ${testCase.name}`, statusColor);
        
        // コードを複数行に分けて表示
        const codeLines = testCase.code.split('\n');
        if (codeLines.length === 1) {
            this.showColoredInfo(`  Code: ${testCase.code}`, 'info');
        } else {
            this.showColoredInfo(`  Code:`, 'info');
            codeLines.forEach((line, index) => {
                this.showColoredInfo(`    Step${index + 1}. ${line}`, 'info');
            });
        }
        
        if (testCase.expectError) {
            this.showColoredInfo(`  Expected: Error should occur`, 'info');
            if (result.errorMessage) {
                this.showColoredInfo(`  Actual error: ${result.errorMessage}`, 'info');
            } else {
                this.showColoredInfo(`  Actual: No error occurred`, passed ? 'info' : 'error');
            }
        } else if (testCase.expectedStack !== undefined) {
            this.showColoredInfo(`  Expected stack: ${this.formatStackForDisplay(testCase.expectedStack)}`, 'info');
            if (result.actualStack !== undefined) {
                this.showColoredInfo(`  Actual stack:   ${this.formatStackForDisplay(result.actualStack)}`, passed ? 'info' : 'error');
                
                // 失敗時には詳細な比較を表示
                if (!passed) {
                    this.showStackDifference(testCase.expectedStack, result.actualStack);
                }
            } else {
                this.showColoredInfo(`  Actual stack: (not captured)`, 'error');
            }
        } else if (testCase.expectedOutput !== undefined) {
            this.showColoredInfo(`  Expected output: "${testCase.expectedOutput}"`, 'info');
            if (result.actualOutput !== undefined) {
                this.showColoredInfo(`  Actual output:   "${result.actualOutput}"`, passed ? 'info' : 'error');
            } else {
                this.showColoredInfo(`  Actual output: (not captured)`, 'error');
            }
        }
        
        if (result.reason) {
            this.showColoredInfo(`  Reason: ${result.reason}`, passed ? 'info' : 'error');
        }

        if (!passed && result.errorMessage) {
            this.showColoredInfo(`  Error: ${result.errorMessage}`, 'error');
        }
        
        this.showColoredInfo('', 'info'); // 空行
    }

    private showStackDifference(expected: Value[], actual: Value[]): void {
        if (expected.length !== actual.length) {
            this.showColoredInfo(`  Stack length mismatch: expected ${expected.length}, got ${actual.length}`, 'error');
        }
        
        const maxLen = Math.max(expected.length, actual.length);
        for (let i = 0; i < maxLen; i++) {
            const exp = i < expected.length ? expected[i] : undefined;
            const act = i < actual.length ? actual[i] : undefined;
            
            if (exp === undefined) {
                this.showColoredInfo(`  [${i}] Extra: ${this.formatValueForDisplay(act!)}`, 'error');
            } else if (act === undefined) {
                this.showColoredInfo(`  [${i}] Missing: ${this.formatValueForDisplay(exp)}`, 'error');
            } else if (!this.compareValue(exp, act)) {
                this.showColoredInfo(`  [${i}] Expected: ${this.formatValueForDisplay(exp)}`, 'error');
                this.showColoredInfo(`  [${i}] Got:      ${this.formatValueForDisplay(act)}`, 'error');
            }
        }
    }
    
    private showTestError(testCase: TestCase, error: any): void {
        this.showColoredInfo(`✗ ERROR: ${testCase.name}`, 'error');
        this.showColoredInfo(`  Code: ${testCase.code}`, 'info');
        this.showColoredInfo(`  Error: ${error}`, 'error');
        this.showColoredInfo('', 'info'); // 空行
    }
    
    private formatStackForDisplay(stack: Value[]): string {
        if (stack.length === 0) {
            return '[]';
        }
        
        const formatted = stack.map(value => this.formatValueForDisplay(value)).join(', ');
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
    
    private compareStack(actual: Value[], expected: Value[]): boolean {
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
                return this.compareStack(actual.value, expected.value);
            
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
                expectedStack: [this.createVector([this.createNumber('42')])],
                category: "Basic Data Types"
            },
            {
                name: "負の整数",
                code: "[ -15 ]",
                expectedStack: [this.createVector([this.createNumber('-15')])],
                category: "Basic Data Types"
            },
            {
                name: "小数",
                code: "[ 3.14 ]",
                expectedStack: [this.createVector([this.createNumber('157', '50')])],
                category: "Basic Data Types"
            },
            {
                name: "分数",
                code: "[ 3/4 ]",
                expectedStack: [this.createVector([this.createNumber('3', '4')])],
                category: "Basic Data Types"
            },
            {
                name: "文字列リテラル",
                code: "[ 'Hello World' ]",
                expectedStack: [this.createVector([this.createString('Hello World')])],
                category: "Basic Data Types"
            },
            {
                name: "真偽値true",
                code: "[ TRUE ]",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Basic Data Types"
            },
            {
                name: "真偽値false",
                code: "[ FALSE ]",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Basic Data Types"
            },
            {
                name: "Nil値",
                code: "[ NIL ]",
                expectedStack: [this.createVector([this.createNil()])],
                category: "Basic Data Types"
            },
            {
                name: "空のベクトル",
                code: "[ ]",
                expectedStack: [this.createVector([])],
                category: "Basic Data Types"
            },
            {
                name: "複数要素のベクトル",
                code: "[ 1 2 3 ]",
                expectedStack: [this.createVector([
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
                expectedStack: [this.createVector([this.createNumber('8')])],
                category: "Arithmetic"
            },
            {
                name: "整数の減算",
                code: "[ 10 ] [ 3 ] -",
                expectedStack: [this.createVector([this.createNumber('7')])],
                category: "Arithmetic"
            },
            {
                name: "整数の乗算",
                code: "[ 4 ] [ 7 ] *",
                expectedStack: [this.createVector([this.createNumber('28')])],
                category: "Arithmetic"
            },
            {
                name: "整数の除算",
                code: "[ 15 ] [ 3 ] /",
                expectedStack: [this.createVector([this.createNumber('5')])],
                category: "Arithmetic"
            },
            {
                name: "分数の加算",
                code: "[ 1/2 ] [ 1/3 ] +",
                expectedStack: [this.createVector([this.createNumber('5', '6')])],
                category: "Arithmetic"
            },
            {
                name: "分数の減算",
                code: "[ 3/4 ] [ 1/4 ] -",
                expectedStack: [this.createVector([this.createNumber('1', '2')])],
                category: "Arithmetic"
            },
            {
                name: "分数の乗算",
                code: "[ 2/3 ] [ 3/4 ] *",
                expectedStack: [this.createVector([this.createNumber('1', '2')])],
                category: "Arithmetic"
            },
            {
                name: "分数の除算",
                code: "[ 2/3 ] [ 1/2 ] /",
                expectedStack: [this.createVector([this.createNumber('4', '3')])],
                category: "Arithmetic"
            },

            // === 比較演算 ===
            {
                name: "等価判定（真）",
                code: "[ 5 ] [ 5 ] =",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "等価判定（偽）",
                code: "[ 5 ] [ 3 ] =",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Comparison"
            },
            {
                name: "より小さい（真）",
                code: "[ 3 ] [ 5 ] <",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "より小さい（偽）",
                code: "[ 5 ] [ 3 ] <",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Comparison"
            },
            {
                name: "以下（真）",
                code: "[ 5 ] [ 5 ] <=",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "より大きい（真）",
                code: "[ 7 ] [ 3 ] >",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "以上（真）",
                code: "[ 5 ] [ 5 ] >=",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },

            // === 論理演算 ===
            {
                name: "論理AND（真）",
                code: "[ TRUE ] [ TRUE ] AND",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Logic"
            },
            {
                name: "論理AND（偽）",
                code: "[ TRUE ] [ FALSE ] AND",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Logic"
            },
            {
                name: "論理OR（真）",
                code: "[ TRUE ] [ FALSE ] OR",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Logic"
            },
            {
                name: "論理OR（偽）",
                code: "[ FALSE ] [ FALSE ] OR",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Logic"
            },
            {
                name: "論理NOT（真→偽）",
                code: "[ TRUE ] NOT",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Logic"
            },
            {
                name: "論理NOT（偽→真）",
                code: "[ FALSE ] NOT",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Logic"
            },

            // === ベクトル操作 - 位置指定（0オリジン） ===
            {
                name: "GET - 正のインデックス",
                code: "[ 10 20 30 ] [ 1 ] GET",
                expectedStack: [this.createVector([this.createNumber('10'), this.createNumber('20'), this.createNumber('30')]), this.createVector([this.createNumber('20')])],
                category: "Vector Operations"
            },
            {
                name: "GET - 負のインデックス",
                code: "[ 10 20 30 ] [ -1 ] GET",
                expectedStack: [this.createVector([this.createNumber('10'), this.createNumber('20'), this.createNumber('30')]), this.createVector([this.createNumber('30')])],
                category: "Vector Operations"
            },
            {
                name: "INSERT - 要素挿入",
                code: "[ 1 3 ] [ 1 ] [ 2 ] INSERT",
                expectedStack: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('2'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },
            {
                name: "INSERT - 負のインデックス（末尾の前）",
                code: "[ 1 2 3 ] [ -1 ] [ 9 ] INSERT",
                expectedStack: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('2'),
                    this.createNumber('9'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },
            {
                name: "INSERT - 負のインデックス（先頭）",
                code: "[ 1 2 3 ] [ -3 ] [ 0 ] INSERT",
                expectedStack: [this.createVector([
                    this.createNumber('0'),
                    this.createNumber('1'),
                    this.createNumber('2'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },
            {
                name: "REPLACE - 要素置換",
                code: "[ 1 2 3 ] [ 1 ] [ 5 ] REPLACE",
                expectedStack: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('5'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },
            {
                name: "REMOVE - 要素削除",
                code: "[ 1 2 3 ] [ 1 ] REMOVE",
                expectedStack: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },

            // === ベクトル操作 - 量指定（1オリジン） ===
            {
                name: "LENGTH - 長さ取得",
                code: "[ 1 2 3 4 5 ] LENGTH",
                expectedStack: [this.createVector([this.createNumber('1'), this.createNumber('2'), this.createNumber('3'), this.createNumber('4'), this.createNumber('5')]), this.createVector([this.createNumber('5')])],
                category: "Vector Operations"
            },
            {
                name: "TAKE - 先頭から取得",
                code: "[ 1 2 3 4 5 ] [ 3 ] TAKE",
                expectedStack: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('2'),
                    this.createNumber('3')
                ])],
                category: "Vector Operations"
            },
            {
                name: "TAKE - 負の数で末尾から",
                code: "[ 1 2 3 4 5 ] [ -2 ] TAKE",
                expectedStack: [this.createVector([
                    this.createNumber('4'),
                    this.createNumber('5')
                ])],
                category: "Vector Operations"
            },
            {
                name: "SPLIT - 分割",
                code: "[ 1 2 3 4 5 6 ] [ 2 ] [ 3 ] [ 1 ] SPLIT",
                expectedStack: [
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
                code: "[ 1 2 ] [ 3 4 ] [ 2 ] CONCAT",
                expectedStack: [this.createVector([
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
                expectedStack: [this.createVector([
                    this.createNumber('4'),
                    this.createNumber('3'),
                    this.createNumber('2'),
                    this.createNumber('1')
                ])],
                category: "Vector Operations"
            },

            // === 条件分岐（ゲート） ===
            {
                name: "ゲート - 単純な条件分岐（真）",
                code: "[ 5 ] [ 5 ] = : [ 10 ] [ 5 ] +",
                expectedStack: [this.createVector([this.createNumber('15')])],
                category: "Conditional Branching"
            },
            {
                name: "ゲート - 単純な条件分岐（偽）",
                code: "[ 5 ] [ 3 ] = : [ 10 ] [ 5 ] +",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Conditional Branching"
            },
            {
                name: "ゲート - 条件分岐の連鎖（正の数）",
                code: "[ '[ 0 ] = : [ 0 ] : [ 0 ] > : [ 1 ] : [ -1 ]' ] 'SIGN' DEF\n[ 5 ] SIGN",
                expectedStack: [this.createVector([this.createNumber('1')])],
                category: "Conditional Branching"
            },
            {
                name: "ゲート - 条件分岐の連鎖（ゼロ）",
                code: "[ '[ 0 ] = : [ 0 ] : [ 0 ] > : [ 1 ] : [ -1 ]' ] 'SIGN' DEF\n[ 0 ] SIGN",
                expectedStack: [this.createVector([this.createNumber('0')])],
                category: "Conditional Branching"
            },
            {
                name: "ゲート - 条件分岐の連鎖（負の数）",
                code: "[ '[ 0 ] = : [ 0 ] : [ 0 ] > : [ 1 ] : [ -1 ]' ] 'SIGN' DEF\n[ -5 ] SIGN",
                expectedStack: [this.createVector([this.createNumber('-1')])],
                category: "Conditional Branching"
            },
            {
                name: "セミコロン - 条件分岐",
                code: "[ 5 ] [ 5 ] = ; [ 100 ]",
                expectedStack: [this.createVector([this.createNumber('100')])],
                category: "Conditional Branching"
            },

            // === カスタムワード定義 ===
            {
                name: "DEF - 最小の定義",
                code: "[ '[ 42 ]' ] 'ANSWER' DEF",
                expectedStack: [],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - 定義したワードの実行",
                code: "[ '[ 42 ]' ] 'ANSWER' DEF\nANSWER",
                expectedStack: [this.createVector([this.createNumber('42')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - 算術演算の定義と実行",
                code: "[ '[ 1 ] [ 2 ] +' ] 'ADD12' DEF\nADD12",
                expectedStack: [this.createVector([this.createNumber('3')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - 説明付き定義と実行",
                code: "[ '[ 2 ] [ 2 ] *' ] 'SQUARE2' '2を二乗する' DEF\nSQUARE2",
                expectedStack: [this.createVector([this.createNumber('4')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - 複数行の定義と実行",
                code: "[ '[ 1 ] [ 2 ] +\n[ 3 ] +' ] 'ADD123' DEF\nADD123",
                expectedStack: [this.createVector([this.createNumber('6')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - 条件付き定義（真の場合）",
                code: "[ '[ 0 ] > : [ 100 ]' ] 'POS_TO_100' DEF\n[ 5 ] POS_TO_100",
                expectedStack: [this.createVector([this.createNumber('100')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - 条件付き定義（偽の場合）",
                code: "[ '[ 0 ] > : [ 100 ]' ] 'POS_TO_100' DEF\n[ -5 ] POS_TO_100",
                expectedStack: [this.createVector([this.createNumber('-5')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - 複数条件の連鎖定義（正）",
                code: "[ '[ 0 ] = : [ 0 ] : [ 0 ] > : [ 1 ] : [ -1 ]' ] 'SIGN' DEF\n[ 5 ] SIGN",
                expectedStack: [this.createVector([this.createNumber('1')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - 複数条件の連鎖定義（ゼロ）",
                code: "[ '[ 0 ] = : [ 0 ] : [ 0 ] > : [ 1 ] : [ -1 ]' ] 'SIGN' DEF\n[ 0 ] SIGN",
                expectedStack: [this.createVector([this.createNumber('0')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - 複数条件の連鎖定義（負）",
                code: "[ '[ 0 ] = : [ 0 ] : [ 0 ] > : [ 1 ] : [ -1 ]' ] 'SIGN' DEF\n[ -3 ] SIGN",
                expectedStack: [this.createVector([this.createNumber('-1')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - デフォルト節のみ",
                code: "[ ': [ 999 ]' ] 'ALWAYS_999' DEF\n[ 123 ] ALWAYS_999",
                expectedStack: [this.createVector([this.createNumber('999')])],
                category: "Custom Word Definition"
            },
            {
                name: "DEF - ワードの再利用",
                code: "[ '[ 2 ] *' ] 'DOUBLE' DEF\n[ 3 ]\nDOUBLE\nDOUBLE",
                expectedStack: [this.createVector([this.createNumber('12')])],
                category: "Custom Word Definition"
            },

            // === TIMES/WAIT制御構造 ===
            {
                name: "TIMES - 基本的な繰り返し",
                code: "[ '[ 1 ] [ 2 ] +' ] 'ADD12' DEF\n'ADD12' [ 3 ] TIMES",
                expectedStack: [
                    this.createVector([this.createNumber('3')]),
                    this.createVector([this.createNumber('3')]),
                    this.createVector([this.createNumber('3')])
                ],
                category: "Control Flow - TIMES/WAIT"
            },
            {
                name: "TIMES - 1回の実行",
                code: "[ '[ 5 ] [ 5 ] *' ] 'SQUARE5' DEF\n'SQUARE5' [ 1 ] TIMES",
                expectedStack: [this.createVector([this.createNumber('25')])],
                category: "Control Flow - TIMES/WAIT"
            },
            {
                name: "WAIT - 基本的な遅延実行",
                code: "[ '[ 100 ]' ] 'HUNDRED' DEF\n'HUNDRED' [ 10 ] WAIT",
                expectedStack: [this.createVector([this.createNumber('100')])],
                category: "Control Flow - TIMES/WAIT"
            },
            {
                name: "TIMES - 組み込みワードでエラー",
                code: "'PRINT' [ 3 ] TIMES",
                expectError: true,
                category: "Control Flow - TIMES/WAIT"
            },
            {
                name: "WAIT - 組み込みワードでエラー",
                code: "'PRINT' [ 100 ] WAIT",
                expectError: true,
                category: "Control Flow - TIMES/WAIT"
            },

            // === ワード管理（DEL） ===
            {
                name: "DEL - ワードの削除",
                code: "[ '[ 42 ]' ] 'TEMP' DEF\n'TEMP' DEL",
                expectedStack: [],
                category: "Word Management"
            },

            // === BigInt対応 ===
            {
                name: "巨大整数作成",
                code: "[ 10000000000000000000000000000000000000000000000000000 ]",
                expectedStack: [this.createVector([
                    this.createNumber('10000000000000000000000000000000000000000000000000000')
                ])],
                category: "BigInt"
            },
            {
                name: "巨大整数の加算",
                code: "[ 9007199254740991 ] [ 9007199254740991 ] +",
                expectedStack: [this.createVector([
                    this.createNumber('18014398509481982')
                ])],
                category: "BigInt"
            },
            {
                name: "巨大分数の計算",
                code: "[ 999999999999999999999/1000000000000000000000 ] [ 1/1000000000000000000000 ] +",
                expectedStack: [this.createVector([
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
                expectedStack: [this.createVector([this.createNumber('1500')])],
                category: "Scientific Notation"
            },
            {
                name: "科学的記数法 - 負の指数",
                code: "[ 2.5e-2 ]",
                expectedStack: [this.createVector([this.createNumber('1', '40')])],
                category: "Scientific Notation"
            },

            // === 複雑な計算 ===
            {
                name: "連続計算",
                code: "[ 2 ] [ 3 ] + [ 4 ] *",
                expectedStack: [this.createVector([this.createNumber('20')])],
                category: "Complex Calculations"
            },
            {
                name: "ベクトルの算術連鎖",
                code: "[ 1 ] [ 2 ] + [ 3 ] + [ 4 ] +",
                expectedStack: [this.createVector([this.createNumber('10')])],
                category: "Complex Calculations"
            },

            // === ネストしたベクトル ===
            {
                name: "ネストしたベクトル",
                code: "[ [ 1 2 ] [ 3 4 ] ]",
                expectedStack: [this.createVector([
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
                name: "スタック不足エラー",
                code: "+",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "未定義ワードのTIMES",
                code: "'UNKNOWN' [ 3 ] TIMES",
                expectError: true,
                category: "Error Cases"
            },

            // === 高階関数 - MAP ===
            {
                name: "MAP - 基本的な使用（2倍）",
                code: "[ '[ 2 ] *' ] 'DOUBLE' DEF\n[ 1 2 3 4 5 ] 'DOUBLE' MAP",
                expectedStack: [this.createVector([
                    this.createNumber('2'),
                    this.createNumber('4'),
                    this.createNumber('6'),
                    this.createNumber('8'),
                    this.createNumber('10')
                ])],
                category: "Higher-Order Functions"
            },
            {
                name: "MAP - 加算処理",
                code: "[ '[ 10 ] +' ] 'ADD10' DEF\n[ 1 2 3 ] 'ADD10' MAP",
                expectedStack: [this.createVector([
                    this.createNumber('11'),
                    this.createNumber('12'),
                    this.createNumber('13')
                ])],
                category: "Higher-Order Functions"
            },
            {
                name: "MAP - 空のベクトル",
                code: "[ '[ 2 ] *' ] 'DOUBLE' DEF\n[ ] 'DOUBLE' MAP",
                expectedStack: [this.createVector([])],
                category: "Higher-Order Functions"
            },
            {
                name: "MAP - 単一要素",
                code: "[ '[ 3 ] *' ] 'TRIPLE' DEF\n[ 5 ] 'TRIPLE' MAP",
                expectedStack: [this.createVector([this.createNumber('15')])],
                category: "Higher-Order Functions"
            },

            // === 高階関数 - FILTER ===
            {
                name: "FILTER - 基本的な使用（5より大きい）",
                code: "[ '[ 5 ] >' ] 'IS-BIG' DEF\n[ 3 7 2 8 1 9 ] 'IS-BIG' FILTER",
                expectedStack: [this.createVector([
                    this.createNumber('7'),
                    this.createNumber('8'),
                    this.createNumber('9')
                ])],
                category: "Higher-Order Functions"
            },
            {
                name: "FILTER - すべて真",
                code: "[ '[ 0 ] >' ] 'IS-POSITIVE' DEF\n[ 1 2 3 4 5 ] 'IS-POSITIVE' FILTER",
                expectedStack: [this.createVector([
                    this.createNumber('1'),
                    this.createNumber('2'),
                    this.createNumber('3'),
                    this.createNumber('4'),
                    this.createNumber('5')
                ])],
                category: "Higher-Order Functions"
            },
            {
                name: "FILTER - すべて偽",
                code: "[ '[ 10 ] >' ] 'IS-BIG' DEF\n[ 1 2 3 ] 'IS-BIG' FILTER",
                expectedStack: [this.createVector([])],
                category: "Higher-Order Functions"
            },
            {
                name: "FILTER - 空のベクトル",
                code: "[ '[ 0 ] >' ] 'IS-POSITIVE' DEF\n[ ] 'IS-POSITIVE' FILTER",
                expectedStack: [this.createVector([])],
                category: "Higher-Order Functions"
            },

            // === 高階関数 - REDUCE ===
            {
                name: "REDUCE - 合計計算",
                code: "[ '+' ] 'ADD' DEF\n[ 1 2 3 4 5 ] [ 0 ] 'ADD' REDUCE",
                expectedStack: [this.createVector([this.createNumber('15')])],
                category: "Higher-Order Functions"
            },
            {
                name: "REDUCE - 積計算",
                code: "[ '*' ] 'MUL' DEF\n[ 1 2 3 4 ] [ 1 ] 'MUL' REDUCE",
                expectedStack: [this.createVector([this.createNumber('24')])],
                category: "Higher-Order Functions"
            },
            {
                name: "REDUCE - 最大値",
                code: "[ '[ 2 ] GET [ 1 ] GET > : [ 2 ] GET : [ 1 ] GET' ] 'MAX2' DEF\n[ 3 7 2 9 1 ] [ 0 ] 'MAX2' REDUCE",
                expectedStack: [this.createVector([this.createNumber('9')])],
                category: "Higher-Order Functions"
            },
            {
                name: "REDUCE - 単一要素",
                code: "[ '+' ] 'ADD' DEF\n[ 5 ] [ 0 ] 'ADD' REDUCE",
                expectedStack: [this.createVector([this.createNumber('5')])],
                category: "Higher-Order Functions"
            },
            {
                name: "REDUCE - 空のベクトル",
                code: "[ '+' ] 'ADD' DEF\n[ ] [ 100 ] 'ADD' REDUCE",
                expectedStack: [this.createVector([this.createNumber('100')])],
                category: "Higher-Order Functions"
            },

            // === 高階関数 - EACH ===
            {
                name: "EACH - 基本的な使用（PRINT）",
                code: "[ 1 2 3 ] 'PRINT' EACH",
                expectedStack: [],
                expectedOutput: "[1] [2] [3] ",
                category: "Higher-Order Functions"
            },
            {
                name: "EACH - カスタムワードで副作用",
                code: "[ 'PRINT' ] 'SHOW' DEF\n[ 10 20 30 ] 'SHOW' EACH",
                expectedStack: [],
                expectedOutput: "[10] [20] [30] ",
                category: "Higher-Order Functions"
            },
            {
                name: "EACH - 空のベクトル",
                code: "[ ] 'PRINT' EACH",
                expectedStack: [],
                category: "Higher-Order Functions"
            },

            // === 高階関数 - 組み合わせ ===
            {
                name: "MAP + FILTER - 2倍してから5より大きいものを選択",
                code: "[ '[ 2 ] *' ] 'DOUBLE' DEF\n[ '[ 5 ] >' ] 'IS-BIG' DEF\n[ 1 2 3 4 5 ] 'DOUBLE' MAP 'IS-BIG' FILTER",
                expectedStack: [this.createVector([
                    this.createNumber('6'),
                    this.createNumber('8'),
                    this.createNumber('10')
                ])],
                category: "Higher-Order Functions"
            },
            {
                name: "FILTER + REDUCE - 正の数だけ合計",
                code: "[ '[ 0 ] >' ] 'IS-POSITIVE' DEF\n[ '+' ] 'ADD' DEF\n[ -2 3 -1 4 5 ] 'IS-POSITIVE' FILTER [ 0 ] 'ADD' REDUCE",
                expectedStack: [this.createVector([this.createNumber('12')])],
                category: "Higher-Order Functions"
            },
            {
                name: "MAP + REDUCE - 全要素を2倍して合計",
                code: "[ '[ 2 ] *' ] 'DOUBLE' DEF\n[ '+' ] 'ADD' DEF\n[ 1 2 3 4 5 ] 'DOUBLE' MAP [ 0 ] 'ADD' REDUCE",
                expectedStack: [this.createVector([this.createNumber('30')])],
                category: "Higher-Order Functions"
            },

            // === 高階関数 - エラーケース ===
            {
                name: "MAP - 未定義ワード",
                code: "[ 1 2 3 ] 'UNDEFINED' MAP",
                expectError: true,
                category: "Higher-Order Functions"
            },
            {
                name: "FILTER - 未定義ワード",
                code: "[ 1 2 3 ] 'UNDEFINED' FILTER",
                expectError: true,
                category: "Higher-Order Functions"
            },
            {
                name: "REDUCE - 未定義ワード",
                code: "[ 1 2 3 ] [ 0 ] 'UNDEFINED' REDUCE",
                expectError: true,
                category: "Higher-Order Functions"
            },
            {
                name: "EACH - 未定義ワード",
                code: "[ 1 2 3 ] 'UNDEFINED' EACH",
                expectError: true,
                category: "Higher-Order Functions"
            },
        ];
    }
}
