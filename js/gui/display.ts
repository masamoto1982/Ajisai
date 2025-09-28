// js/gui/display.ts (音声機能追加・Stack対応版)

import type { Value, ExecuteResult } from '../wasm-types';
import { AUDIO_ENGINE } from '../audio/audio-engine';

interface DisplayElements {
    outputDisplay: HTMLElement;
    stackDisplay: HTMLElement;
}

export class Display {
    private elements!: DisplayElements;
    private mainOutput = '';
    private scientificThreshold = 10; // 10桁以上で科学的記数法
    private mantissaPrecision = 6;    // 仮数部の精度

    init(elements: DisplayElements): void {
        this.elements = elements;
        this.elements.outputDisplay.style.whiteSpace = 'pre-wrap';
        
        // Audio engine initialization
        AUDIO_ENGINE.init().catch(console.error);
    }

    showExecutionResult(result: ExecuteResult): void {
        const debugText = (result.debugOutput || '').trim();
        const programOutput = (result.output || '').trim();
        
        // Process audio commands
        this.processAudioCommands(programOutput);
        
        // Filter out audio commands from displayed output
        const filteredOutput = this.filterAudioCommands(programOutput);
        
        this.mainOutput = `${debugText}\n${filteredOutput}`;
        this.elements.outputDisplay.innerHTML = '';

        if (debugText) {
            const debugSpan = document.createElement('span');
            debugSpan.style.color = '#333';
            debugSpan.textContent = debugText.replace(/\\n/g, '\n');
            this.elements.outputDisplay.appendChild(debugSpan);
        }

        if (debugText && filteredOutput) {
            this.elements.outputDisplay.appendChild(document.createElement('br'));
        }

        if (filteredOutput) {
            const outputSpan = document.createElement('span');
            outputSpan.style.color = '#007bff';
            outputSpan.textContent = filteredOutput.replace(/\\n/g, '\n');
            this.elements.outputDisplay.appendChild(outputSpan);
        }

        if (!debugText && !filteredOutput && result.status === 'OK') {
            const okSpan = document.createElement('span');
            okSpan.style.color = '#333';
            okSpan.textContent = 'OK';
            this.elements.outputDisplay.appendChild(okSpan);
        }
    }

    private processAudioCommands(output: string): void {
        const lines = output.split('\n');
        
        for (const line of lines) {
            if (line.startsWith('AUDIO:')) {
                const audioJson = line.substring(6);
                try {
                    const audioCommand = JSON.parse(audioJson);
                    AUDIO_ENGINE.playAudioCommand(audioCommand).catch(console.error);
                } catch (error) {
                    console.error('Failed to parse audio command:', error);
                }
            }
        }
    }

    private filterAudioCommands(output: string): string {
        const lines = output.split('\n');
        const filteredLines = lines.filter(line => !line.startsWith('AUDIO:'));
        return filteredLines.join('\n');
    }
    
    showOutput(text: string): void {
        this.processAudioCommands(text);
        const filteredText = this.filterAudioCommands(text);
        
        this.mainOutput = filteredText;
        this.elements.outputDisplay.innerHTML = '';
        const span = document.createElement('span');
        span.style.color = '#007bff';
        span.textContent = filteredText.replace(/\\n/g, '\n');
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
        container.style.alignContent = 'flex-end';
        container.style.flexDirection = 'row';
        
        stack.forEach((item, index) => {
            const elem = document.createElement('span');
            elem.className = 'stack-item';
            
            try {
                elem.textContent = this.formatValue(item);
            } catch (error) {
                console.error(`Error formatting item ${index}:`, error);
                elem.textContent = 'ERROR';
            }
            
            if (index === stack.length - 1) {
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
        if (!item || !item.type) {
            return 'unknown';
        }
        
        switch (item.type) {
            case 'number': {
                if (!item.value || typeof item.value !== 'object') return '?';
                const frac = item.value as any;
                if (!('numerator' in frac) || !('denominator' in frac)) return '?';
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
                    const bracketType = item.bracketType || 'square';
                    let openBracket: string, closeBracket: string;
                    
                    switch (bracketType) {
                        case 'curly': openBracket = '{'; closeBracket = '}'; break;
                        case 'round': openBracket = '('; closeBracket = ')'; break;
                        default: openBracket = '['; closeBracket = ']'; break;
                    }
                    
                    const elements = item.value.map((v: Value) => {
                        try { return this.formatValue(v); } catch { return '?'; }
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

    private formatFractionScientific(numerStr: string, denomStr: string): string {
        if (denomStr === '1') {
            return this.formatIntegerScientific(numerStr);
        }
        
        const numSci = this.formatIntegerScientific(numerStr);
        const denSci = this.formatIntegerScientific(denomStr);
        
        if (numSci.includes('e') && denSci.includes('e')) {
            const numMatch = numSci.match(/^([+-]?\d+\.?\d*)e([+-]?\d+)$/);
            const denMatch = denSci.match(/^([+-]?\d+\.?\d*)e([+-]?\d+)$/);
            
            if (numMatch && denMatch) {
                const numMantissa = parseFloat(numMatch[1]!);
                const numExponent = parseInt(numMatch[2]!);
                const denMantissa = parseFloat(denMatch[1]!);
                const denExponent = parseInt(denMatch[2]!);
                
                let resultMantissa = numMantissa / denMantissa;
                let resultExponent = numExponent - denExponent;
                
                while (Math.abs(resultMantissa) >= 10) {
                    resultMantissa /= 10;
                    resultExponent += 1;
                }
                while (Math.abs(resultMantissa) < 1 && resultMantissa !== 0) {
                    resultMantissa *= 10;
                    resultExponent -= 1;
                }
                
                const rounded = resultMantissa.toPrecision(this.mantissaPrecision);
                return resultExponent === 0 ? rounded : `${rounded}e${resultExponent}`;
            }
        }
        
        return `${numSci}/${denSci}`;
    }

    private formatIntegerScientific(numStr: string): string {
        const isNegative = numStr.startsWith('-');
        const absNumStr = isNegative ? numStr.substring(1) : numStr;
        
        if (absNumStr.length < this.scientificThreshold) {
            return numStr;
        }
        
        const firstDigit = absNumStr[0];
        const remainingDigits = absNumStr.substring(1);
        const exponent = remainingDigits.length;
        
        let mantissa = firstDigit!;
        if (remainingDigits.length > 0) {
            const fractionalDigits = Math.min(this.mantissaPrecision - 1, remainingDigits.length);
            if (fractionalDigits > 0) {
                mantissa += '.' + remainingDigits.substring(0, fractionalDigits);
            }
        }
        
        mantissa = mantissa.replace(/\.?0+$/, '');
        if (isNegative) mantissa = '-' + mantissa;
        
        return `${mantissa}e${exponent}`;
    }
}
