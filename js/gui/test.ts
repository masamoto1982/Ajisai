// js/gui/test.ts - テストランナー

import type { Value } from '../wasm-types';
import { TEST_CASES, type TestCase } from './test-cases';
import { formatStack, formatValueSimple, compareStack, compareValue } from './value-formatter';

export class TestRunner {
    private gui: any;

    constructor(gui: any) {
        this.gui = gui;
    }

    async runAllTests(): Promise<void> {
        let totalPassed = 0;
        let totalFailed = 0;
        const failedTests: string[] = [];

        this.gui.elements.outputDisplay.innerHTML = '';
        this.showColoredInfo('=== Ajisai Comprehensive Test Suite ===', 'info');
        this.showColoredInfo(`Running ${TEST_CASES.length} test cases...`, 'info');

        const categories = [...new Set(TEST_CASES.map(t => t.category))].filter(Boolean);

        for (const category of categories) {
            this.showColoredInfo(`\n--- ${category} Tests ---`, 'info');
            const categoryTests = TEST_CASES.filter(t => t.category === category);

            for (const testCase of categoryTests) {
                try {
                    const result = await this.runSingleTest(testCase);
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
            const currentOutput = this.gui.elements.outputDisplay.innerHTML;
            await window.ajisaiInterpreter.reset();
            this.gui.elements.outputDisplay.innerHTML = currentOutput;
        }
    }

    private async runSingleTest(testCase: TestCase): Promise<{
        passed: boolean;
        actualStack?: Value[];
        actualOutput?: string;
        errorMessage?: string;
        reason?: string;
    }> {
        await this.resetInterpreter();

        // DEFを含む場合の処理
        if (testCase.code.includes(' DEF')) {
            return this.executeWithDef(testCase);
        }

        // 通常のテスト
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

        return this.checkExpectations(testCase);
    }

    private async executeWithDef(testCase: TestCase): Promise<{
        passed: boolean;
        actualStack?: Value[];
        errorMessage?: string;
        reason?: string;
    }> {
        const lines = testCase.code.split('\n');

        // 最後のDEF行を探す
        let defEndIndex = -1;
        for (let i = lines.length - 1; i >= 0; i--) {
            if (lines[i]?.trim().includes(' DEF')) {
                defEndIndex = i;
                break;
            }
        }

        if (defEndIndex < 0) {
            return { passed: false, reason: 'DEF not found' };
        }

        // DEFまでの部分を実行
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
                .map(line => line.trim())
                .filter(line => line.length > 0)
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

        return this.checkExpectations(testCase);
    }

    private async checkExpectations(testCase: TestCase): Promise<{
        passed: boolean;
        actualStack?: Value[];
        actualOutput?: string;
        reason?: string;
    }> {
        if (testCase.expectedStack) {
            const stack = window.ajisaiInterpreter.get_stack();
            const matches = compareStack(stack, testCase.expectedStack);
            return {
                passed: matches,
                actualStack: stack,
                reason: matches ? 'Stack matches expected' : 'Stack mismatch'
            };
        }

        if (testCase.expectedOutput) {
            await this.resetInterpreter();
            const result = await window.ajisaiInterpreter.execute(testCase.code);
            const matches = result.output?.trim() === testCase.expectedOutput.trim();
            return {
                passed: matches,
                actualOutput: result.output,
                reason: matches ? 'Output matches expected' : 'Output mismatch'
            };
        }

        return { passed: true, reason: 'Test completed successfully' };
    }

    private showTestResult(testCase: TestCase, result: any, passed: boolean): void {
        const statusIcon = passed ? '[OK]' : '[NG]';
        const statusText = passed ? 'PASS' : 'FAIL';
        const statusColor = passed ? 'success' : 'error';

        console.log(`${statusIcon} ${statusText}: ${testCase.name}`);
        this.showColoredInfo(`${statusIcon} ${statusText}: ${testCase.name}`, statusColor);

        // コードを表示
        const codeLines = testCase.code.split('\n');
        if (codeLines.length === 1) {
            this.showColoredInfo(`  Code: ${testCase.code}`, 'info');
        } else {
            this.showColoredInfo(`  Code:`, 'info');
            codeLines.forEach((line, index) => {
                this.showColoredInfo(`    Step${index + 1}. ${line}`, 'info');
            });
        }

        // 期待値と実際の値を表示
        if (testCase.expectError) {
            this.showColoredInfo(`  Expected: Error should occur`, 'info');
            if (result.errorMessage) {
                this.showColoredInfo(`  Actual error: ${result.errorMessage}`, 'info');
            } else {
                this.showColoredInfo(`  Actual: No error occurred`, passed ? 'info' : 'error');
            }
        } else if (testCase.expectedStack !== undefined) {
            this.showColoredInfo(`  Expected stack: ${formatStack(testCase.expectedStack)}`, 'info');
            if (result.actualStack !== undefined) {
                this.showColoredInfo(`  Actual stack:   ${formatStack(result.actualStack)}`, passed ? 'info' : 'error');
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

        this.showColoredInfo('', 'info');
    }

    private showStackDifference(expected: Value[], actual: Value[]): void {
        if (expected.length !== actual.length) {
            this.showColoredInfo(`  Stack length mismatch: expected ${expected.length}, got ${actual.length}`, 'error');
        }

        const maxLen = Math.max(expected.length, actual.length);
        for (let i = 0; i < maxLen; i++) {
            const exp = expected[i];
            const act = actual[i];

            if (exp === undefined) {
                this.showColoredInfo(`  [${i}] Extra: ${formatValueSimple(act!)}`, 'error');
            } else if (act === undefined) {
                this.showColoredInfo(`  [${i}] Missing: ${formatValueSimple(exp)}`, 'error');
            } else if (!compareValue(exp, act)) {
                this.showColoredInfo(`  [${i}] Expected: ${formatValueSimple(exp)}`, 'error');
                this.showColoredInfo(`  [${i}] Got:      ${formatValueSimple(act)}`, 'error');
            }
        }
    }

    private showTestError(testCase: TestCase, error: any): void {
        this.showColoredInfo(`[NG] ERROR: ${testCase.name}`, 'error');
        this.showColoredInfo(`  Code: ${testCase.code}`, 'info');
        this.showColoredInfo(`  Error: ${error}`, 'error');
        this.showColoredInfo('', 'info');
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
}
