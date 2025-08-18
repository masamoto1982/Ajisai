// js/gui/display.ts

import type { Value } from '../wasm-types';

interface DisplayElements {
    outputDisplay: HTMLElement;
    stackDisplay: HTMLElement;
}

export class Display {
    private elements!: DisplayElements;
    private mainOutput = '';

    init(elements: DisplayElements): void {
        this.elements = elements;
    }

    showOutput(text: string): void {
        this.mainOutput = text;
        this.elements.outputDisplay.textContent = this.mainOutput;
    }

    showError(error: Error | { message?: string } | string): void {
        const errorMessage = typeof error === 'string' 
            ? `Error: ${error}`
            : `Error: ${error.message || error}`;
        this.mainOutput = errorMessage;
        this.elements.outputDisplay.textContent = this.mainOutput;
    }

    showInfo(text: string, append = false): void {
        if (append && this.mainOutput) {
            this.elements.outputDisplay.textContent = `${this.mainOutput}\n${text}`;
        } else {
            this.elements.outputDisplay.textContent = text;
        }
    }

    updateStack(stack: Value[]): void {
        const display = this.elements.stackDisplay;
        display.innerHTML = '';
        
        if (!Array.isArray(stack) || stack.length === 0) {
            display.textContent = '(empty)';
            display.style.color = '#ccc';
            return;
        }
        
        display.style.color = '#333';
        const container = document.createElement('div');
        container.style.display = 'flex';
        container.style.flexWrap = 'wrap-reverse';
        container.style.justifyContent = 'flex-start';
        container.style.alignContent = 'flex-start';
        
        stack.forEach((item, index) => {
            const elem = document.createElement('span');
            elem.className = 'stack-item';
            elem.textContent = this.formatValue(item);
            
            if (index === stack.length - 1) {
                elem.style.fontWeight = 'bold';
            }
            
            elem.style.margin = '2px 4px';
            elem.style.padding = '2px 6px';
            elem.style.backgroundColor = '#e0e0e0';
            elem.style.borderRadius = '3px';
            
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
                return `"${item.value}"`;
            case 'symbol':
                return String(item.value);
            case 'boolean':
                return item.value ? 'true' : 'false';
            case 'vector':
                if (Array.isArray(item.value)) {
                    return `[ ${item.value.map(v => this.formatValue(v)).join(' ')} ]`;
                }
                return '[ ]';
            case 'nil':
                return 'nil';
            case 'quotation':
                return '{ ... }';
            default:
                return JSON.stringify(item.value);
        }
    }
}
