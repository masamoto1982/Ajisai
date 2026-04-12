

import type { Value } from '../wasm-interpreter-types';
import { TEST_CASES, type TestCase } from './gui-interpreter-test-cases';
import { formatStack, formatValueSimple, compareStack, compareValue } from './value-formatter';





export interface TestResult {
    readonly passed: boolean;
    readonly actualStack?: Value[];
    readonly actualOutput?: string;
    readonly errorMessage?: string;
    readonly reason?: string;
}

export interface TestSummary {
    readonly totalPassed: number;
    readonly totalFailed: number;
    readonly failedTests: readonly string[];
}

export interface TestRunnerCallbacks {
    readonly showInfo: (text: string, append: boolean) => void;
    readonly showError: (error: Error | string) => void;
    readonly updateDisplays: () => void;
}

export interface TestRunner {
    readonly runAllTests: () => Promise<TestSummary>;
}

type InfoType = 'success' | 'error' | 'info';








const groupByCategory = (testCases: TestCase[]): Map<string, TestCase[]> => {
    const groups = new Map<string, TestCase[]>();

    testCases.forEach(test => {
        const category = test.category || 'Other';
        const existing = groups.get(category) || [];
        groups.set(category, [...existing, test]);
    });

    return groups;
};




const checkIsTestPassed = (result: TestResult): boolean => result.passed;




const calculateStackDifference = (
    expected: Value[],
    actual: Value[]
): Array<{ index: number; type: 'extra' | 'missing' | 'mismatch'; expected?: Value; actual?: Value }> => {
    const differences: Array<{ index: number; type: 'extra' | 'missing' | 'mismatch'; expected?: Value; actual?: Value }> = [];

    const maxLen = Math.max(expected.length, actual.length);

    for (let i = 0; i < maxLen; i++) {
        const exp = expected[i];
        const act = actual[i];

        if (exp === undefined) {
            differences.push({ index: i, type: 'extra', actual: act });
        } else if (act === undefined) {
            differences.push({ index: i, type: 'missing', expected: exp });
        } else if (!compareValue(exp, act)) {
            differences.push({ index: i, type: 'mismatch', expected: exp, actual: act });
        }
    }

    return differences;
};





