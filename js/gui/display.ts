// js/gui/display.ts (BigInt対応・デバッグ版)

import type { Value, ExecuteResult, Fraction } from '../wasm-types';

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
        console.log('updateWorkspace called with:', workspace);
        console.log('Workspace JSON:', JSON.stringify(workspace, null, 2));
        
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
            console.log(`Workspace item ${index}:`, item);
            
            const elem = document.createElement('span');
            elem.className = 'workspace-item';
            
            try {
                elem.textContent = this.formatValue(item);
                console.log(`Formatted item ${index}:`, elem.textContent);
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

    private formatValue(item: Value): string {
        console.log('formatValue input:', item);
        
        if (!item) {
            console.error('formatValue: item is undefined or null');
            return 'undefined';
        }
        
        if (!item.type) {
            console.error('formatValue: item.type is undefined');
            console.log('Item structure:', JSON.stringify(item));
            return 'unknown';
        }
        
        switch (item.type) {
            case 'number':
                console.log('Formatting number:', item.value);
                const frac = item.value as Fraction;
                if (!frac) {
                    console.error('Number value is undefined');
                    return '?';
                }
                // BigIntは文字列として送られてくる
                if (frac.denominator === '1' || frac.denominator === 1) {
                    return String(frac.numerator);
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
                console.log('Formatting vector:', item.value);
                if (Array.isArray(item.value)) {
                    const bracketType = item.bracketType || 'square';
                    let openBracket: string, closeBracket: string;
                    
                    switch (bracketType) {
                        case 'curly': openBracket = '{'; closeBracket = '}'; break;
                        case 'round': openBracket = '('; closeBracket = ')'; break;
                        default: openBracket = '['; closeBracket = ']'; break;
                    }
                    
                    const elements = item.value.map((v: Value) => {
                        try {
                            return this.formatValue(v);
                        } catch (e) {
                            console.error('Error formatting vector element:', e);
                            return '?';
                        }
                    }).join(' ');
                    
                    return `${openBracket}${elements ? ' ' + elements + ' ' : ''}${closeBracket}`;
                }
                console.error('Vector value is not an array:', item.value);
                return '[ ]';
                
            case 'nil':
                return 'nil';
                
            default:
                console.error('Unknown type:', item.type);
                return JSON.stringify(item.value);
        }
    }
}
