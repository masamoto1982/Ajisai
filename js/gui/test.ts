// js/gui/test.ts (ビルドエラー完全修正版)

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

        this.showColoredInfo('Ajisai BigInt System Tests Starting...', 'info');

        for (const testCase of testCases) {
            try {
                const result = await this.runSingleTest(testCase);
                if (result) {
                    totalPassed++;
                    this.showColoredInfo(`  PASS: ${testCase.name}`, 'success');
                } else {
                    totalFailed++;
                    this.showColoredInfo(`  FAIL: ${testCase.name}`, 'error');
                }
            } catch (error) {
                totalFailed++;
                this.showColoredInfo(`  ERROR: ${testCase.name}: ${error}`, 'error');
            }
        }

        this.showColoredInfo(`\n=== Final Results ===`, 'info');
        this.showColoredInfo(`Total Passed: ${totalPassed}`, 'success');
        
        if (totalFailed > 0) {
            this.showColoredInfo(`Total Failed: ${totalFailed}`, 'error');
        } else {
            this.showColoredInfo('All tests passed!', 'success');
        }
    }
    
    private async runSingleTest(testCase: TestCase): Promise<boolean> {
        window.ajisaiInterpreter.reset();
        const result = window.ajisaiInterpreter.execute(testCase.code);
        
        if (testCase.expectError) return result.status === 'ERROR';
        if (result.status === 'ERROR') return false;

        if (testCase.expectedWorkspace) {
            const workspace = window.ajisaiInterpreter.get_workspace();
            return this.compareWorkspace(workspace, testCase.expectedWorkspace);
        }
        if (testCase.expectedOutput) return result.output?.trim() === testCase.expectedOutput.trim();
        
        return true;
    }
    
    private compareWorkspace(actual: Value[], expected: Value[]): boolean {
        if (actual.length !== expected.length) return false;
        for (let i = 0; i < actual.length; i++) {
            const actualItem = actual[i];
            const expectedItem = expected[i];
            if (actualItem === undefined || expectedItem === undefined) return false;
            if (!this.compareValue(actualItem, expectedItem)) return false;
        }
        return true;
    }

    private compareValue(actual: Value, expected: Value): boolean {
        if (actual.type !== expected.type) return false;

        switch (actual.type) {
            case 'number':
                const actualFrac = actual.value as Fraction;
                const expectedFrac = expected.value as Fraction;
                return actualFrac.numerator === expectedFrac.numerator && 
                       actualFrac.denominator === expectedFrac.denominator;
            case 'vector':
                return this.compareWorkspace(actual.value, expected.value);
            default:
                return JSON.stringify(actual.value) === JSON.stringify(expected.value);
        }
    }
    
    private showColoredInfo(text: string, type: 'success' | 'error' | 'info'): void {
        const outputElement = this.gui.elements.outputDisplay;
        const span = document.createElement('span');
        span.textContent = text + '\n';
        switch (type) {
            case 'success': span.style.color = '#28a745'; break;
            case 'error': span.style.color = '#dc3545'; break;
            case 'info': span.style.color = '#333'; break;
        }
        outputElement.appendChild(span);
    }
    
    private getTestCases(): TestCase[] {
        return [
            {
                name: "巨大な整数の作成",
                code: "[ 10000000000000000000000000000000000000000000000000000 ]",
                expectedWorkspace: [{
                    type: 'vector', value: [{ 
                        type: 'number', value: { 
                            numerator: '10000000000000000000000000000000000000000000000000000', 
                            denominator: '1' 
                        } 
                    }]
                }],
                category: "BigInt"
            },
            {
                name: "巨大な整数の足し算",
                code: "[ 9007199254740991 ] [ 9007199254740991 ] +",
                expectedWorkspace: [{
                    type: 'vector', value: [{
                        type: 'number', value: {
                            numerator: '18014398509481982',
                            denominator: '1'
                        }
                    }]
                }],
                category: "BigInt"
            },
        ];
    }
}