export const createTestRunner = (_callbacks: TestRunnerCallbacks): TestRunner => {

    const lookupOutputElement = (): HTMLElement | null =>
        document.getElementById('output-display');


    const showColoredInfo = (text: string, type: InfoType): void => {
        const outputElement = lookupOutputElement();
        if (!outputElement) return;

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
    };


    const resetInterpreter = async (): Promise<void> => {
        if (!window.ajisaiInterpreter) return;

        const outputElement = lookupOutputElement();
        const currentOutput = outputElement?.innerHTML || '';

        await window.ajisaiInterpreter.reset();

        if (outputElement) {
            outputElement.innerHTML = currentOutput;
        }
    };


    const checkExpectations = async (testCase: TestCase): Promise<TestResult> => {
        if (testCase.expectedStack) {
            const stack = window.ajisaiInterpreter.collect_stack();
            const matches = compareStack(stack, testCase.expectedStack);
            return {
                passed: matches,
                actualStack: stack,
                reason: matches ? 'Stack matches expected' : 'Stack mismatch'
            };
        }

        if (testCase.expectedOutput) {
            await resetInterpreter();
            const result = await window.ajisaiInterpreter.execute(testCase.code);
            const matches = result.output?.trim() === testCase.expectedOutput.trim();
            return {
                passed: matches,
                actualOutput: result.output,
                reason: matches ? 'Output matches expected' : 'Output mismatch'
            };
        }

        return { passed: true, reason: 'Test completed successfully' };
    };


    const executeWithDef = async (testCase: TestCase): Promise<TestResult> => {
        const lines = testCase.code.split('\n');


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


        const defPart = lines.slice(0, defEndIndex + 1).join('\n');
        const defResult = await window.ajisaiInterpreter.execute(defPart);

        if (defResult.status === 'ERROR') {
            return {
                passed: testCase.expectError === true,
                errorMessage: defResult.message,
                reason: 'Error during word definition'
            };
        }


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
                        reason: execResult.status === 'ERROR'
                            ? 'Expected error occurred'
                            : 'Expected error but execution succeeded'
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

        return checkExpectations(testCase);
    };


    const runSingleTest = async (testCase: TestCase): Promise<TestResult> => {
        await resetInterpreter();


        if (testCase.code.includes(' DEF')) {
            return executeWithDef(testCase);
        }


        const result = await window.ajisaiInterpreter.execute(testCase.code);

        if (testCase.expectError) {
            return {
                passed: result.status === 'ERROR',
                errorMessage: result.message,
                reason: result.status === 'ERROR'
                    ? 'Expected error occurred'
                    : 'Expected error but execution succeeded'
            };
        }

        if (result.status === 'ERROR') {
            return {
                passed: false,
                errorMessage: result.message,
                reason: 'Unexpected error during execution'
            };
        }

        return checkExpectations(testCase);
    };


    const showStackDifference = (expected: Value[], actual: Value[]): void => {
        if (expected.length !== actual.length) {
            showColoredInfo(
                `  Stack length mismatch → expected ${expected.length}, got ${actual.length}`,
                'error'
            );
        }

        const differences = calculateStackDifference(expected, actual);

        differences.forEach(diff => {
            switch (diff.type) {
                case 'extra':
                    showColoredInfo(`  [${diff.index}] Extra → ${formatValueSimple(diff.actual!)}`, 'error');
                    break;
                case 'missing':
                    showColoredInfo(`  [${diff.index}] Missing → ${formatValueSimple(diff.expected!)}`, 'error');
                    break;
                case 'mismatch':
                    showColoredInfo(`  [${diff.index}] Expected → ${formatValueSimple(diff.expected!)}`, 'error');
                    showColoredInfo(`  [${diff.index}] Got      → ${formatValueSimple(diff.actual!)}`, 'error');
                    break;
            }
        });
    };


    const showTestResult = (testCase: TestCase, result: TestResult, passed: boolean): void => {
        const statusIcon = passed ? '[OK]' : '[NG]';
        const statusText = passed ? 'PASS' : 'FAIL';
        const statusColor: InfoType = passed ? 'success' : 'error';

        console.log(`${statusIcon} ${statusText} → ${testCase.name}`);
        showColoredInfo(`${statusIcon} ${statusText} → ${testCase.name}`, statusColor);


        const codeLines = testCase.code.split('\n');
        if (codeLines.length === 1) {
            showColoredInfo(`  Code → ${testCase.code}`, 'info');
        } else {
            showColoredInfo(`  Code →`, 'info');
            codeLines.forEach((line, index) => {
                showColoredInfo(`    Step${index + 1}. ${line}`, 'info');
            });
        }


        if (testCase.expectError) {
            showColoredInfo(`  Expected → Error should occur`, 'info');
            if (result.errorMessage) {
                showColoredInfo(`  Actual error → ${result.errorMessage}`, 'info');
            } else {
                showColoredInfo(`  Actual → No error occurred`, passed ? 'info' : 'error');
            }
        } else if (testCase.expectedStack !== undefined) {
            showColoredInfo(`  Expected stack → ${formatStack(testCase.expectedStack)}`, 'info');
            if (result.actualStack !== undefined) {
                showColoredInfo(
                    `  Actual stack   → ${formatStack(result.actualStack)}`,
                    passed ? 'info' : 'error'
                );
                if (!passed) {
                    showStackDifference(testCase.expectedStack, result.actualStack);
                }
            } else {
                showColoredInfo(`  Actual stack → (not captured)`, 'error');
            }
        } else if (testCase.expectedOutput !== undefined) {
            showColoredInfo(`  Expected output → "${testCase.expectedOutput}"`, 'info');
            if (result.actualOutput !== undefined) {
                showColoredInfo(
                    `  Actual output   → "${result.actualOutput}"`,
                    passed ? 'info' : 'error'
                );
            } else {
                showColoredInfo(`  Actual output → (not captured)`, 'error');
            }
        }

        if (result.reason) {
            showColoredInfo(`  Reason → ${result.reason}`, passed ? 'info' : 'error');
        }

        if (!passed && result.errorMessage) {
            showColoredInfo(`  Error → ${result.errorMessage}`, 'error');
        }

        showColoredInfo('', 'info');
    };


    const showTestError = (testCase: TestCase, error: unknown): void => {
        showColoredInfo(`[NG] ERROR → ${testCase.name}`, 'error');
        showColoredInfo(`  Code → ${testCase.code}`, 'info');
        showColoredInfo(`  Error → ${error}`, 'error');
        showColoredInfo('', 'info');
    };


    const runAllTests = async (): Promise<TestSummary> => {
        let totalPassed = 0;
        let totalFailed = 0;
        const failedTests: string[] = [];

        const outputElement = lookupOutputElement();
        if (outputElement) {
            outputElement.innerHTML = '';
        }

        showColoredInfo('=== Ajisai Comprehensive Test Suite ===', 'info');
        showColoredInfo(`Running ${TEST_CASES.length} test cases...`, 'info');

        const categoryGroups = groupByCategory(TEST_CASES);
        const categories = [...categoryGroups.keys()].sort();

        for (const category of categories) {
            showColoredInfo(`\n--- ${category} Tests ---`, 'info');
            const categoryTests = categoryGroups.get(category) || [];

            for (const testCase of categoryTests) {
                try {
                    const result = await runSingleTest(testCase);
                    const passed = checkIsTestPassed(result);

                    if (passed) {
                        totalPassed++;
                    } else {
                        totalFailed++;
                        failedTests.push(testCase.name);
                    }

                    showTestResult(testCase, result, passed);
                } catch (error) {
                    totalFailed++;
                    failedTests.push(testCase.name);
                    showTestError(testCase, error);
                }
            }
        }

        showColoredInfo(`\n=== Final Results ===`, 'info');
        showColoredInfo(`Total Passed → ${totalPassed}`, 'success');

        if (totalFailed > 0) {
            showColoredInfo(`Total Failed → ${totalFailed}`, 'error');
            showColoredInfo(`Failed tests → ${failedTests.join(', ')}`, 'error');
        } else {
            showColoredInfo('All tests passed!', 'success');
        }

        return { totalPassed, totalFailed, failedTests };
    };

    return { runAllTests };
};
