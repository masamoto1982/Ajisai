// js/gui/display.ts - 括弧自動変換対応版

import type { Value, ExecuteResult } from '../wasm-types';

interface DisplayElements {
    outputDisplay: HTMLElement;
    workspaceDisplay: HTMLElement;
}

export class Display {
    private elements!: DisplayElements;
    private mainOutput = '';
    private scientificThreshold = 10; // 10桁以上で科学的記数法
    private mantissaPrecision = 6;    // 仮数部の精度

    init(elements: DisplayElements): void {
        this.elements = elements;
        this.elements.outputDisplay.style.whiteSpace = 'pre-wrap';
    }

    showExecutionResult(result: ExecuteResult): void {
        const debugText = (result.debugOutput || '').trim();
        const programOutput = (result.output || '').trim();
        
        this.mainOutput = `${debugText}\n${programOutput}`;
        this.elements.outputDisplay.innerHTML = '';

        if (debugText) {
            const debugSpan = document.createElement('span');
            debugSpan.style.color = '#333';
            debugSpan.textContent = debugText.replace(/\\n/g, '\n');
            this.elements.outputDisplay.appendChild(debugSpan);
        }

        if (debugText && programOutput) {
            this.elements.outputDisplay.appendChild(document.createElement('br'));
        }

        if (programOutput) {
            const outputSpan = document.createElement('span');
            outputSpan.style.color = '#007bff';
            outputSpan.textContent = programOutput.replace(/\\n/g, '\n');
            this.elements.outputDisplay.appendChild(outputSpan);
        }

        if (!debugText && !programOutput && result.status === 'OK') {
            const okSpan = document.createElement('span');
            okSpan.style.color = '#333';
            okSpan.textContent = 'OK';
            this.elements.outputDisplay.appendChild(okSpan);
        }
    }
    
    showOutput(text: string): void {
        this.mainOutput = text;
        this.elements.outputDisplay.innerHTML = '';
        const span = document.createElement('span');
        span.style.color = '#007bff';
        span.textContent = text.replace(/\\n/g, '\n');
        this.elements.outputDisplay.appendChild(span);
    }

    showError(error: Error | { message?: string } | string): void {
        const errorMessage = typeof error === 'string' 
            ? `Error: ${error}`
            : `Error: ${error.message || error}`;
        
        this.mainOutput = errorMessage;
        this.elements.outputDisplay.innerHTML = '';
        
        const errorSpan = document.createElement('span');
        errorSpan.style.color = '#dc3545';
        errorSpan.style.fontWeight = 'bold';
        errorSpan.textContent = errorMessage.replace(/\\n/g, '\n');
        this.elements.outputDisplay.appendChild(errorSpan);
    }

    showInfo(text: string, append = false): void {
        const infoSpan = document.createElement('span');
        infoSpan.style.color = '#666';
        infoSpan.textContent = (append ? '\n' : '') + text.replace(/\\n/g, '\n');

        if (append && this.elements.outputDisplay.innerHTML.trim() !== '') {
            this.mainOutput = `${this.mainOutput}\n${text}`;
            this.elements.outputDisplay.appendChild(infoSpan);
        } else {
            this.mainOutput = text;
            this.elements.outputDisplay.innerHTML = '';
            this.elements.outputDisplay.appendChild(infoSpan);
        }
    }

    updateWorkspace(workspace: Value[]): void {
        const display = this.elements.workspaceDisplay;
        display.innerHTML = '';
        
        if (!Array.isArray(workspace) || workspace.length === 0) {
            display.textContent = '(empty)';
            display.style.color = '#ccc';
            return;
        }
        
        display.style.color = '#333';
        const container = document.createElement('div');
        container.style.display = 'flex';
        container.style.flexWrap = 'wrap-reverse';
        container.style.justifyContent = 'flex-start';
        container.style.alignContent = 'flex-end';
        container.style.flexDirection = 'row';
        
        workspace.forEach((item, index) => {
            const elem = document.createElement('span');
            elem.className = 'workspace-item';
            
            try {
                elem.textContent = this.formatValueWithBrackets(item, 0);
            } catch (error) {
                console.error(`Error formatting item ${index}:`, error);
                elem.textContent = 'ERROR';
            }
            
            if (index === workspace.length - 1) {
                elem.style.fontWeight = 'bold';
                elem.style.backgroundColor = '#4CAF50';
                elem.style.color = 'white';
            } else {
                elem.style.backgroundColor = '#e0e0e0';
                elem.style.color = '#333';
            }
            
            elem.style.margin = '2px 4px';
            elem.style.padding = '2px 6px';
            elem.style.borderRadius = '3px';
            elem.style.fontSize = '0.875rem';
            elem.style.fontFamily = "'Consolas', 'Monaco', monospace";
            
            container.appendChild(elem);
        });
        
        display.appendChild(container);
    }

