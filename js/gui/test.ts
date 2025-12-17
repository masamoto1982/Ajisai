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
            // ============================================
            // Basic Types - 基本型
            // ============================================
            {
                name: "Number - integer",
                code: "[ 42 ]",
                expectedStack: [this.createVector([this.createNumber('42')])],
                category: "Basic Types"
            },
            {
                name: "Number - negative",
                code: "[ -17 ]",
                expectedStack: [this.createVector([this.createNumber('-17')])],
                category: "Basic Types"
            },
            {
                name: "Number - fraction",
                code: "[ 3/4 ]",
                expectedStack: [this.createVector([this.createNumber('3', '4')])],
                category: "Basic Types"
            },
            {
                name: "Number - decimal converts to fraction",
                code: "[ 0.5 ]",
                expectedStack: [this.createVector([this.createNumber('1', '2')])],
                category: "Basic Types"
            },
            {
                name: "String - simple",
                code: "[ 'hello' ]",
                expectedStack: [this.createVector([this.createString('hello')])],
                category: "Basic Types"
            },
            {
                name: "String - with spaces",
                code: "[ 'hello world' ]",
                expectedStack: [this.createVector([this.createString('hello world')])],
                category: "Basic Types"
            },
            {
                name: "Boolean - TRUE",
                code: "[ TRUE ]",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Basic Types"
            },
            {
                name: "Boolean - FALSE",
                code: "[ FALSE ]",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Basic Types"
            },
            {
                name: "NIL",
                code: "[ NIL ]",
                expectedStack: [this.createVector([this.createNil()])],
                category: "Basic Types"
            },

            // ============================================
            // Arithmetic - 算術演算
            // ============================================
            {
                name: "Addition - integers",
                code: "[ 2 ] [ 3 ] +",
                expectedStack: [this.createVector([this.createNumber('5')])],
                category: "Arithmetic"
            },
            {
                name: "Addition - fractions",
                code: "[ 1/2 ] [ 1/3 ] +",
                expectedStack: [this.createVector([this.createNumber('5', '6')])],
                category: "Arithmetic"
            },
            {
                name: "Subtraction",
                code: "[ 10 ] [ 3 ] -",
                expectedStack: [this.createVector([this.createNumber('7')])],
                category: "Arithmetic"
            },
            {
                name: "Multiplication",
                code: "[ 4 ] [ 5 ] *",
                expectedStack: [this.createVector([this.createNumber('20')])],
                category: "Arithmetic"
            },
            {
                name: "Division",
                code: "[ 10 ] [ 4 ] /",
                expectedStack: [this.createVector([this.createNumber('5', '2')])],
                category: "Arithmetic"
            },
            {
                name: "Division by zero - error",
                code: "[ 1 ] [ 0 ] /",
                expectError: true,
                category: "Arithmetic"
            },
            {
                name: "Modulo",
                code: "[ 7 ] [ 3 ] MOD",
                expectedStack: [this.createVector([this.createNumber('1')])],
                category: "Arithmetic"
            },
            {
                name: "Floor",
                code: "[ 7/3 ] FLOOR",
                expectedStack: [this.createVector([this.createNumber('2')])],
                category: "Arithmetic"
            },
            {
                name: "Ceil",
                code: "[ 7/3 ] CEIL",
                expectedStack: [this.createVector([this.createNumber('3')])],
                category: "Arithmetic"
            },
            {
                name: "Round",
                code: "[ 5/2 ] ROUND",
                expectedStack: [this.createVector([this.createNumber('3')])],
                category: "Arithmetic"
            },

            // ============================================
            // Comparison - 比較演算
            // ============================================
            {
                name: "Less than - true",
                code: "[ 3 ] [ 5 ] <",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "Less than - false",
                code: "[ 5 ] [ 3 ] <",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Comparison"
            },
            {
                name: "Greater than",
                code: "[ 5 ] [ 3 ] >",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "Less than or equal",
                code: "[ 3 ] [ 3 ] <=",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "Greater than or equal",
                code: "[ 3 ] [ 3 ] >=",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "Equal - numbers",
                code: "[ 5 ] [ 5 ] =",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },
            {
                name: "Equal - strings",
                code: "[ 'abc' ] [ 'abc' ] =",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Comparison"
            },

            // ============================================
            // Logic - 論理演算
            // ============================================
            {
                name: "AND - true && true",
                code: "[ TRUE ] [ TRUE ] AND",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Logic"
            },
            {
                name: "AND - true && false",
                code: "[ TRUE ] [ FALSE ] AND",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Logic"
            },
            {
                name: "OR - false || true",
                code: "[ FALSE ] [ TRUE ] OR",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Logic"
            },
            {
                name: "NOT - true",
                code: "[ TRUE ] NOT",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Logic"
            },
            {
                name: "NOT - false",
                code: "[ FALSE ] NOT",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Logic"
            },

            // ============================================
            // Vector Operations - ベクタ操作
            // ============================================
            {
                name: "LENGTH",
                code: "[ 1 2 3 4 5 ] LENGTH",
                expectedStack: [
                    this.createVector([this.createNumber('1'), this.createNumber('2'), this.createNumber('3'), this.createNumber('4'), this.createNumber('5')]),
                    this.createVector([this.createNumber('5')])
                ],
                category: "Vector Operations"
            },
            {
                name: "GET - first element",
                code: "[ 10 20 30 ] [ 0 ] GET",
                expectedStack: [
                    this.createVector([this.createNumber('10'), this.createNumber('20'), this.createNumber('30')]),
                    this.createVector([this.createNumber('10')])
                ],
                category: "Vector Operations"
            },
            {
                name: "GET - negative index",
                code: "[ 10 20 30 ] [ -1 ] GET",
                expectedStack: [
                    this.createVector([this.createNumber('10'), this.createNumber('20'), this.createNumber('30')]),
                    this.createVector([this.createNumber('30')])
                ],
                category: "Vector Operations"
            },
            {
                name: "TAKE - positive",
                code: "[ 1 2 3 4 5 ] [ 3 ] TAKE",
                expectedStack: [this.createVector([this.createNumber('1'), this.createNumber('2'), this.createNumber('3')])],
                category: "Vector Operations"
            },
            {
                name: "TAKE - negative",
                code: "[ 1 2 3 4 5 ] [ -2 ] TAKE",
                expectedStack: [this.createVector([this.createNumber('4'), this.createNumber('5')])],
                category: "Vector Operations"
            },
            {
                name: "REVERSE",
                code: "[ 1 2 3 ] REVERSE",
                expectedStack: [this.createVector([this.createNumber('3'), this.createNumber('2'), this.createNumber('1')])],
                category: "Vector Operations"
            },
            {
                name: "CONCAT",
                code: "[ 1 2 ] [ 3 4 ] CONCAT",
                expectedStack: [this.createVector([this.createNumber('1'), this.createNumber('2'), this.createNumber('3'), this.createNumber('4')])],
                category: "Vector Operations"
            },
            {
                name: "INSERT",
                code: "[ 1 3 ] [ 1 ] [ 2 ] INSERT",
                expectedStack: [this.createVector([this.createNumber('1'), this.createNumber('2'), this.createNumber('3')])],
                category: "Vector Operations"
            },
            {
                name: "REPLACE",
                code: "[ 1 2 3 ] [ 1 ] [ 9 ] REPLACE",
                expectedStack: [this.createVector([this.createNumber('1'), this.createNumber('9'), this.createNumber('3')])],
                category: "Vector Operations"
            },
            {
                name: "REMOVE",
                code: "[ 1 2 3 ] [ 1 ] REMOVE",
                expectedStack: [this.createVector([this.createNumber('1'), this.createNumber('3')])],
                category: "Vector Operations"
            },

            // ============================================
            // Tensor Operations - テンソル操作
            // ============================================
            {
                name: "SHAPE - 1D",
                code: "[ 1 2 3 ] SHAPE",
                expectedStack: [
                    this.createVector([this.createNumber('1'), this.createNumber('2'), this.createNumber('3')]),
                    this.createVector([this.createNumber('3')])
                ],
                category: "Tensor Operations"
            },
            {
                name: "SHAPE - 2D",
                code: "[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE",
                expectedStack: [
                    this.createVector([
                        this.createVector([this.createNumber('1'), this.createNumber('2'), this.createNumber('3')]),
                        this.createVector([this.createNumber('4'), this.createNumber('5'), this.createNumber('6')])
                    ]),
                    this.createVector([this.createNumber('2'), this.createNumber('3')])
                ],
                category: "Tensor Operations"
            },
            {
                name: "RANK - 1D",
                code: "[ 1 2 3 ] RANK",
                expectedStack: [
                    this.createVector([this.createNumber('1'), this.createNumber('2'), this.createNumber('3')]),
                    this.createVector([this.createNumber('1')])
                ],
                category: "Tensor Operations"
            },
            {
                name: "RANK - 2D",
                code: "[ [ 1 2 ] [ 3 4 ] ] RANK",
                expectedStack: [
                    this.createVector([
                        this.createVector([this.createNumber('1'), this.createNumber('2')]),
                        this.createVector([this.createNumber('3'), this.createNumber('4')])
                    ]),
                    this.createVector([this.createNumber('2')])
                ],
                category: "Tensor Operations"
            },
            {
                name: "TRANSPOSE",
                code: "[ [ 1 2 3 ] [ 4 5 6 ] ] TRANSPOSE",
                expectedStack: [
                    this.createVector([
                        this.createVector([this.createNumber('1'), this.createNumber('4')]),
                        this.createVector([this.createNumber('2'), this.createNumber('5')]),
                        this.createVector([this.createNumber('3'), this.createNumber('6')])
                    ])
                ],
                category: "Tensor Operations"
            },
            {
                name: "RESHAPE",
                code: "[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE",
                expectedStack: [
                    this.createVector([
                        this.createVector([this.createNumber('1'), this.createNumber('2'), this.createNumber('3')]),
                        this.createVector([this.createNumber('4'), this.createNumber('5'), this.createNumber('6')])
                    ])
                ],
                category: "Tensor Operations"
            },

            // ============================================
            // Broadcasting - ブロードキャスト
            // ============================================
            {
                name: "Broadcast - scalar + vector",
                code: "[ 10 ] [ 1 2 3 ] +",
                expectedStack: [this.createVector([this.createNumber('11'), this.createNumber('12'), this.createNumber('13')])],
                category: "Broadcasting"
            },
            {
                name: "Broadcast - vector * scalar",
                code: "[ 1 2 3 ] [ 2 ] *",
                expectedStack: [this.createVector([this.createNumber('2'), this.createNumber('4'), this.createNumber('6')])],
                category: "Broadcasting"
            },
            {
                name: "Broadcast - vector + vector (same length)",
                code: "[ 1 2 3 ] [ 10 20 30 ] +",
                expectedStack: [this.createVector([this.createNumber('11'), this.createNumber('22'), this.createNumber('33')])],
                category: "Broadcasting"
            },

            // ============================================
            // Higher-Order Functions - 高階関数
            // ============================================
            {
                name: "MAP - double",
                code: "[ ': [ 2 ] *' ] 'DBL' DEF\n[ 1 2 3 ] 'DBL' MAP",
                expectedStack: [this.createVector([this.createNumber('2'), this.createNumber('4'), this.createNumber('6')])],
                category: "Higher-Order Functions"
            },
            {
                name: "FILTER - positive",
                code: "[ ': [ 0 ] >' ] 'POS' DEF\n[ -2 -1 0 1 2 ] 'POS' FILTER",
                expectedStack: [this.createVector([this.createNumber('1'), this.createNumber('2')])],
                category: "Higher-Order Functions"
            },
            {
                name: "FOLD - sum",
                code: "[ 1 2 3 4 ] [ 0 ] '+' FOLD",
                expectedStack: [this.createVector([this.createNumber('10')])],
                category: "Higher-Order Functions"
            },

            // ============================================
            // Type Conversion - 型変換
            // ============================================
            {
                name: "STR - number to string",
                code: "[ 42 ] STR",
                expectedStack: [this.createVector([this.createString('42')])],
                category: "Type Conversion"
            },
            {
                name: "STR - fraction to string",
                code: "[ 3/4 ] STR",
                expectedStack: [this.createVector([this.createString('3/4')])],
                category: "Type Conversion"
            },
            {
                name: "NUM - string to number",
                code: "[ '42' ] NUM",
                expectedStack: [this.createVector([this.createNumber('42')])],
                category: "Type Conversion"
            },
            {
                name: "BOOL - 1 to true",
                code: "[ 1 ] BOOL",
                expectedStack: [this.createVector([this.createBoolean(true)])],
                category: "Type Conversion"
            },
            {
                name: "BOOL - 0 to false",
                code: "[ 0 ] BOOL",
                expectedStack: [this.createVector([this.createBoolean(false)])],
                category: "Type Conversion"
            },

            // ============================================
            // String Operations - 文字列操作
            // ============================================
            {
                name: "CHARS - split string",
                code: "[ 'hello' ] CHARS",
                expectedStack: [this.createVector([
                    this.createString('h'),
                    this.createString('e'),
                    this.createString('l'),
                    this.createString('l'),
                    this.createString('o')
                ])],
                category: "String Operations"
            },
            {
                name: "JOIN - join strings",
                code: "[ 'h' 'e' 'l' 'l' 'o' ] JOIN",
                expectedStack: [this.createVector([this.createString('hello')])],
                category: "String Operations"
            },

            // ============================================
            // Stack Mode (..) - スタックモード
            // ============================================
            {
                name: "Stack mode - LENGTH",
                code: "[ 1 ] [ 2 ] [ 3 ] .. LENGTH",
                expectedStack: [
                    this.createVector([this.createNumber('1')]),
                    this.createVector([this.createNumber('2')]),
                    this.createVector([this.createNumber('3')]),
                    this.createVector([this.createNumber('3')])
                ],
                category: "Stack Mode"
            },
            {
                name: "Stack mode - GET",
                code: "[ 'a' ] [ 'b' ] [ 'c' ] [ 1 ] .. GET",
                expectedStack: [
                    this.createVector([this.createString('a')]),
                    this.createVector([this.createString('b')]),
                    this.createVector([this.createString('c')]),
                    this.createVector([this.createString('b')])
                ],
                category: "Stack Mode"
            },
            {
                name: "Stack mode - REVERSE",
                code: "[ 1 ] [ 2 ] [ 3 ] .. REVERSE",
                expectedStack: [
                    this.createVector([this.createNumber('3')]),
                    this.createVector([this.createNumber('2')]),
                    this.createVector([this.createNumber('1')])
                ],
                category: "Stack Mode"
            },

            // ============================================
            // Custom Word Definition - カスタムワード定義
            // ============================================
            {
                name: "DEF and call",
                code: "[ ': [ 2 ] *' ] 'DOUBLE' DEF\n[ 5 ] DOUBLE",
                expectedStack: [this.createVector([this.createNumber('10')])],
                category: "Custom Words"
            },
            {
                name: "DEF with guard clause",
                code: "[ ': [ 0 ] >\n: [ 1 ]\n: [ 0 ]' ] 'SIGN' DEF\n[ 5 ] SIGN",
                expectedStack: [this.createVector([this.createNumber('1')])],
                category: "Custom Words"
            },
            {
                name: "DEL - delete custom word",
                code: "[ ': [ 2 ] *' ] 'TEMP' DEF\n'TEMP' DEL\nTEMP",
                expectError: true,
                category: "Custom Words"
            },

            // ============================================
            // Control Flow - 制御フロー
            // ============================================
            {
                name: "TIMES - repeat",
                code: "[ ': [ 1 ] +' ] 'INC' DEF\n[ 0 ] 'INC' [ 5 ] TIMES",
                expectedStack: [this.createVector([this.createNumber('5')])],
                category: "Control Flow"
            },

            // ============================================
            // Tensor Generation - テンソル生成
            // ============================================
            {
                name: "FILL",
                code: "[ 3 ] [ 7 ] FILL",
                expectedStack: [this.createVector([
                    this.createNumber('7'),
                    this.createNumber('7'),
                    this.createNumber('7')
                ])],
                category: "Tensor Generation"
            },

            // ============================================
            // Error Cases - エラーケース
            // ============================================
            {
                name: "Error - stack underflow",
                code: "+",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "Error - unknown word",
                code: "UNKNOWNWORD",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "Error - index out of bounds",
                code: "[ 1 2 3 ] [ 10 ] GET",
                expectError: true,
                category: "Error Cases"
            },
            {
                name: "Error - type mismatch in arithmetic",
                code: "[ 'hello' ] [ 1 ] +",
                expectError: true,
                category: "Error Cases"
            }
        ];
    }
}
