// js/gui/display.ts - 表示管理（関数型スタイル）

import type { Value, ExecuteResult } from '../wasm-types';
import { AUDIO_ENGINE } from '../audio/audio-engine';
import { formatFractionScientific } from './value-formatter';
import { pipe } from './fp-utils';

// ============================================================
// 型定義
// ============================================================

export interface DisplayElements {
    outputDisplay: HTMLElement;
    stackDisplay: HTMLElement;
}

export interface DisplayState {
    readonly mainOutput: string;
}

export interface Display {
    readonly init: () => void;
    readonly showExecutionResult: (result: ExecuteResult) => void;
    readonly appendExecutionResult: (result: ExecuteResult) => void;
    readonly showOutput: (text: string) => void;
    readonly showError: (error: Error | { message?: string } | string) => void;
    readonly showInfo: (text: string, append?: boolean, en?: string) => void;
    readonly updateStack: (stack: Value[]) => void;
    readonly getState: () => DisplayState;
}

// ============================================================
// 純粋関数: 値のフォーマット
// ============================================================

const getBrackets = (depth: number): [string, string] => {
    switch (depth % 3) {
        case 0: return ['[', ']'];
        case 1: return ['{', '}'];
        case 2: return ['(', ')'];
        default: return ['[', ']'];
    }
};

const formatNumber = (value: unknown): string => {
    if (!value || typeof value !== 'object') return '?';
    const v = value as Record<string, unknown>;
    if (!('numerator' in v) || !('denominator' in v)) return '?';
    return formatFractionScientific(String(v.numerator), String(v.denominator));
};

const formatFraction = (frac: unknown): string => {
    if (!frac || typeof frac !== 'object') return '?';
    const f = frac as Record<string, unknown>;
    if (!('numerator' in f) || !('denominator' in f)) return '?';
    return formatFractionScientific(String(f.numerator), String(f.denominator));
};

