// js/gui/display.ts (音声機能追加・Stack対応版 + 段階的実行追加出力対応)

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
            outputSpan.style.color = '#4DC4FF';
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

    appendExecutionResult(result: ExecuteResult): void {
        const programOutput = (result.output || '').trim();
        
        // Process audio commands
        this.processAudioCommands(programOutput);
        
        // Filter out audio commands from displayed output
        const filteredOutput = this.filterAudioCommands(programOutput);
        
        if (filteredOutput) {
            const outputSpan = document.createElement('span');
            outputSpan.style.color = '#4DC4FF';
            outputSpan.textContent = filteredOutput.replace(/\\n/g, '\n');
            this.elements.outputDisplay.appendChild(outputSpan);
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
        span.style.color = '#4DC4FF';
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
                elem.textContent = this.formatValue(item, 0);
            } catch (error) {
                console.error(`Error formatting item ${index}:`, error);
                elem.textContent = 'ERROR';
            }

            container.appendChild(elem);
        });
        
        display.appendChild(container);
    }

    private formatValue(item: Value, depth: number = 0): string { // <--- 修正点: depth引数を追加
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
            case 'datetime': {
                // DateTime型をローカル日時として表示
                if (!item.value || typeof item.value !== 'object') return '@?';
                const frac = item.value as any;
                if (!('numerator' in frac) || !('denominator' in frac)) return '@?';
                return this.formatDateTime(String(frac.numerator), String(frac.denominator));
            }
            case 'tensor': {
                if (!item.value || typeof item.value !== 'object') return '?';
                const tensor = item.value as any;
                if (!('shape' in tensor) || !('data' in tensor)) return '?';

                const shape = tensor.shape as number[];
                const data = tensor.data as any[];

                return this.formatTensor(shape, data, 1);  // スタックエリアが暗黙のdepth=0なのでテンソルは1から開始
            }
            case 'string':
                return `'${item.value}'`;
            case 'symbol':
                return String(item.value);
            case 'boolean':
                return item.value ? 'TRUE' : 'FALSE';
            case 'vector': {
                // depth + 1 でオフセット（スタックが暗黙の深さ0）
                const bracketIndex = (depth + 1) % 3;
                let openBracket: string, closeBracket: string;

                switch (bracketIndex) {
                    case 0: openBracket = '['; closeBracket = ']'; break;
                    case 1: openBracket = '{'; closeBracket = '}'; break;
                    case 2: openBracket = '('; closeBracket = ')'; break;
                    default: openBracket = '['; closeBracket = ']'; break;
                }

                if (Array.isArray(item.value)) {
                    const elements = item.value.map((v: Value) => {
                        // 再帰呼び出し時に depth を +1 する
                        try { return this.formatValue(v, depth + 1); } catch { return '?'; }
                    }).join(' ');
                    
                    return `${openBracket}${elements ? ' ' + elements + ' ' : ''}${closeBracket}`;
                }
                // 空のベクタの場合
                return `${openBracket}${closeBracket}`;
            }
            case 'nil':
                return 'NIL';
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

    /**
     * DateTime型をローカル日時文字列としてフォーマット
     *
     * タイムスタンプ（秒単位、分数でサブ秒を表現）をローカル日時に変換
     * 例: 1732531200500/1000 → "2024-11-25 14:00:00.500"
     */
    private formatDateTime(numerStr: string, denomStr: string): string {
        try {
            // 分数をミリ秒に変換
            const numer = BigInt(numerStr);
            const denom = BigInt(denomStr);

            // ミリ秒単位に変換（タイムスタンプは秒単位なので1000倍）
            const timestampMs = Number((numer * 1000n) / denom);

            const date = new Date(timestampMs);

            // 有効な日付かチェック
            if (isNaN(date.getTime())) {
                return `@${numerStr}${denomStr === '1' ? '' : '/' + denomStr}`;
            }

            // ローカル日時形式でフォーマット
            const year = date.getFullYear();
            const month = String(date.getMonth() + 1).padStart(2, '0');
            const day = String(date.getDate()).padStart(2, '0');
            const hours = String(date.getHours()).padStart(2, '0');
            const minutes = String(date.getMinutes()).padStart(2, '0');
            const seconds = String(date.getSeconds()).padStart(2, '0');
            const ms = date.getMilliseconds();

            // サブ秒があれば表示
            if (ms > 0) {
                const msStr = String(ms).padStart(3, '0');
                return `@${year}-${month}-${day} ${hours}:${minutes}:${seconds}.${msStr}`;
            }

            return `@${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
        } catch {
            // 変換失敗時は生の値を表示
            return `@${numerStr}${denomStr === '1' ? '' : '/' + denomStr}`;
        }
    }

    /**
     * テンソルを階層的にフォーマット
     * 深さに応じて括弧をサイクル: depth % 3 (0: [], 1: {}, 2: ())
     */
    private formatTensor(shape: number[], data: any[], depth: number): string {
        const bracketIndex = depth % 3;
        let openBracket: string, closeBracket: string;

        switch (bracketIndex) {
            case 0: openBracket = '['; closeBracket = ']'; break;
            case 1: openBracket = '{'; closeBracket = '}'; break;
            case 2: openBracket = '('; closeBracket = ')'; break;
            default: openBracket = '['; closeBracket = ']'; break;
        }

        if (shape.length === 0) {
            // スカラー
            if (data.length === 0) return `${openBracket}${closeBracket}`;
            const frac = data[0];
            return `${openBracket} ${this.formatFraction(frac)} ${closeBracket}`;
        }

        if (shape.length === 1) {
            // 1次元：数値を並べる
            if (data.length === 0) return `${openBracket}${closeBracket}`;
            const elements = data.map(frac => this.formatFraction(frac)).join(' ');
            return `${openBracket} ${elements} ${closeBracket}`;
        }

        // 多次元：再帰的に処理
        const outerSize = shape[0] ?? 0;
        const innerShape = shape.slice(1);
        const innerSize = innerShape.reduce((a, b) => a * b, 1);

        const parts: string[] = [];
        for (let i = 0; i < outerSize; i++) {
            const start = i * innerSize;
            const innerData = data.slice(start, start + innerSize);
            parts.push(this.formatTensor(innerShape, innerData, depth + 1));
        }

        return `${openBracket} ${parts.join(' ')} ${closeBracket}`;
    }

    /**
     * 分数をフォーマット
     */
    private formatFraction(frac: any): string {
        if (!frac || !('numerator' in frac) || !('denominator' in frac)) return '?';
        const denomStr = String(frac.denominator);
        const numerStr = String(frac.numerator);
        return this.formatFractionScientific(numerStr, denomStr);
    }
}
