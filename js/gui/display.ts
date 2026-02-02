// js/gui/display.ts

import type { Value, ExecuteResult } from '../wasm-types';
import { AUDIO_ENGINE } from '../audio/audio-engine';
import { formatFractionScientific } from './value-formatter';
import { pipe } from './fp-utils';

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
    readonly showInfo: (text: string, append?: boolean) => void;
    readonly updateStack: (stack: Value[]) => void;
    readonly getState: () => DisplayState;
}

// Bracket cycling: depth 0 → {}, depth 1 → (), depth 2 → []
const getBrackets = (depth: number): [string, string] => {
    switch (depth % 3) {
        case 0: return ['{', '}'];
        case 1: return ['(', ')'];
        case 2: return ['[', ']'];
        default: return ['{', '}'];
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

const bytesToString = (data: unknown[]): string => {
    const bytes = data.map(frac => {
        if (!frac || typeof frac !== 'object') return 0;
        const f = frac as Record<string, unknown>;
        const num = parseInt(String(f.numerator || '0'), 10);
        const den = parseInt(String(f.denominator || '1'), 10);
        return den === 1 ? num : Math.floor(num / den);
    }).filter(n => n >= 0 && n <= 255);

    try {
        return new TextDecoder('utf-8').decode(new Uint8Array(bytes));
    } catch {
        return bytes.map(b => String.fromCharCode(b)).join('');
    }
};

const formatTensorRecursive = (shape: number[], data: unknown[], depth: number, displayHint?: string): string => {
    const [open, close] = getBrackets(depth);

    if (shape.length === 0) {
        if (data.length === 0) return `${open}${close}`;
        return `${open} ${formatFraction(data[0])} ${close}`;
    }

    if (shape.length === 1) {
        if (data.length === 0) return `${open}${close}`;
        if (displayHint === 'string') {
            const str = bytesToString(data);
            return `'${str}'`;
        }
        const elements = data.map(frac => formatFraction(frac)).join(' ');
        return `${open} ${elements} ${close}`;
    }

    const outerSize = shape[0] ?? 0;
    const innerShape = shape.slice(1);
    const innerSize = innerShape.reduce((a, b) => a * b, 1);

    const parts: string[] = [];
    for (let i = 0; i < outerSize; i++) {
        const innerData = data.slice(i * innerSize, (i + 1) * innerSize);
        parts.push(formatTensorRecursive(innerShape, innerData, depth + 1, displayHint));
    }

    return `${open} ${parts.join(' ')} ${close}`;
};

const formatTensor = (value: unknown, depth: number): string => {
    if (!value || typeof value !== 'object') return '?';
    const v = value as Record<string, unknown>;
    if (!('shape' in v) || !('data' in v)) return '?';
    const displayHint = v.displayHint as string | undefined;
    return formatTensorRecursive(v.shape as number[], v.data as unknown[], depth, displayHint);
};

const formatVector = (value: unknown, depth: number, pipeSeparated?: boolean): string => {
    const [open, close] = getBrackets(depth);

    if (Array.isArray(value)) {
        if (value.length === 0) {
            return `${open}${close}`;
        }
        const elements = value.map((v: Value) => {
            try { return formatValue(v, depth + 1); } catch { return '?'; }
        });
        // パイプ区切りの場合は | で結合
        const separator = pipeSeparated ? ' | ' : ' ';
        return `${open} ${elements.join(separator)} ${close}`;
    }
    return `${open}${close}`;
};

const formatValue = (item: Value, depth: number): string => {
    if (!item || !item.type) return 'unknown';

    switch (item.type) {
        case 'number':
            return formatNumber(item.value);
        case 'datetime':
            return formatDateTime(item.value);
        case 'tensor':
            return formatTensor(item.value, depth);
        case 'string':
            return `'${item.value}'`;
        case 'symbol':
            return String(item.value);
        case 'boolean':
            return item.value ? 'TRUE' : 'FALSE';
        case 'vector':
            return formatVector(item.value, depth, item.pipeSeparated);
        case 'nil':
            return 'NIL';
        case 'block': {
            const source = (item as unknown as { source: string }).source || '';
            return `"${source}"`;
        }
        default:
            return JSON.stringify(item.value);
    }
};

const extractAudioCommands = (output: string): string[] =>
    output.split('\n')
        .filter(line => line.startsWith('AUDIO:'))
        .map(line => line.substring(6));

const extractConfigCommands = (output: string): string[] =>
    output.split('\n')
        .filter(line => line.startsWith('CONFIG:'))
        .map(line => line.substring(7));

const extractEffectCommands = (output: string): string[] =>
    output.split('\n')
        .filter(line => line.startsWith('EFFECT:'))
        .map(line => line.substring(7));

const filterAudioCommands = (output: string): string =>
    output.split('\n')
        .filter(line => !line.startsWith('AUDIO:') && !line.startsWith('CONFIG:') && !line.startsWith('EFFECT:'))
        .join('\n');

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

const processEffectCommands = (output: string): void => {
    extractEffectCommands(output).forEach(commandStr => {
        try {
            const effect = JSON.parse(commandStr);
            if (effect.gain !== undefined) {
                AUDIO_ENGINE.setGain(effect.gain);
            }
            if (effect.pan !== undefined) {
                AUDIO_ENGINE.setPan(effect.pan);
            }
        } catch {
            console.error('Failed to parse EFFECT command:', commandStr);
        }
    });
};

const processConfigCommands = (output: string): void => {
    extractConfigCommands(output).forEach(commandStr => {
        try {
            const config = JSON.parse(commandStr);
            if (config.slot_duration !== undefined) {
                AUDIO_ENGINE.setSlotDuration(config.slot_duration);
                console.log(`Slot duration set to ${config.slot_duration}s`);
            }
        } catch {
            console.error('Failed to parse CONFIG command');
        }
    });
};

const processAudioCommands = (output: string): void => {
    // Process EFFECT commands first (they set gain/pan before playback)
    processEffectCommands(output);
    // Process CONFIG commands (they may affect audio playback)
    processConfigCommands(output);

    extractAudioCommands(output).forEach(commandStr => {
        try {
            const audioCommand = JSON.parse(commandStr);
            AUDIO_ENGINE.playAudioCommand(audioCommand).catch(console.error);
        } catch {
            console.error('Failed to parse audio command');
        }
    });
};

export const createDisplay = (elements: DisplayElements): Display => {
    let mainOutput = '';

    const init = (): void => {
        elements.outputDisplay.style.whiteSpace = 'pre-wrap';
        AUDIO_ENGINE.init().catch(console.error);
    };

    const appendSpan = (text: string, color: string): HTMLSpanElement => {
        const span = createSpan(text.replace(/\\n/g, '\n'), color);
        appendToElement(elements.outputDisplay, span);
        return span;
    };

    const showExecutionResult = (result: ExecuteResult): void => {
        const { debug, program } = formatExecutionOutput(result);
        processAudioCommands(result.output || '');

        mainOutput = `${debug}\n${program}`;
        clearElement(elements.outputDisplay);

        if (debug) {
            appendSpan(debug, '#333');
        }

        if (debug && program) {
            appendToElement(elements.outputDisplay, document.createElement('br'));
        }

        if (program) {
            appendSpan(program, '#4DC4FF');
        }

        if (!debug && !program && result.status === 'OK') {
            appendSpan('OK', '#333');
        }
    };

    const appendExecutionResult = (result: ExecuteResult): void => {
        const programOutput = (result.output || '').trim();
        processAudioCommands(programOutput);
        const filteredOutput = filterAudioCommands(programOutput);

        if (filteredOutput) {
            appendSpan(filteredOutput, '#4DC4FF');
        }
    };

    const showOutput = (text: string): void => {
        processAudioCommands(text);
        const filteredText = filterAudioCommands(text);

        mainOutput = filteredText;
        clearElement(elements.outputDisplay);
        appendSpan(filteredText, '#4DC4FF');
    };

    const showError = (error: Error | { message?: string } | string): void => {
        const errorMessage = formatErrorMessage(error);

        mainOutput = errorMessage;
        clearElement(elements.outputDisplay);

        const span = appendSpan(errorMessage, '#dc3545');
        span.style.fontWeight = 'bold';
    };

    // Show info message
    const showInfo = (text: string, append = false): void => {
        if (append && elements.outputDisplay.innerHTML.trim() !== '') {
            mainOutput = `${mainOutput}\n${text}`;
            appendSpan('\n' + text, '#666');
        } else {
            mainOutput = text;
            clearElement(elements.outputDisplay);
            appendSpan(text, '#666');
        }
    };

    const updateStack = (stack: Value[]): void => {
        const display = elements.stackDisplay;
        clearElement(display);

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
                elem.textContent = formatValue(item, 0);
            } catch {
                console.error(`Error formatting item ${index}`);
                elem.textContent = 'ERROR';
            }
            appendToElement(container, elem);
        });

        appendToElement(display, container);
    };

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
