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
            this.testResults.style.boxSizing = 'border-box';
            
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
        const failedResults = results.filter(r => !r.passed);
        
        this.testResults.innerHTML = `
            <div style="background: white; margin: 20px auto; max-width: 1200px; padding: 20px; border-radius: 8px; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">
                <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                    <h2 style="margin: 0;">テスト結果: ${passed}/${total} 成功</h2>
                    <button onclick="document.getElementById('test-results').style.display='none'" style="background: #f0f0f0; border: 1px solid #ccc; padding: 8px 12px; border-radius: 4px; cursor: pointer; font-size: 16px;">閉じる</button>
                </div>
                
                <div style="margin-bottom: 20px;">
                    <div style="background: ${passed === total ? '#d4edda' : '#f8d7da'}; padding: 15px; border-radius: 6px; color: ${passed === total ? '#155724' : '#721c24'}; border: 1px solid ${passed === total ? '#c3e6cb' : '#f5c6cb'};">
                        ${passed === total ? 
                            '✅ すべてのテストが成功しました！Ajisaiの各機能が正常に動作しています。' : 
                            `❌ ${total - passed}個のテストが失敗しました。詳細を確認してください。`
                        }
                    </div>
                </div>

                ${failedResults.length > 0 ? `
                    <div style="margin-bottom: 20px;">
                        <h3 style="color: #dc3545; margin-bottom: 10px;">🚨 失敗したテスト</h3>
                        ${failedResults.map(result => this.renderTestResult(result, true)).join('')}
                    </div>
                ` : ''}

                <details ${passed < total ? '' : 'open'} style="margin-bottom: 20px;">
                    <summary style="padding: 10px; background: #f8f9fa; border: 1px solid #dee2e6; border-radius: 4px; cursor: pointer; font-weight: bold;">
                        📋 全テスト結果 (${total}件)
                    </summary>
                    <div style="margin-top: 10px;">
                        ${results.map(result => this.renderTestResult(result, false)).join('')}
                    </div>
                </details>
            </div>
        `;
        
        this.testResults.style.display = 'block';
    }

    private renderTestResult(result: TestResult, isFailureSection: boolean): string {
        const status = result.passed ? '✅' : '❌';
        const bgColor = result.passed ? '#d4edda' : '#f8d7da';
        const textColor = result.passed ? '#155724' : '#721c24';
        const borderColor = result.passed ? '#c3e6cb' : '#f5c6cb';
        
        return `
            <details style="margin-bottom: 8px; border: 1px solid ${borderColor}; border-radius: 6px; ${isFailureSection ? 'background: #fff5f5;' : ''}">
                <summary style="padding: 12px; background: ${bgColor}; color: ${textColor}; cursor: pointer; font-weight: 500;">
                    ${status} <strong>${result.name}</strong> - ${result.description}
                </summary>
                <div style="padding: 15px; background: white;">
                    <div style="margin-bottom: 10px;">
                        <strong>🔤 テストコード:</strong>
                        <code style="background: #f8f9fa; padding: 4px 8px; border-radius: 4px; font-family: 'Consolas', 'Monaco', monospace; border: 1px solid #e9ecef;">${result.code}</code>
                    </div>
                    
                    ${result.passed ? 
                        `<div style="color: #155724; padding: 10px; background: #d4edda; border-radius: 4px; border: 1px solid #c3e6cb;">
                            ✅ <strong>テスト成功</strong><br>
                            期待した結果が得られました: <span style="font-family: 'Consolas', 'Monaco', monospace;">${result.actual}</span>
                        </div>` : 
                        `<div style="color: #721c24; padding: 10px; background: #f8d7da; border-radius: 4px; border: 1px solid #f5c6cb;">
                            ❌ <strong>テスト失敗</strong><br><br>
                            <div style="margin-bottom: 8px;">
                                <strong>🎯 期待値:</strong><br>
                                <span style="font-family: 'Consolas', 'Monaco', monospace; background: white; padding: 4px 8px; border-radius: 3px; border: 1px solid #dee2e6;">${result.expected}</span>
                            </div>
                            <div style="margin-bottom: 8px;">
                                <strong>📊 実際の値:</strong><br>
                                <span style="font-family: 'Consolas', 'Monaco', monospace; background: white; padding: 4px 8px; border-radius: 3px; border: 1px solid #dee2e6;">${result.actual}</span>
                            </div>
                            ${result.error ? `<div><strong>⚠️ エラー詳細:</strong><br><span style="font-family: 'Consolas', 'Monaco', monospace;">${result.error.message}</span></div>` : ''}
                        </div>`
                    }
                </div>
            </details>
        `;
    }
}
