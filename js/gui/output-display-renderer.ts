import type { Value, ExecuteResult } from '../wasm-interpreter-types';
import { AUDIO_ENGINE } from '../audio/audio-engine';
import { formatFractionScientific } from './value-formatter';
import { pipe } from './functional-result-helpers';

export interface DisplayElements {
    outputDisplay: HTMLElement;
    stackDisplay: HTMLElement;
}

export type StackEditCallback = (updatedStack: Value[]) => void;

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

// Paul Tol "Muted" palette — color-vision-deficiency safe (9 chromatic colors, excluding pale grey)
const BRACKET_DEPTH_COLORS: readonly string[] = [
    '#332288', // indigo
    '#88CCEE', // cyan
    '#44AA99', // teal
    '#117733', // green
    '#999933', // olive
    '#DDCC77', // sand
    '#CC6677', // rose
    '#882255', // wine
    '#AA4499', // purple
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

// ---------------------------------------------------------------------------
// Grid (spreadsheet-style) rendering for Stack area
// ---------------------------------------------------------------------------

type GridClassification =
    | { kind: 'scalar' }
    | { kind: '1d'; elements: Value[] }
    | { kind: '2d'; rows: Value[][]; cols: number }
    | { kind: '3d-plus'; elements: Value[] }
    | { kind: 'fallback' };

const classifyVector = (item: Value): GridClassification => {
    if (item.type !== 'vector' || !Array.isArray(item.value)) return { kind: 'fallback' };
    const elements: Value[] = item.value;
    if (elements.length === 0) return { kind: 'fallback' };

    const checkLeaf = (v: Value): boolean =>
        v.type !== 'vector' && v.type !== 'tensor';

    // All elements are leaves → 1D
    if (elements.every(checkLeaf)) return { kind: '1d', elements };

    // All elements are vectors → check for uniform 2D
    if (elements.every((v: Value) => v.type === 'vector' && Array.isArray(v.value))) {
        const rows: Value[][] = elements.map((v: Value) => v.value as Value[]);
        const colCounts: number[] = rows.map((r: Value[]) => r.length);
        const cols: number = colCounts[0] ?? 0;

        // Uniform column count → 2D grid
        if (cols > 0 && colCounts.every((c: number) => c === cols)) {
            // Check if all cells are leaves → pure 2D
            const allLeaves: boolean = rows.every((row: Value[]) => row.every(checkLeaf));
            if (allLeaves) return { kind: '2d', rows, cols };

            // Cells contain sub-vectors → 3D+
            return { kind: '3d-plus', elements };
        }

        // Non-uniform columns → fallback
        return { kind: 'fallback' };
    }

    // Mixed types → fallback
    return { kind: 'fallback' };
};

const GRID_DEPTH_BACKGROUNDS: readonly string[] = [
    'rgba(51, 34, 136, 0.06)',   // indigo
    'rgba(136, 204, 238, 0.10)', // cyan
    'rgba(68, 170, 153, 0.10)',  // teal
    'rgba(17, 119, 51, 0.08)',   // green
    'rgba(153, 153, 51, 0.08)',  // olive
] as const;

const lookupGridBackground = (depth: number): string =>
    GRID_DEPTH_BACKGROUNDS[(depth - 1) % GRID_DEPTH_BACKGROUNDS.length] ?? GRID_DEPTH_BACKGROUNDS[0]!;

const formatLeafValue = (item: Value): string => formatValue(item, 1);

interface EditContext {
    readonly stack: Value[];
    readonly stackIndex: number;
    readonly onEdit: StackEditCallback;
}

const cloneValue = (v: Value): Value => {
    if (v.type === 'vector' && Array.isArray(v.value)) {
        return { ...v, value: (v.value as Value[]).map(cloneValue) };
    }
    return { ...v };
};

const cloneStack = (stack: Value[]): Value[] => stack.map(cloneValue);

const resolveValueAtPath = (root: Value, path: number[]): Value | null => {
    let current: Value = root;
    for (const idx of path) {
        if (current.type !== 'vector' || !Array.isArray(current.value)) return null;
        const arr: Value[] = current.value;
        if (idx < 0 || idx >= arr.length) return null;
        current = arr[idx]!;
    }
    return current;
};

const applyEditAtPath = (root: Value, path: number[], newLeaf: Value): void => {
    if (path.length === 0) return;
    const parentPath: number[] = path.slice(0, -1);
    const leafIndex: number = path[path.length - 1]!;
    const parent: Value | null = parentPath.length === 0 ? root : resolveValueAtPath(root, parentPath);
    if (!parent || parent.type !== 'vector' || !Array.isArray(parent.value)) return;
    (parent.value as Value[])[leafIndex] = newLeaf;
};

const parseEditedValue = (text: string): Value | null => {
    const trimmed: string = text.trim();
    if (trimmed === '') return null;

    // Boolean
    if (trimmed === 'TRUE') return { type: 'boolean', value: true, displayHint: 'boolean' };
    if (trimmed === 'FALSE') return { type: 'boolean', value: false, displayHint: 'boolean' };
    if (trimmed === 'NIL') return { type: 'nil', value: null, displayHint: 'nil' };

    // String (quoted)
    if (trimmed.startsWith("'") && trimmed.endsWith("'") && trimmed.length >= 2) {
        return { type: 'string', value: trimmed.slice(1, -1) };
    }

    // Fraction (e.g. "3/4")
    const fractionMatch: RegExpMatchArray | null = trimmed.match(/^(-?\d+)\/(-?\d+)$/);
    if (fractionMatch) {
        return { type: 'number', value: { numerator: fractionMatch[1], denominator: fractionMatch[2] } };
    }

    // Integer
    if (/^-?\d+$/.test(trimmed)) {
        return { type: 'number', value: { numerator: trimmed, denominator: '1' } };
    }

    // Decimal → fraction approximation
    const num: number = parseFloat(trimmed);
    if (!isNaN(num)) {
        // Simple decimal to fraction: multiply by 10^decimals
        const parts: string[] = trimmed.split('.');
        const decimals: number = parts[1]?.length ?? 0;
        const denom: number = Math.pow(10, decimals);
        const numer: number = Math.round(num * denom);
        return { type: 'number', value: { numerator: String(numer), denominator: String(denom) } };
    }

    return null;
};

const activateInlineEdit = (td: HTMLElement, currentText: string, editCtx: EditContext, cellPath: number[]): void => {
    const input = document.createElement('input');
    input.type = 'text';
    input.className = 'stack-grid-edit-input';
    input.value = currentText;

    td.textContent = '';
    td.appendChild(input);
    input.focus();
    input.select();

    const commitEdit = (): void => {
        const newValue: Value | null = parseEditedValue(input.value);
        if (!newValue) {
            // Revert
            td.textContent = currentText;
            return;
        }

        const updatedStack: Value[] = cloneStack(editCtx.stack);
        const rootItem: Value = updatedStack[editCtx.stackIndex]!;

        if (cellPath.length === 0) {
            // Editing a scalar stack item directly
            updatedStack[editCtx.stackIndex] = newValue;
        } else {
            applyEditAtPath(rootItem, cellPath, newValue);
        }

        editCtx.onEdit(updatedStack);
    };

    let committed = false;
    input.addEventListener('keydown', (e: KeyboardEvent) => {
        if (e.key === 'Enter') {
            committed = true;
            commitEdit();
        } else if (e.key === 'Escape') {
            committed = true;
            td.textContent = currentText;
        }
    });

    input.addEventListener('blur', () => {
        if (!committed) {
            commitEdit();
        }
    });
};

const renderGridCell = (item: Value, depth: number, editCtx: EditContext | null, cellPath: number[]): HTMLElement => {
    const td = document.createElement('td');
    td.className = 'stack-grid-cell';

    if (item.type === 'vector' && Array.isArray(item.value)) {
        // Nested vector inside a cell → render sub-grid
        const subGrid = renderVectorAsGrid(item, depth + 1, editCtx, cellPath);
        td.appendChild(subGrid);
    } else {
        const text: string = formatLeafValue(item);
        td.textContent = text;

        if (editCtx && (item.type === 'number' || item.type === 'string' || item.type === 'boolean' || item.type === 'nil')) {
            td.classList.add('stack-grid-cell-editable');
            td.addEventListener('click', () => {
                activateInlineEdit(td, text, editCtx, cellPath);
            });
        }
    }
    return td;
};

const renderVectorAsGrid = (item: Value, depth: number, editCtx: EditContext | null, parentPath: number[]): HTMLElement => {
    const classification = classifyVector(item);

    if (classification.kind === 'fallback' || classification.kind === 'scalar') {
        return renderStackValueNode(item, depth);
    }

    const table = document.createElement('table');
    table.className = 'stack-grid-table';
    table.dataset.depth = String(depth);
    if (depth > 1) {
        table.style.backgroundColor = lookupGridBackground(depth);
    }
    table.style.borderColor = lookupBracketColor(depth);

    if (classification.kind === '1d') {
        const tr = document.createElement('tr');
        classification.elements.forEach((element: Value, colIdx: number) => {
            const cellPath: number[] = [...parentPath, colIdx];
            tr.appendChild(renderGridCell(element, depth, editCtx, cellPath));
        });
        table.appendChild(tr);
        return table;
    }

    if (classification.kind === '2d') {
        classification.rows.forEach((row: Value[], rowIdx: number) => {
            const tr = document.createElement('tr');
            row.forEach((cell: Value, colIdx: number) => {
                const cellPath: number[] = [...parentPath, rowIdx, colIdx];
                tr.appendChild(renderGridCell(cell, depth, editCtx, cellPath));
            });
            table.appendChild(tr);
        });
        return table;
    }

    if (classification.kind === '3d-plus') {
        const outerElements: Value[] = classification.elements;
        const innerRows: Value[][] = outerElements.map((v: Value) => v.value as Value[]);

        innerRows.forEach((row: Value[], rowIdx: number) => {
            const tr = document.createElement('tr');
            row.forEach((cell: Value, colIdx: number) => {
                const cellPath: number[] = [...parentPath, rowIdx, colIdx];
                tr.appendChild(renderGridCell(cell, depth, editCtx, cellPath));
            });
            table.appendChild(tr);
        });
        return table;
    }

    return renderStackValueNode(item, depth);
};

const renderScalarAsGrid = (item: Value, editCtx: EditContext | null): HTMLElement => {
    const table = document.createElement('table');
    table.className = 'stack-grid-table';
    table.dataset.depth = '1';
    table.style.borderColor = lookupBracketColor(1);

    const tr = document.createElement('tr');
    const td = document.createElement('td');
    td.className = 'stack-grid-cell';

    const text: string = formatLeafValue(item);
    td.textContent = text;

    if (editCtx && (item.type === 'number' || item.type === 'string' || item.type === 'boolean' || item.type === 'nil')) {
        td.classList.add('stack-grid-cell-editable');
        td.addEventListener('click', () => {
            activateInlineEdit(td, text, editCtx, []);
        });
    }

    tr.appendChild(td);
    table.appendChild(tr);
    return table;
};

const renderStackItemAsGrid = (item: Value, editCtx: EditContext | null): HTMLElement => {
    if (item.type === 'vector' && Array.isArray(item.value)) {
        return renderVectorAsGrid(item, 1, editCtx, []);
    }
    if (item.type === 'tensor' && item.value && typeof item.value === 'object') {
        // Tensor: render as text node inside a 1×1 grid wrapper for visual consistency
        const table = document.createElement('table');
        table.className = 'stack-grid-table';
        table.dataset.depth = '1';
        table.style.borderColor = lookupBracketColor(1);
        const tr = document.createElement('tr');
        const td = document.createElement('td');
        td.className = 'stack-grid-cell';
        td.appendChild(renderStackValueNode(item, 1));
        tr.appendChild(td);
        table.appendChild(tr);
        return table;
    }
    return renderScalarAsGrid(item, editCtx);
};

// ---------------------------------------------------------------------------
// Original text-mode rendering
// ---------------------------------------------------------------------------

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

export const createDisplay = (elements: DisplayElements, onStackEdit?: StackEditCallback): Display => {
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
                const editCtx: EditContext | null = onStackEdit
                    ? { stack, stackIndex: index, onEdit: onStackEdit }
                    : null;
                elem.appendChild(renderStackItemAsGrid(item, editCtx));
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

export const displayUtils = {
    formatValue,
    formatNumber,
    formatDateTime,
    formatTensor,
    formatVector,
    lookupBracketsAtDepth,
    removeSpecialCommandLines,
    formatErrorMessage
};
