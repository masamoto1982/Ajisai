// js/gui/display.ts

import type { Value } from '../wasm-types';

interface DisplayElements {
    outputDisplay: HTMLElement;
    workspaceDisplay: HTMLElement;
}

export class Display {
    private elements!: DisplayElements;
    private mainOutput = '';

    init(elements: DisplayElements): void {
        this.elements = elements;
        this.elements.outputDisplay.style.whiteSpace = 'pre-wrap';
    }

    showOutput(text: string): void {
        this.mainOutput = text;
        this.elements.outputDisplay.innerHTML = ''; // innerHTMLに変更してスタイル適用を可能に
        
        const span = document.createElement('span');
        span.style.color = '#333';
        span.textContent = text.replace(/\\n/g, '\n');
        this.elements.outputDisplay.appendChild(span);
    }

    showError(error: Error | { message?: string } | string): void {
        const errorMessage = typeof error === 'string' 
            ? `Error: ${error}`
            : `Error: ${error.message || error}`;
        
        this.mainOutput = errorMessage;
        this.elements.outputDisplay.innerHTML = ''; // 既存内容をクリア
        
        // エラーメッセージを赤字で表示
        const errorSpan = document.createElement('span');
        errorSpan.style.color = '#dc3545';  // Bootstrap の danger 色
        errorSpan.style.fontWeight = 'bold';
        errorSpan.textContent = errorMessage.replace(/\\n/g, '\n');
        this.elements.outputDisplay.appendChild(errorSpan);
    }

    showInfo(text: string, append = false): void {
        if (append && this.mainOutput) {
            this.mainOutput = `${this.mainOutput}\n${text}`;
            
            // 既存の内容に追加
            const infoSpan = document.createElement('span');
            infoSpan.style.color = '#666';
            infoSpan.textContent = '\n' + text.replace(/\\n/g, '\n');
            this.elements.outputDisplay.appendChild(infoSpan);
        } else {
            this.mainOutput = text;
            this.elements.outputDisplay.innerHTML = '';
            
            const infoSpan = document.createElement('span');
            infoSpan.style.color = '#666';
            infoSpan.textContent = text.replace(/\\n/g, '\n');
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
    container.style.flexWrap = 'wrap-reverse';  // 上向きに折り返し
    container.style.justifyContent = 'flex-start';  // 左詰め
    container.style.alignContent = 'flex-end';  // 下揃え（wrap-reverseと組み合わせて上に積み上げ）
    container.style.flexDirection = 'row';  // 左から右に並ぶ
    
    // スタックの順序そのままで表示（ボトムから順番に配置、トップが右上に）
    workspace.forEach((item, index) => {
        const elem = document.createElement('span');
        elem.className = 'workspace-item';
        elem.textContent = this.formatValue(item);
        
        // スタックトップ（最後の要素）を目立たせる
        if (index === workspace.length - 1) {
            elem.style.fontWeight = 'bold';
            elem.style.backgroundColor = '#4CAF50';  // 明るい緑色でスタックトップを強調
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

    private formatValue(item: Value): string {
    if (!item) return 'undefined';
    
    switch (item.type) {
        case 'number':
            if (typeof item.value === 'object' && item.value !== null && 'numerator' in item.value && 'denominator' in item.value) {
                const frac = item.value as { numerator: number; denominator: number };
                if (frac.denominator === 1) {
                    return frac.numerator.toString();
                } else {
                    return `${frac.numerator}/${frac.denominator}`;
                }
            }
            return typeof item.value === 'string' ? item.value : String(item.value);
        case 'string':
            return `'${item.value}'`; // シングルクォートに変更
        case 'symbol':
            return String(item.value);
        case 'boolean':
            return item.value ? 'true' : 'false';
        case 'vector':
            if (Array.isArray(item.value)) {
                // 括弧タイプに応じて適切な括弧を使用
                const bracketType = (item as any).bracketType || 'square';
                let openBracket: string, closeBracket: string;
                
                switch (bracketType) {
                    case 'curly':
                        openBracket = '{';
                        closeBracket = '}';
                        break;
                    case 'round':
                        openBracket = '(';
                        closeBracket = ')';
                        break;
                    case 'square':
                    default:
                        openBracket = '[';
                        closeBracket = ']';
                        break;
                }
                
                return `${openBracket} ${item.value.map(v => this.formatValue(v)).join(' ')} ${closeBracket}`;
            }
            return '[ ]';
        case 'nil':
            return 'nil';
        default:
            return JSON.stringify(item.value);
    }
}
}
