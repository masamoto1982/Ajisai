// js/gui/test-ui.ts

import { TestRunner, TestResult } from './test-runner';

export class TestUI {
    private testRunner: TestRunner;
    private testButton!: HTMLButtonElement;
    private testResults!: HTMLElement;

    constructor() {
        this.testRunner = new TestRunner();
    }

    init(): void {
        this.createTestButton();
        this.createTestResultsArea();
    }

    private createTestButton(): void {
        const headerActions = document.querySelector('.header-actions');
        if (headerActions) {
            this.testButton = document.createElement('button');
            this.testButton.textContent = 'Run Tests';
            this.testButton.className = 'reference-btn';
            this.testButton.style.marginLeft = '0.5rem';
            this.testButton.addEventListener('click', () => this.runTests());
            headerActions.appendChild(this.testButton);
        }
    }

    private createTestResultsArea(): void {
        const container = document.querySelector('.container');
        if (container) {
            this.testResults = document.createElement('div');
            this.testResults.id = 'test-results';
            this.testResults.style.display = 'none';
            this.testResults.style.position = 'fixed';
            this.testResults.style.top = '0';
            this.testResults.style.left = '0';
            this.testResults.style.width = '100%';
            this.testResults.style.height = '100%';
            this.testResults.style.backgroundColor = 'rgba(0,0,0,0.8)';
            this.testResults.style.zIndex = '1000';
            this.testResults.style.overflow = 'auto';
            this.testResults.style.padding = '20px';
            
            container.appendChild(this.testResults);
        }
    }

    private async runTests(): Promise<void> {
        this.testButton.disabled = true;
        this.testButton.textContent = 'Running...';

        try {
            const results = await this.testRunner.runAllTests();
            this.displayResults(results);
        } catch (error) {
            console.error('Test execution failed:', error);
        } finally {
            this.testButton.disabled = false;
            this.testButton.textContent = 'Run Tests';
        }
    }

    private displayResults(results: TestResult[]): void {
        const passed = results.filter(r => r.passed).length;
        const total = results.length;
        
        this.testResults.innerHTML = `
            <div style="background: white; margin: 20px auto; max-width: 1000px; padding: 20px; border-radius: 8px;">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                    <h2>テスト結果: ${passed}/${total} 成功</h2>
                    <button onclick="document.getElementById('test-results').style.display='none'">×</button>
                </div>
                <div style="margin-bottom: 20px;">
                    <div style="background: ${passed === total ? '#d4edda' : '#f8d7da'}; padding: 10px; border-radius: 4px; color: ${passed === total ? '#155724' : '#721c24'};">
                        ${passed === total ? '✅ すべてのテストが成功しました！' : `❌ ${total - passed}個のテストが失敗しました`}
                    </div>
                </div>
                <div>
                    ${results.map(result => this.renderTestResult(result)).join('')}
                </div>
            </div>
        `;
        
        this.testResults.style.display = 'block';
    }

    private renderTestResult(result: TestResult): string {
        const status = result.passed ? '✅' : '❌';
        const bgColor = result.passed ? '#d4edda' : '#f8d7da';
        const textColor = result.passed ? '#155724' : '#721c24';
        
        return `
            <details style="margin-bottom: 10px; border: 1px solid #ddd; border-radius: 4px;">
                <summary style="padding: 10px; background: ${bgColor}; color: ${textColor}; cursor: pointer;">
                    ${status} ${result.name} - ${result.description}
                </summary>
                <div style="padding: 10px;">
                    ${result.passed ? 
                        '<p style="color: green;">✅ テスト成功</p>' : 
                        `<p style="color: red;">❌ テスト失敗</p>
                         <p><strong>期待値:</strong> <pre>${JSON.stringify(result.expected, null, 2)}</pre></p>
                         <p><strong>実際の値:</strong> <pre>${JSON.stringify(result.actual, null, 2)}</pre></p>
                         ${result.error ? `<p><strong>エラー:</strong> ${result.error.message}</p>` : ''}`
                    }
                </div>
            </details>
        `;
    }
}