const formatDateTime = (value: unknown): string => {
    if (!value || typeof value !== 'object') return '@?';
    const v = value as Record<string, unknown>;
    if (!('numerator' in v) || !('denominator' in v)) return '@?';

    try {
        const numer = BigInt(v.numerator as string);
        const denom = BigInt(v.denominator as string);
        const timestampMs = Number((numer * 1000n) / denom);
        const date = new Date(timestampMs);

        if (isNaN(date.getTime())) {
            return `@${v.numerator}${v.denominator === '1' ? '' : '/' + v.denominator}`;
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
        return `@${v.numerator}${v.denominator === '1' ? '' : '/' + v.denominator}`;
    }
};

const formatTensorRecursive = (shape: number[], data: unknown[], depth: number): string => {
    const [open, close] = getBrackets(depth);

    if (shape.length === 0) {
        if (data.length === 0) return `${open}${close}`;
        return `${open} ${formatFraction(data[0])} ${close}`;
    }

    if (shape.length === 1) {
        if (data.length === 0) return `${open}${close}`;
        const elements = data.map(frac => formatFraction(frac)).join(' ');
        return `${open} ${elements} ${close}`;
    }

    const outerSize = shape[0] ?? 0;
    const innerShape = shape.slice(1);
    const innerSize = innerShape.reduce((a, b) => a * b, 1);

    const parts: string[] = [];
    for (let i = 0; i < outerSize; i++) {
        const innerData = data.slice(i * innerSize, (i + 1) * innerSize);
        parts.push(formatTensorRecursive(innerShape, innerData, depth + 1));
    }

    return `${open} ${parts.join(' ')} ${close}`;
};

const formatTensor = (value: unknown, depth: number): string => {
    if (!value || typeof value !== 'object') return '?';
    const v = value as Record<string, unknown>;
    if (!('shape' in v) || !('data' in v)) return '?';
    return formatTensorRecursive(v.shape as number[], v.data as unknown[], depth);
};

const formatVector = (value: unknown, depth: number): string => {
    const [open, close] = getBrackets(depth + 1);

    if (Array.isArray(value)) {
        const elements = value.map((v: Value) => {
            try { return formatValue(v, depth + 1); } catch { return '?'; }
        }).join(' ');
        return `${open}${elements ? ' ' + elements + ' ' : ''}${close}`;
    }
    return `${open}${close}`;
};

// 再帰的な値フォーマット（純粋関数）
const formatValue = (item: Value, depth: number): string => {
    if (!item || !item.type) return 'unknown';

    switch (item.type) {
        case 'number':
            return formatNumber(item.value);
        case 'datetime':
            return formatDateTime(item.value);
        case 'tensor':
            return formatTensor(item.value, depth + 1);
        case 'string':
            return `'${item.value}'`;
        case 'symbol':
            return String(item.value);
        case 'boolean':
            return item.value ? 'TRUE' : 'FALSE';
        case 'vector':
            return formatVector(item.value, depth);
        case 'nil':
            return 'NIL';
        default:
            return JSON.stringify(item.value);
    }
};

// ============================================================
// 純粋関数: Audio処理
// ============================================================

const extractAudioCommands = (output: string): string[] =>
    output.split('\n')
        .filter(line => line.startsWith('AUDIO:'))
        .map(line => line.substring(6));

const filterAudioCommands = (output: string): string =>
    output.split('\n')
        .filter(line => !line.startsWith('AUDIO:'))
        .join('\n');

// ============================================================
// 純粋関数: 出力整形
// ============================================================

const formatExecutionOutput = (result: ExecuteResult): { debug: string; program: string } => ({
    debug: (result.debugOutput || '').trim(),
    program: pipe(
        (result.output || '').trim(),
        filterAudioCommands
    )
});

const formatErrorMessage = (error: Error | { message?: string } | string): string =>
    typeof error === 'string'
        ? `Error: ${error}`
        : `Error: ${(error as Error).message || error}`;

// ============================================================
// 副作用関数: DOM操作
// ============================================================

const createSpan = (text: string, color: string): HTMLSpanElement => {
    const span = document.createElement('span');
    span.style.color = color;
    span.textContent = text;
    return span;
};

const clearElement = (element: HTMLElement): void => {
    element.innerHTML = '';
};

const appendToElement = (parent: HTMLElement, child: HTMLElement): void => {
    parent.appendChild(child);
};

// ============================================================
// 副作用関数: Audio再生
// ============================================================

const processAudioCommands = (output: string): void => {
    extractAudioCommands(output).forEach(commandStr => {
        try {
            const audioCommand = JSON.parse(commandStr);
            AUDIO_ENGINE.playAudioCommand(audioCommand).catch(console.error);
        } catch {
            console.error('Failed to parse audio command');
        }
    });
};

// ============================================================
// ファクトリ関数: Display作成
// ============================================================

export const createDisplay = (elements: DisplayElements): Display => {
    // 状態（クロージャで保持）
    let mainOutput = '';

    // 初期化
    const init = (): void => {
        elements.outputDisplay.style.whiteSpace = 'pre-wrap';
        AUDIO_ENGINE.init().catch(console.error);
    };

    // 共通のspan追加
    const appendSpan = (text: string, color: string): HTMLSpanElement => {
        const span = createSpan(text.replace(/\\n/g, '\n'), color);
        appendToElement(elements.outputDisplay, span);
        return span;
    };

    // 実行結果の表示
    const showExecutionResult = (result: ExecuteResult): void => {
        const { debug, program } = formatExecutionOutput(result);

        // Audio処理
        processAudioCommands(result.output || '');

        mainOutput = `${debug}\n${program}`;
        clearElement(elements.outputDisplay);

        if (debug) {
            appendSpan(debug, '#a0a0b8');
        }

        if (debug && program) {
            appendToElement(elements.outputDisplay, document.createElement('br'));
        }

        if (program) {
            appendSpan(program, '#82aaff');
        }

        if (!debug && !program && result.status === 'OK') {
            appendSpan('OK', '#a0a0b8');
        }
    };

    // 実行結果の追記
    const appendExecutionResult = (result: ExecuteResult): void => {
        const programOutput = (result.output || '').trim();
        processAudioCommands(programOutput);
        const filteredOutput = filterAudioCommands(programOutput);

        if (filteredOutput) {
            appendSpan(filteredOutput, '#82aaff');
        }
    };

    // 出力表示
    const showOutput = (text: string): void => {
        processAudioCommands(text);
        const filteredText = filterAudioCommands(text);

        mainOutput = filteredText;
        clearElement(elements.outputDisplay);
        appendSpan(filteredText, '#82aaff');
    };

    // エラー表示
    const showError = (error: Error | { message?: string } | string): void => {
        const errorMessage = formatErrorMessage(error);

        mainOutput = errorMessage;
        clearElement(elements.outputDisplay);

        const span = appendSpan(errorMessage, '#ff5370');
        span.style.fontWeight = 'bold';
    };

    // 斜体span追加（英語用）
    const appendItalicSpan = (text: string, color: string): HTMLSpanElement => {
        const span = createSpan(text.replace(/\\n/g, '\n'), color);
        span.style.fontStyle = 'italic';
        appendToElement(elements.outputDisplay, span);
        return span;
    };

    // 情報表示（日本語 + 英語併記対応）
    const showInfo = (text: string, append = false, en?: string): void => {
        const fullText = en ? `${text} (${en})` : text;

        if (append && elements.outputDisplay.innerHTML.trim() !== '') {
            mainOutput = `${mainOutput}\n${fullText}`;
            appendSpan('\n' + text, '#a0a0b8');
            if (en) {
                appendSpan(' ', '#a0a0b8');
                appendItalicSpan(`(${en})`, '#808098');
            }
        } else {
            mainOutput = fullText;
            clearElement(elements.outputDisplay);
            appendSpan(text, '#a0a0b8');
            if (en) {
                appendSpan(' ', '#a0a0b8');
                appendItalicSpan(`(${en})`, '#808098');
            }
        }
    };

    // スタック表示の更新
    const updateStack = (stack: Value[]): void => {
        const display = elements.stackDisplay;
        clearElement(display);

        if (!Array.isArray(stack) || stack.length === 0) {
            display.textContent = '(empty)';
            display.style.color = '#606080';
            return;
        }

        display.style.color = '#e8e8f0';
        const container = document.createElement('div');
        container.style.cssText = 'display:flex;flex-wrap:wrap-reverse;justify-content:flex-start;align-content:flex-end;flex-direction:row';

        stack.forEach((item, index) => {
            const elem = document.createElement('span');
            elem.className = 'stack-item';
            try {
                elem.textContent = formatValue(item, 0);
            } catch {
                console.error(`Error formatting item ${index}`);
                elem.textContent = 'ERROR';
            }
            appendToElement(container, elem);
        });

        appendToElement(display, container);
    };

    // 状態取得
    const getState = (): DisplayState => ({ mainOutput });

    return {
        init,
        showExecutionResult,
        appendExecutionResult,
        showOutput,
        showError,
        showInfo,
        updateStack,
        getState
    };
};

// 純粋関数をエクスポート（テスト用）
export const displayUtils = {
    formatValue,
    formatNumber,
    formatDateTime,
    formatTensor,
    formatVector,
    getBrackets,
    filterAudioCommands,
    formatErrorMessage
};