    private formatValueWithBrackets(item: Value, depth: number): string {
        if (!item) {
            return 'undefined';
        }
        
        if (!item.type) {
            return 'unknown';
        }
        
        switch (item.type) {
            case 'number': {
                if (!item.value || typeof item.value !== 'object') {
                    return '?';
                }
                
                const frac = item.value as any;
                
                if (!('numerator' in frac) || !('denominator' in frac)) {
                    return '?';
                }
                
                const denomStr = String(frac.denominator);
                const numerStr = String(frac.numerator);
                
                return this.formatFractionScientific(numerStr, denomStr);
            }
                
            case 'string':
                return `'${item.value}'`;
                
            case 'symbol':
                return String(item.value);
                
            case 'boolean':
                return item.value ? 'true' : 'false';
                
            case 'vector': {
                if (Array.isArray(item.value)) {
                    const [openBracket, closeBracket] = this.getBracketForDepth(depth);
                    
                    const elements = item.value.map((v: Value) => {
                        try {
                            return this.formatValueWithBrackets(v, depth + 1);
                        } catch (e) {
                            console.error('Error formatting vector element:', e);
                            return '?';
                        }
                    }).join(' ');
                    
                    return `${openBracket}${elements ? ' ' + elements + ' ' : ''}${closeBracket}`;
                }
                return '[ ]';
            }
                
            case 'nil':
                return 'nil';
                
            default:
                return JSON.stringify(item.value);
        }
    }

    private getBracketForDepth(depth: number): [string, string] {
        switch (depth % 3) {
            case 0: return ['[', ']'];  // レベル 0, 3, 6, ...
            case 1: return ['{', '}'];  // レベル 1, 4, 7, ...
            case 2: return ['(', ')'];  // レベル 2, 5, 8, ...
            default: return ['[', ']'];
        }
    }

    private formatValue(item: Value): string {
        // 従来の形式（括弧変換なし）
        return this.formatValueWithBrackets(item, 0);
    }

    private formatFractionScientific(numerStr: string, denomStr: string): string {
        // 整数の場合（分母が1）
        if (denomStr === '1') {
            return this.formatIntegerScientific(numerStr);
        }
        
        // 真の分数の場合
        // 分子と分母それぞれを科学的記数法に変換
        const numSci = this.formatIntegerScientific(numerStr);
        const denSci = this.formatIntegerScientific(denomStr);
        
        // 両方が科学的記数法の場合、指数を計算して一つの科学的記数法にまとめる
        if (numSci.includes('e') && denSci.includes('e')) {
            const numMatch = numSci.match(/^([+-]?\d+\.?\d*)e([+-]?\d+)$/);
            const denMatch = denSci.match(/^([+-]?\d+\.?\d*)e([+-]?\d+)$/);
            
            if (numMatch && denMatch) {
                // 配列要素の存在を確認
                const numMantissaStr = numMatch[1];
                const numExponentStr = numMatch[2];
                const denMantissaStr = denMatch[1];
                const denExponentStr = denMatch[2];
                
                if (numMantissaStr && numExponentStr && denMantissaStr && denExponentStr) {
                    const numMantissa = parseFloat(numMantissaStr);
                    const numExponent = parseInt(numExponentStr);
                    const denMantissa = parseFloat(denMantissaStr);
                    const denExponent = parseInt(denExponentStr);
                    
                    // 仮数部の除算
                    const resultMantissa = numMantissa / denMantissa;
                    // 指数部の減算
                    const resultExponent = numExponent - denExponent;
                    
                    // 仮数部を正規化（1 <= |m| < 10）
                    let normalizedMantissa = resultMantissa;
                    let normalizedExponent = resultExponent;
                    
                    while (Math.abs(normalizedMantissa) >= 10) {
                        normalizedMantissa /= 10;
                        normalizedExponent += 1;
                    }
                    while (Math.abs(normalizedMantissa) < 1 && normalizedMantissa !== 0) {
                        normalizedMantissa *= 10;
                        normalizedExponent -= 1;
                    }
                    
                    // 精度を制限
                    const rounded = normalizedMantissa.toPrecision(this.mantissaPrecision);
                    
                    if (normalizedExponent === 0) {
                        return rounded;
                    } else {
                        return `${rounded}e${normalizedExponent}`;
                    }
                }
            }
        }
        
        // 片方だけが科学的記数法、または両方とも通常の数値の場合
        return `${numSci}/${denSci}`;
    }

    private formatIntegerScientific(numStr: string): string {
        const isNegative = numStr.startsWith('-');
        const absNumStr = isNegative ? numStr.substring(1) : numStr;
        
        // 空文字列チェック
        if (absNumStr.length === 0) {
            return '0';
        }
        
        // 小さい数値はそのまま表示
        if (absNumStr.length < this.scientificThreshold) {
            return numStr;
        }
        
        // 科学的記数法に変換
        const firstDigit = absNumStr[0];
        if (!firstDigit) {
            return '0';
        }
        
        const remainingDigits = absNumStr.substring(1);
        const exponent = remainingDigits.length;
        
        // 仮数部を構成（最初の数桁のみ使用）
        let mantissa: string = firstDigit;
        if (remainingDigits.length > 0) {
            // 小数点以下の桁数を計算
            const fractionalDigits = Math.min(this.mantissaPrecision - 1, remainingDigits.length);
            if (fractionalDigits > 0) {
                mantissa += '.' + remainingDigits.substring(0, fractionalDigits);
            }
        }
        
        // 末尾の0を削除
        mantissa = mantissa.replace(/\.?0+$/, '');
        
        // 符号を付加
        if (isNegative) {
            mantissa = '-' + mantissa;
        }
        
        return `${mantissa}e${exponent}`;
    }
}
