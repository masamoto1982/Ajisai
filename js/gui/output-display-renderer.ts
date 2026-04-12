import type { Value, ExecuteResult } from '../wasm-interpreter-types';
import { AUDIO_ENGINE } from '../audio/audio-engine';
import { formatFractionScientific } from './value-formatter';
import { pipe } from './functional-result-helpers';

export interface DisplayElements {
    outputDisplay: HTMLElement;
    stackDisplay: HTMLElement;
}

export interface DisplayState {
    readonly mainOutput: string;
}

export interface Display {
    readonly init: () => void;
    readonly renderExecutionResult: (result: ExecuteResult) => void;
    readonly appendExecutionResult: (result: ExecuteResult) => void;
    readonly renderOutput: (text: string) => void;
    readonly renderError: (error: Error | { message?: string } | string) => void;
    readonly renderInfo: (text: string, append?: boolean) => void;
    readonly renderStack: (stack: Value[]) => void;
    readonly extractState: () => DisplayState;
}

const lookupBracketsAtDepth = (_depth: number): [string, string] => ['[', ']'];


const BRACKET_DEPTH_COLORS: readonly string[] = [
    '#332288',
    '#88CCEE',
    '#44AA99',
    '#117733',
    '#999933',
    '#DDCC77',
    '#CC6677',
    '#882255',
    '#AA4499',
] as const;

const lookupBracketColor = (depth: number): string =>
    BRACKET_DEPTH_COLORS[depth - 1] ?? '#332288';

const createBracketSpan = (bracket: string, depth: number): HTMLSpanElement => {
    const span = document.createElement('span');
    span.className = 'stack-bracket';
    span.style.color = lookupBracketColor(depth);
    span.textContent = bracket;
    return span;
};


const checkFractionObject = (value: unknown): Record<string, unknown> | null => {
    if (!value || typeof value !== 'object') return null;
    const candidate = value as Record<string, unknown>;
    if (!('numerator' in candidate) || !('denominator' in candidate)) return null;
    return candidate;
};

const formatFractionToText = (fraction: Record<string, unknown>): string => {
    const numerator = String(fraction.numerator);
    const denominator = String(fraction.denominator);
    return denominator === '1' ? numerator : `${numerator}/${denominator}`;
};

const parseFractionToNumber = (fraction: Record<string, unknown>): number | null => {
    const numerator = parseInt(String(fraction.numerator || '0'), 10);
    const denominator = parseInt(String(fraction.denominator || '1'), 10);
    if (Number.isNaN(numerator) || Number.isNaN(denominator) || denominator === 0) return null;
    return denominator === 1 ? numerator : Math.floor(numerator / denominator);
};

const formatNumber = (value: unknown): string => {
    const fraction = checkFractionObject(value);
    if (!fraction) return '?';
    return formatFractionScientific(String(fraction.numerator), String(fraction.denominator));
};

const formatFraction = (frac: unknown): string => {
    const fraction = checkFractionObject(frac);
    if (!fraction) return '?';
    return formatFractionScientific(String(fraction.numerator), String(fraction.denominator));
};

