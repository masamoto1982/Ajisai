// js/gui/display.ts - 表示管理

import type { Value, ExecuteResult } from '../wasm-types';
import { AUDIO_ENGINE } from '../audio/audio-engine';
import { formatFractionScientific } from './value-formatter';

interface DisplayElements {
    outputDisplay: HTMLElement;
    stackDisplay: HTMLElement;
}

export class Display {
    private elements!: DisplayElements;
    private mainOutput = '';

    init(elements: DisplayElements): void {
        this.elements = elements;
        this.elements.outputDisplay.style.whiteSpace = 'pre-wrap';
        AUDIO_ENGINE.init().catch(console.error);
    }

    showExecutionResult(result: ExecuteResult): void {
        const debugText = (result.debugOutput || '').trim();
        const programOutput = (result.output || '').trim();

        this.processAudioCommands(programOutput);
        const filteredOutput = this.filterAudioCommands(programOutput);

        this.mainOutput = `${debugText}\n${filteredOutput}`;
        this.elements.outputDisplay.innerHTML = '';

        if (debugText) {
            this.appendSpan(debugText.replace(/\\n/g, '\n'), '#333');
        }

        if (debugText && filteredOutput) {
            this.elements.outputDisplay.appendChild(document.createElement('br'));
        }

        if (filteredOutput) {
            this.appendSpan(filteredOutput.replace(/\\n/g, '\n'), '#4DC4FF');
        }

        if (!debugText && !filteredOutput && result.status === 'OK') {
            this.appendSpan('OK', '#333');
        }
    }

    appendExecutionResult(result: ExecuteResult): void {
        const programOutput = (result.output || '').trim();
        this.processAudioCommands(programOutput);
        const filteredOutput = this.filterAudioCommands(programOutput);

        if (filteredOutput) {
            this.appendSpan(filteredOutput.replace(/\\n/g, '\n'), '#4DC4FF');
        }
    }

    showOutput(text: string): void {
        this.processAudioCommands(text);
        const filteredText = this.filterAudioCommands(text);

        this.mainOutput = filteredText;
        this.elements.outputDisplay.innerHTML = '';
        this.appendSpan(filteredText.replace(/\\n/g, '\n'), '#4DC4FF');
    }

    showError(error: Error | { message?: string } | string): void {
        const errorMessage = typeof error === 'string'
            ? `Error: ${error}`
            : `Error: ${error.message || error}`;

        this.mainOutput = errorMessage;
        this.elements.outputDisplay.innerHTML = '';

        const span = this.appendSpan(errorMessage.replace(/\\n/g, '\n'), '#dc3545');
        span.style.fontWeight = 'bold';
    }

