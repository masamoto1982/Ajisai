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

    // 組み込みワードの構文ヒント（簡単な例）
    private readonly syntaxHints: Record<string, string> = {
        // 位置指定操作
        'GET': '[ 10 20 30 ] [ 0 ] GET → [ 10 20 30 ] [ 10 ]',
        'INSERT': '[ 1 3 ] [ 1 ] [ 2 ] INSERT → [ 1 2 3 ]',
        'REPLACE': '[ 1 2 3 ] [ 0 ] [ 9 ] REPLACE → [ 9 2 3 ]',
        'REMOVE': '[ 1 2 3 ] [ 0 ] REMOVE → [ 2 3 ]',

        // 量指定操作
        'LENGTH': '[ 1 2 3 4 5 ] LENGTH → [ 1 2 3 4 5 ] [ 5 ]',
        'TAKE': '[ 1 2 3 4 5 ] [ 3 ] TAKE → [ 1 2 3 ]',

        // Vector構造操作
        'SPLIT': '[ 1 2 3 4 5 6 ] [ 2 ] [ 3 ] SPLIT → [ 1 2 ] [ 3 4 5 ] [ 6 ]',
        'CONCAT': '[ a ] [ b ] CONCAT → [ a b ]',
        'REVERSE': '[ a b c ] REVERSE → [ c b a ]',
        'LEVEL': '[ [ a b ] [ c ] ] LEVEL → [ a b c ]',

        // 算術演算
        '+': '[ 1 2 3 ] [ 4 5 6 ] + → [ 5 7 9 ]',
        '-': '[ 5 7 9 ] [ 1 2 3 ] - → [ 4 5 6 ]',
        '*': '[ 1 2 3 ] [ 4 5 6 ] * → [ 4 10 18 ]',
        '/': '[ 10 20 30 ] [ 2 4 5 ] / → [ 5 5 6 ]',

        // 比較演算
        '=': '[ 3 ] [ 3 ] = → [ true ]',
        '<': '[ 1 2 3 ] [ 2 2 2 ] < → [ true false false ]',
        '<=': '[ 1 2 3 ] [ 2 2 2 ] <= → [ true true false ]',
        '>': '[ 3 2 1 ] [ 2 2 2 ] > → [ true false false ]',
        '>=': '[ 3 2 1 ] [ 2 2 2 ] >= → [ true true false ]',

        // 論理演算
        'AND': '[ true true false ] [ true false true ] AND → [ true false false ]',
        'OR': '[ true true false ] [ true false true ] OR → [ true true true ]',
        'NOT': '[ true false true ] NOT → [ false true false ]',

        // 制御構造
        ':': '[ 5 ] [ 0 ] > : \'positive\' : \'negative or zero\'',

        // 高階関数
        'MAP': '[ 1 2 3 ] \'[ 2 ] *\' MAP → [ 2 4 6 ]',
        'FILTER': '[ 1 2 3 4 5 ] \'[ 3 ] >\' FILTER → [ 4 5 ]',

        // 入出力
        'PRINT': '[ 42 ] PRINT → 出力: 42',

        // ワード管理
        'DEF': '[ \'[ 2 ] *\' ] \'DOUBLE\' DEF',
        'DEL': '\'DOUBLE\' DEL',
        '?': '\'GET\' ? → 詳細説明を表示',

        // 制御フロー
        'TIMES': '\'[ 1 ] +\' [ 5 ] TIMES → 5回実行',
        'WAIT': '\'PRINT\' [ 1000 ] WAIT → 1秒後に実行',

        // モード指定
        'STACK': 'a b c [ 1 ] STACK GET → スタック全体から取得',
        'STACKTOP': '[ 1 2 3 ] [ 0 ] GET → スタックトップのベクタから取得'
    }

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

    appendExecutionResult(result: ExecuteResult): void {
        const programOutput = (result.output || '').trim();
        
        // Process audio commands
        this.processAudioCommands(programOutput);
        
        // Filter out audio commands from displayed output
        const filteredOutput = this.filterAudioCommands(programOutput);
        
        if (filteredOutput) {
            const outputSpan = document.createElement('span');
            outputSpan.style.color = '#007bff';
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

        // エラーメッセージから関連する組み込みワードを検出してヒントを表示
        const hint = this.detectSyntaxHint(errorMessage);
        if (hint) {
            this.elements.outputDisplay.appendChild(document.createElement('br'));
            this.elements.outputDisplay.appendChild(document.createElement('br'));

            const hintLabel = document.createElement('span');
            hintLabel.style.color = '#28a745';
            hintLabel.style.fontWeight = 'bold';
            hintLabel.textContent = 'ヒント:';
            this.elements.outputDisplay.appendChild(hintLabel);

            this.elements.outputDisplay.appendChild(document.createElement('br'));

            const hintSpan = document.createElement('span');
            hintSpan.style.color = '#28a745';
            hintSpan.textContent = hint;
            hintSpan.style.fontFamily = "'Consolas', 'Monaco', monospace";
            this.elements.outputDisplay.appendChild(hintSpan);
        }
    }

    /**
     * エラーメッセージから関連する組み込みワードを検出し、構文ヒントを返す
     */
    private detectSyntaxHint(errorMessage: string): string | null {
        // エラーメッセージ中のすべての大文字ワードを抽出
        const words = errorMessage.match(/\b[A-Z][A-Z0-9]*\b/g) || [];

        // 最初に見つかった組み込みワードのヒントを返す
        for (const word of words) {
            if (word in this.syntaxHints) {
                return this.syntaxHints[word]!;
            }
        }

        // 演算子を検出（+, -, *, /, =, <, <=, >, >=, :）
        const operators = ['+', '-', '*', '/', '=', '<=', '>=', '<', '>', ':'];
        for (const op of operators) {
            if (errorMessage.includes(op) && op in this.syntaxHints) {
                return this.syntaxHints[op]!;
            }
        }

        return null;
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
                elem.textContent = this.formatValue(item, 0); // <--- 修正点: depth 0で呼び出し
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
            case 'string':
                return `'${item.value}'`;
            case 'symbol':
                return String(item.value);
            case 'boolean':
                return item.value ? 'true' : 'false';
            case 'vector': {
                // ★ ここからが修正箇所
                const bracketIndex = depth % 3; // ネストレベルを3で割った剰余
                let openBracket: string, closeBracket: string;

                switch (bracketIndex) {
                    case 0: openBracket = '['; closeBracket = ']'; break; // レベル 0, 3, 6...
                    case 1: openBracket = '{'; closeBracket = '}'; break; // レベル 1, 4, 7...
                    case 2: openBracket = '('; closeBracket = ')'; break; // レベル 2, 5, 8...
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
                // ★ 修正ここまで
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