const formatDateTime = (value: unknown): string => {
    const fraction = checkFractionObject(value);
    if (!fraction) return '@?';

    try {
        const numer = BigInt(String(fraction.numerator));
        const denom = BigInt(String(fraction.denominator));
        const timestampMs = Number((numer * 1000n) / denom);
        const date = new Date(timestampMs);

        if (isNaN(date.getTime())) {
            return `@${formatFractionToText(fraction)}`;
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
        return `@${formatFractionToText(fraction)}`;
    }
};

const extractByteFromFraction = (frac: unknown): number | null => {
    const fraction: Record<string, unknown> | null = checkFractionObject(frac);
    if (!fraction) return null;
    return parseFractionToNumber(fraction);
};

const deserializeBytesToString = (data: unknown[]): string => {
    const bytes: number[] = data
        .map(extractByteFromFraction)
        .filter((value): value is number => value !== null && value >= 0 && value <= 255);

    try {
        return new TextDecoder('utf-8').decode(new Uint8Array(bytes));
    } catch {
        return bytes.map(b => String.fromCharCode(b)).join('');
    }
};

const formatTensorRecursive = (shape: number[], data: unknown[], depth: number, displayHint?: string): string => {
    const [open, close] = lookupBracketsAtDepth(depth);

    if (shape.length === 0) {
        if (data.length === 0) return `${open}${close}`;
        return `${open} ${formatFraction(data[0])} ${close}`;
    }

    if (shape.length === 1) {
        if (data.length === 0) return `${open}${close}`;
        if (displayHint === 'string') {
            const str = deserializeBytesToString(data);
            return `'${str}'`;
        }
        const elements: string = data.map(frac => formatFraction(frac)).join(' ');
        return `${open} ${elements} ${close}`;
    }

    const outerSize: number = shape[0] ?? 0;
    const innerShape: number[] = shape.slice(1);
    const innerSize: number = innerShape.reduce((a: number, b: number) => a * b, 1);

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

const formatVector = (value: unknown, depth: number): string => {
    const [open, close] = lookupBracketsAtDepth(depth);

    if (Array.isArray(value)) {
        if (value.length === 0) {
            return `${open}${close}`;
        }
        const formatSingleElement = (v: Value): string => {
            try { return formatValue(v, depth + 1); } catch { return '?'; }
        };
        const elements: string = value.map(formatSingleElement).join(' ');
        return `${open} ${elements} ${close}`;
    }
    return `${open}${close}`;
};





const renderStackValueNode = (item: Value, depth: number): HTMLElement => {
    const node = document.createElement('span');
    node.className = 'stack-node';

    if (item.type === 'vector' && Array.isArray(item.value)) {
        node.classList.add('stack-node-vector');
        node.dataset.depth = String(depth);
        node.appendChild(createBracketSpan('[', depth));
        item.value.forEach((child, index) => {
            if (index > 0) node.append(' ');
            node.appendChild(renderStackValueNode(child, depth + 1));
        });
        node.appendChild(createBracketSpan(']', depth));
        return node;
    }

    if (item.type === 'tensor' && item.value && typeof item.value === 'object') {
        const tensor = item.value as { shape?: number[]; data?: unknown[]; displayHint?: string };
        const shape = Array.isArray(tensor.shape) ? tensor.shape : [];
        const data = Array.isArray(tensor.data) ? tensor.data : [];

        const renderTensorNode = (tensorShape: number[], tensorData: unknown[], tensorDepth: number): HTMLElement => {
            const tensorNode = document.createElement('span');
            tensorNode.className = 'stack-node stack-node-vector';
            tensorNode.dataset.depth = String(tensorDepth);

            if (tensorShape.length === 0) {
                tensorNode.appendChild(createBracketSpan('[', tensorDepth));
                tensorNode.appendChild(createBracketSpan(']', tensorDepth));
                return tensorNode;
            }

            if (tensorShape.length === 1) {
                if ((tensor.displayHint ?? '').toLowerCase() === 'string') {
                    tensorNode.append(deserializeBytesToString(tensorData));
                } else {
                    tensorNode.appendChild(createBracketSpan('[', tensorDepth));
                    tensorData.forEach((frac, index) => {
                        if (index > 0) tensorNode.append(' ');
                        tensorNode.append(formatFraction(frac));
                    });
                    tensorNode.appendChild(createBracketSpan(']', tensorDepth));
                }
                return tensorNode;
            }

            tensorNode.appendChild(createBracketSpan('[', tensorDepth));
            const outerSize = tensorShape[0] ?? 0;
            const innerShape = tensorShape.slice(1);
            const innerSize = innerShape.reduce((a, b) => a * b, 1);
            for (let i = 0; i < outerSize; i++) {
                if (i > 0) tensorNode.append(' ');
                const innerData = tensorData.slice(i * innerSize, (i + 1) * innerSize);
                tensorNode.appendChild(renderTensorNode(innerShape, innerData, tensorDepth + 1));
            }
            tensorNode.appendChild(createBracketSpan(']', tensorDepth));
            return tensorNode;
        };

        return renderTensorNode(shape, data, depth);
    }

    if (depth === 1) {
        node.dataset.depth = String(depth);
    }
    node.textContent = formatValue(item, depth);
    return node;
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
            return formatVector(item.value, depth);
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

const extractJsonExportCommands = (output: string): string[] =>
    output.split('\n')
        .filter(line => line.startsWith('JSONEXPORT:'))
        .map(line => line.substring(11));

const checkIsSpecialCommand = (line: string): boolean =>
    line.startsWith('AUDIO:') || line.startsWith('CONFIG:') ||
    line.startsWith('EFFECT:') || line.startsWith('JSONEXPORT:');

const removeSpecialCommandLines = (output: string): string =>
    output.split('\n')
        .filter(line => !checkIsSpecialCommand(line))
        .join('\n');

const formatExecutionOutput = (result: ExecuteResult): { debug: string; program: string } => ({
    debug: (result.debugOutput || '').trim(),
    program: pipe(
        (result.output || '').trim(),
        removeSpecialCommandLines
    )
});

const formatErrorMessage = (error: Error | { message?: string } | string): string =>
    typeof error === 'string'
        ? `Error: ${error}`
        : `Error: ${(error as Error).message || error}`;

const createSpanElement = (text: string, color: string): HTMLSpanElement => {
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

const applyEffectCommands = (output: string): void => {
    extractEffectCommands(output).forEach(commandStr => {
        try {
            const effect = JSON.parse(commandStr);
            if (effect.gain !== undefined) {
                AUDIO_ENGINE.updateGain(effect.gain);
            }
            if (effect.pan !== undefined) {
                AUDIO_ENGINE.updatePan(effect.pan);
            }
        } catch {
            console.error('Failed to parse EFFECT command:', commandStr);
        }
    });
};

const applyConfigCommands = (output: string): void => {
    extractConfigCommands(output).forEach(commandStr => {
        try {
            const config = JSON.parse(commandStr);
            if (config.slot_duration !== undefined) {
                AUDIO_ENGINE.updateSlotDuration(config.slot_duration);
                console.log(`Slot duration set to ${config.slot_duration}s`);
            }
        } catch {
            console.error('Failed to parse CONFIG command');
        }
    });
};

const executeAudioCommands = (output: string): void => {
    applyEffectCommands(output);
    applyConfigCommands(output);

    extractAudioCommands(output).forEach(commandStr => {
        try {
            const audioCommand = JSON.parse(commandStr);
            AUDIO_ENGINE.playAudioCommand(audioCommand).catch(console.error);
        } catch {
            console.error('Failed to parse audio command');
        }
    });
};

const createJsonDownloadLinkElement = (jsonCompact: string): HTMLAnchorElement => {
    let prettyJson: string;
    try {
        prettyJson = JSON.stringify(JSON.parse(jsonCompact), null, 2);
    } catch {
        prettyJson = jsonCompact;
    }

    const blob = new Blob([prettyJson], { type: 'application/json' });
    const url = URL.createObjectURL(blob);

    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const filename = `ajisai_export_${timestamp}.json`;

    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.className = 'json-download-link';
    a.textContent = `Download: ${filename}`;
    return a;
};

const renderJsonExportLinks = (output: string, outputDisplay: HTMLElement): void => {
    extractJsonExportCommands(output).forEach(jsonCompact => {
        const link = createJsonDownloadLinkElement(jsonCompact);
        appendToElement(outputDisplay, document.createElement('br'));
        appendToElement(outputDisplay, link);
    });
};

export const createDisplay = (elements: DisplayElements): Display => {
    let mainOutput = '';

    const init = (): void => {
        elements.outputDisplay.style.whiteSpace = 'pre-wrap';
        AUDIO_ENGINE.init().catch(console.error);
    };

    const appendSpan = (text: string, color: string): HTMLSpanElement => {
        const span = createSpanElement(text.replace(/\\n/g, '\n'), color);
        appendToElement(elements.outputDisplay, span);
        return span;
    };

    const renderExecutionResult = (result: ExecuteResult): void => {
        const { debug, program } = formatExecutionOutput(result);
        const rawOutput = result.output || '';
        executeAudioCommands(rawOutput);

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

        renderJsonExportLinks(rawOutput, elements.outputDisplay);

        if (!debug && !program && !extractJsonExportCommands(rawOutput).length && result.status === 'OK') {
            appendSpan('OK', '#333');
        }
    };

    const appendExecutionResult = (result: ExecuteResult): void => {
        const programOutput = (result.output || '').trim();
        executeAudioCommands(programOutput);
        const filteredOutput = removeSpecialCommandLines(programOutput);

        if (filteredOutput) {
            appendSpan(filteredOutput, '#4DC4FF');
        }
    };

    const renderOutput = (text: string): void => {
        executeAudioCommands(text);
        const filteredText = removeSpecialCommandLines(text);

        mainOutput = filteredText;
        clearElement(elements.outputDisplay);
        appendSpan(filteredText, '#4DC4FF');
    };

    const renderError = (error: Error | { message?: string } | string): void => {
        const errorMessage = formatErrorMessage(error);

        mainOutput = errorMessage;
        clearElement(elements.outputDisplay);

        const span = appendSpan(errorMessage, '#dc3545');
        span.style.fontWeight = 'bold';
    };

    const renderInfo = (text: string, append = false): void => {
        if (append && elements.outputDisplay.innerHTML.trim() !== '') {
            mainOutput = `${mainOutput}\n${text}`;
            appendSpan('\n' + text, '#666');
        } else {
            mainOutput = text;
            clearElement(elements.outputDisplay);
            appendSpan(text, '#666');
        }
    };

    const renderStack = (stack: Value[]): void => {
        const display = elements.stackDisplay;
        clearElement(display);

        if (!Array.isArray(stack) || stack.length === 0) {
            display.classList.add('is-empty');
            const message = document.createElement('div');
            message.className = 'empty-words-message';
            message.textContent = 'No values on the stack yet.';
            display.appendChild(message);
            return;
        }

        display.classList.remove('is-empty');

        const container = document.createElement('div');
        container.className = 'area-content-flow stack-content-flow';

        stack.forEach((item, index) => {
            const elem = document.createElement('span');
            elem.className = 'stack-item';
            try {
                elem.appendChild(renderStackValueNode(item, 1));
            } catch {
                console.error(`Error formatting item ${index}`);
                elem.textContent = 'ERROR';
            }
            appendToElement(container, elem);
        });

        appendToElement(display, container);
    };

    const extractState = (): DisplayState => ({ mainOutput });

    return {
        init,
        renderExecutionResult,
        appendExecutionResult,
        renderOutput,
        renderError,
        renderInfo,
        renderStack,
        extractState
    };
};