    showInfo(text: string, append = false): void {
        if (append && this.elements.outputDisplay.innerHTML.trim() !== '') {
            this.mainOutput = `${this.mainOutput}\n${text}`;
            this.appendSpan('\n' + text.replace(/\\n/g, '\n'), '#666');
        } else {
            this.mainOutput = text;
            this.elements.outputDisplay.innerHTML = '';
            this.appendSpan(text.replace(/\\n/g, '\n'), '#666');
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
        container.style.cssText = 'display:flex;flex-wrap:wrap-reverse;justify-content:flex-start;align-content:flex-end;flex-direction:row';

        stack.forEach((item, index) => {
            const elem = document.createElement('span');
            elem.className = 'stack-item';
            try {
                elem.textContent = this.formatValue(item, 0);
            } catch {
                console.error(`Error formatting item ${index}`);
                elem.textContent = 'ERROR';
            }
            container.appendChild(elem);
        });

        display.appendChild(container);
    }

    // Audio処理
    private processAudioCommands(output: string): void {
        for (const line of output.split('\n')) {
            if (line.startsWith('AUDIO:')) {
                try {
                    const audioCommand = JSON.parse(line.substring(6));
                    AUDIO_ENGINE.playAudioCommand(audioCommand).catch(console.error);
                } catch {
                    console.error('Failed to parse audio command');
                }
            }
        }
    }

    private filterAudioCommands(output: string): string {
        return output.split('\n').filter(line => !line.startsWith('AUDIO:')).join('\n');
    }

    // DOM操作ヘルパー
    private appendSpan(text: string, color: string): HTMLSpanElement {
        const span = document.createElement('span');
        span.style.color = color;
        span.textContent = text;
        this.elements.outputDisplay.appendChild(span);
        return span;
    }

    // 値フォーマット
    private formatValue(item: Value, depth: number): string {
        if (!item || !item.type) return 'unknown';

        switch (item.type) {
            case 'number':
                return this.formatNumber(item.value);
            case 'datetime':
                return this.formatDateTime(item.value);
            case 'tensor':
                return this.formatTensor(item.value, depth + 1);
            case 'string':
                return `'${item.value}'`;
            case 'symbol':
                return String(item.value);
            case 'boolean':
                return item.value ? 'TRUE' : 'FALSE';
            case 'vector':
                return this.formatVector(item.value, depth);
            case 'nil':
                return 'NIL';
            default:
                return JSON.stringify(item.value);
        }
    }

    private formatNumber(value: any): string {
        if (!value || typeof value !== 'object') return '?';
        if (!('numerator' in value) || !('denominator' in value)) return '?';
        return formatFractionScientific(String(value.numerator), String(value.denominator));
    }

    private formatVector(value: any, depth: number): string {
        const [open, close] = this.getBrackets(depth + 1);

        if (Array.isArray(value)) {
            const elements = value.map((v: Value) => {
                try { return this.formatValue(v, depth + 1); } catch { return '?'; }
            }).join(' ');
            return `${open}${elements ? ' ' + elements + ' ' : ''}${close}`;
        }
        return `${open}${close}`;
    }

    private formatDateTime(value: any): string {
        if (!value || typeof value !== 'object') return '@?';
        if (!('numerator' in value) || !('denominator' in value)) return '@?';

        try {
            const numer = BigInt(value.numerator);
            const denom = BigInt(value.denominator);
            const timestampMs = Number((numer * 1000n) / denom);
            const date = new Date(timestampMs);

            if (isNaN(date.getTime())) {
                return `@${value.numerator}${value.denominator === '1' ? '' : '/' + value.denominator}`;
            }

            const pad = (n: number) => String(n).padStart(2, '0');
            const year = date.getFullYear();
            const month = pad(date.getMonth() + 1);
            const day = pad(date.getDate());
            const hours = pad(date.getHours());
            const minutes = pad(date.getMinutes());
            const seconds = pad(date.getSeconds());
            const ms = date.getMilliseconds();

            const dateStr = `@${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
            return ms > 0 ? `${dateStr}.${String(ms).padStart(3, '0')}` : dateStr;
        } catch {
            return `@${value.numerator}${value.denominator === '1' ? '' : '/' + value.denominator}`;
        }
    }

    private formatTensor(value: any, depth: number): string {
        if (!value || typeof value !== 'object') return '?';
        if (!('shape' in value) || !('data' in value)) return '?';
        return this.formatTensorRecursive(value.shape, value.data, depth);
    }

    private formatTensorRecursive(shape: number[], data: any[], depth: number): string {
        const [open, close] = this.getBrackets(depth);

        if (shape.length === 0) {
            if (data.length === 0) return `${open}${close}`;
            return `${open} ${this.formatFraction(data[0])} ${close}`;
        }

        if (shape.length === 1) {
            if (data.length === 0) return `${open}${close}`;
            const elements = data.map(frac => this.formatFraction(frac)).join(' ');
            return `${open} ${elements} ${close}`;
        }

        const outerSize = shape[0] ?? 0;
        const innerShape = shape.slice(1);
        const innerSize = innerShape.reduce((a, b) => a * b, 1);

        const parts: string[] = [];
        for (let i = 0; i < outerSize; i++) {
            const innerData = data.slice(i * innerSize, (i + 1) * innerSize);
            parts.push(this.formatTensorRecursive(innerShape, innerData, depth + 1));
        }

        return `${open} ${parts.join(' ')} ${close}`;
    }

    private formatFraction(frac: any): string {
        if (!frac || !('numerator' in frac) || !('denominator' in frac)) return '?';
        return formatFractionScientific(String(frac.numerator), String(frac.denominator));
    }

    private getBrackets(depth: number): [string, string] {
        switch (depth % 3) {
            case 0: return ['[', ']'];
            case 1: return ['{', '}'];
            case 2: return ['(', ')'];
            default: return ['[', ']'];
        }
    }
}
