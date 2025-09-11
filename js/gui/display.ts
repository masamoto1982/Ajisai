// js/gui/display.ts (BigInt対応版)

import type { Value, ExecuteResult, Fraction } from '../wasm-types';

interface DisplayElements {
    outputDisplay: HTMLElement;
    workspaceDisplay: HTMLElement;
}

export class Display {
    private elements!: DisplayElements;

    init(elements: DisplayElements): void {
        this.elements = elements;
        this.elements.outputDisplay.style.whiteSpace = 'pre-wrap';
    }

    showExecutionResult(result: ExecuteResult): void {
        const debugText = (result.debugOutput || '').trim();
        const programOutput = (result.output || '').trim();
        
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

    showError(error: Error | { message?: string } | string): void {
        const errorMessage = typeof error === 'string' ? `Error: ${error}` : `Error: ${error.message || error}`;
        this.elements.outputDisplay.innerHTML = '';
        const errorSpan = document.createElement('span');
        errorSpan.style.color = '#dc3545';
        errorSpan.style.fontWeight = 'bold';
        errorSpan.textContent = errorMessage.replace(/\\n/g, '\n');
        this.elements.outputDisplay.appendChild(errorSpan);
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
            elem.textContent = this.formatValue(item);
            
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

    private formatValue(item: Value): string {
        if (!item) return 'undefined';
        
        switch (item.type) {
            case 'number':
                const frac = item.value as Fraction;
                if (frac.denominator === '1') {
                    return frac.numerator;
                } else {
                    return `${frac.numerator}/${frac.denominator}`;
                }
            case 'string':
                return `'${item.value}'`;
            case 'symbol':
                return String(item.value);
            case 'boolean':
                return item.value ? 'true' : 'false';
            case 'vector':
                if (Array.isArray(item.value)) {
                    const bracketType = item.bracketType || 'square';
                    let openBracket: string, closeBracket: string;
                    
                    switch (bracketType) {
                        case 'curly': openBracket = '{'; closeBracket = '}'; break;
                        case 'round': openBracket = '('; closeBracket = ')'; break;
                        default: openBracket = '['; closeBracket = ']'; break;
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
