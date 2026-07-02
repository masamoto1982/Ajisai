// Sheet view cell display formatting (redesign plan §2.1–§2.2). Pure
// data → display mapping, DOM-free so vitest covers it directly.
//
// WYSIWYG choices (Numbers-oriented, plan §0): a lone string value renders
// without quotes (the cell shows content, not source — the raw text stays
// available in the cell editor), numbers render in canonical fraction form
// via the shared value-formatter, and Vector structure stays visible with
// brackets (structure visibility is an Ajisai principle). Partial failure
// is shown, not flattened: NIL renders with its reason and UNKNOWN renders
// as the third truth value, exactly as in the Editor view (plan §2.2).

import type { Fraction, Value } from '../../wasm-interpreter-types';
import { formatFraction } from '../value-formatter';
import type { CellEvaluationState } from '../../sheet/sheet-evaluator';

/** Visual category of a rendered cell; the grid maps it to a CSS class. */
export type CellDisplayKind =
    | 'empty'
    | 'number'
    | 'text'
    | 'boolean'
    | 'vector'
    | 'nil'
    | 'unknown'
    | 'error'
    | 'cyclic'
    | 'blocked'
    | 'stack'
    | 'pending';

export interface CellDisplay {
    /** Text shown inside the cell (may contain newlines for stacked values). */
    text: string;
    kind: CellDisplayKind;
    /** Longer diagnosis for hover (`title`), when there is one. */
    detail: string | null;
}

// Cell-level number rendering: the canonical Editor display keeps the
// denominator (`3/1`), but a WYSIWYG cell shows integers as integers.
// Display only — the value stays the exact fraction (plan §10: 丸めは表示のみ,
// here not even rounding, just dropping a unit denominator).
const formatNumberForCell = (fraction: { numerator: string; denominator: string }): string =>
    fraction.denominator === '1' ? fraction.numerator : formatFraction(fraction);

/**
 * Strings travel the wire as Vectors of UTF-8 byte numbers with
 * `displayHint: 'text'` (vector-oriented language: strings are codepoint
 * vectors). Decode when every element is an integral byte; otherwise the
 * value renders structurally.
 */
const decodeTextVector = (items: Value[]): string | null => {
    const bytes: number[] = [];
    for (const item of items) {
        if (item.type !== 'number') return null;
        const fraction = item.value as Fraction;
        if (fraction.denominator !== '1') return null;
        const byte = Number(fraction.numerator);
        if (!Number.isInteger(byte) || byte < 0 || byte > 255) return null;
        bytes.push(byte);
    }
    try {
        return new TextDecoder('utf-8').decode(new Uint8Array(bytes));
    } catch {
        return bytes.map((b) => String.fromCharCode(b)).join('');
    }
};

const extractVectorItems = (value: Value): Value[] =>
    Array.isArray(value.value) ? (value.value as Value[]) : [];

const formatValueForCell = (value: Value, depth: number): string => {
    switch (value.type) {
        case 'number':
            return formatNumberForCell(value.value);
        case 'string':
            // Top-level strings are content (WYSIWYG); nested strings keep
            // quotes so Vector structure stays readable.
            return depth === 0 ? String(value.value) : `'${value.value}'`;
        case 'boolean':
            return value.value ? 'TRUE' : 'FALSE';
        case 'truthValue':
            return value.value === 'unknown' ? 'UNKNOWN' : String(value.value).toUpperCase();
        case 'vector': {
            const items = extractVectorItems(value);
            if (value.displayHint === 'text') {
                const text = decodeTextVector(items);
                if (text !== null) return depth === 0 ? text : `'${text}'`;
            }
            // Scalar convention (plan §2.1): a 1-element Vector IS the
            // scalar, so the cell shows the element, not the brackets.
            if (items.length === 1) {
                return formatValueForCell(items[0] as Value, depth);
            }
            if (items.length === 0) return '[ ]';
            const body = items.map((item) => formatValueForCell(item, depth + 1)).join(' ');
            return `[ ${body} ]`;
        }
        case 'nil': {
            const reason = value.semantics?.absence?.reason;
            return reason ? `NIL · ${reason}` : 'NIL';
        }
        default:
            return typeof value.value === 'string'
                ? String(value.value)
                : JSON.stringify(value.value);
    }
};

const classifySingle = (value: Value): CellDisplayKind => {
    switch (value.type) {
        case 'number':
            return 'number';
        case 'string':
            return 'text';
        case 'boolean':
            return 'boolean';
        case 'truthValue':
            return value.value === 'unknown' ? 'unknown' : 'boolean';
        case 'vector': {
            const items = extractVectorItems(value);
            if (value.displayHint === 'text' && decodeTextVector(items) !== null) {
                return 'text';
            }
            if (items.length === 1) {
                return classifySingle(items[0] as Value);
            }
            return 'vector';
        }
        case 'nil':
            return 'nil';
        default:
            return 'text';
    }
};

const extractDetail = (value: Value): string | null => {
    const absence = value.semantics?.absence;
    if (!absence) return null;
    const parts: string[] = [];
    if (absence.reason) parts.push(`reason: ${absence.reason}`);
    if (absence.diagnosis?.summary) parts.push(absence.diagnosis.summary);
    return parts.length > 0 ? parts.join(' — ') : null;
};

/**
 * Render an evaluation state for display in a grid cell. A multi-value
 * stack renders stacked top-last (plan §8.1: 縦積み表示 — playable, not an
 * error), flagged as `stack` so the grid can add its lint hint.
 */
export function renderCellDisplay(state: CellEvaluationState | null): CellDisplay {
    if (state === null) {
        return { text: '', kind: 'empty', detail: null };
    }
    switch (state.kind) {
        case 'error':
            return { text: '⚠ error', kind: 'error', detail: state.message };
        case 'cyclic':
            return { text: '⟳ 循環参照', kind: 'cyclic', detail: 'このセルは自分自身を参照しています' };
        case 'blocked':
            return {
                text: '⟳ 循環待ち',
                kind: 'blocked',
                detail: '循環参照のセルに依存しているため評価できません',
            };
        case 'value': {
            if (state.stack.length === 0) {
                return { text: '', kind: 'empty', detail: null };
            }
            if (state.stack.length === 1) {
                const value = state.stack[0] as Value;
                return {
                    text: formatValueForCell(value, 0),
                    kind: classifySingle(value),
                    detail: extractDetail(value),
                };
            }
            const lines = state.stack.map((value) => formatValueForCell(value, 0));
            return {
                text: lines.join('\n'),
                kind: 'stack',
                detail: `スタックに ${state.stack.length} 値が残っています`,
            };
        }
    }
}
