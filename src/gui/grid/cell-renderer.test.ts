// Cell display formatting tests (plan §2.1–§2.2, §8.1).

import { describe, expect, test } from 'vitest';
import type { Value } from '../../wasm-interpreter-types';
import { renderCellDisplay } from './cell-renderer';

const num = (numerator: string, denominator = '1'): Value => ({
    type: 'number',
    value: { numerator, denominator },
});

const value = (stack: Value[]) => ({ kind: 'value' as const, stack });

describe('renderCellDisplay', () => {
    test('null state renders empty', () => {
        expect(renderCellDisplay(null)).toEqual({ text: '', kind: 'empty', detail: null });
    });

    test('a scalar number renders in canonical fraction form', () => {
        expect(renderCellDisplay(value([num('42')]))).toMatchObject({
            text: '42',
            kind: 'number',
        });
        expect(renderCellDisplay(value([num('1', '3')]))).toMatchObject({
            text: '1/3',
            kind: 'number',
        });
    });

    test('a top-level string renders without quotes (WYSIWYG)', () => {
        expect(
            renderCellDisplay(value([{ type: 'string', value: 'hello' }])),
        ).toMatchObject({ text: 'hello', kind: 'text' });
    });

    test('a 1-element Vector renders as its scalar (plan §2.1 convention)', () => {
        const wrapped: Value = { type: 'vector', value: [num('42')] };
        expect(renderCellDisplay(value([wrapped]))).toMatchObject({
            text: '42',
            kind: 'number',
        });
    });

    test('a text-hinted byte Vector decodes to plain text (WYSIWYG)', () => {
        const text: Value = {
            type: 'vector',
            displayHint: 'text',
            value: [num('116'), num('111'), num('116'), num('97'), num('108')],
        };
        expect(renderCellDisplay(value([text]))).toMatchObject({
            text: 'total',
            kind: 'text',
        });
    });

    test('a text-hinted Vector nested in a Vector keeps quotes', () => {
        const inner: Value = {
            type: 'vector',
            displayHint: 'text',
            value: [num('97'), num('98')],
        };
        const outer: Value = { type: 'vector', value: [num('1'), inner, num('2')] };
        expect(renderCellDisplay(value([outer]))).toMatchObject({
            text: "[ 1 'ab' 2 ]",
            kind: 'vector',
        });
    });

    test('a vector keeps its structure visible, nested strings quoted', () => {
        const vec: Value = {
            type: 'vector',
            value: [num('1'), { type: 'string', value: 'a' }, num('2')],
        };
        expect(renderCellDisplay(value([vec]))).toMatchObject({
            text: "[ 1 'a' 2 ]",
            kind: 'vector',
        });
    });

    test('NIL renders with its reason (plan §2.2)', () => {
        const nil: Value = {
            type: 'nil',
            value: null,
            semantics: {
                semanticKind: 'absence',
                shape: 'scalar',
                capabilities: [],
                origin: 'runtime',
                absence: {
                    reason: 'divisionByZero',
                    origin: 'runtime',
                    recoverability: 'recoverable',
                },
            },
        };
        const display = renderCellDisplay(value([nil]));
        expect(display.text).toBe('NIL · divisionByZero');
        expect(display.kind).toBe('nil');
        expect(display.detail).toContain('divisionByZero');
    });

    test('UNKNOWN renders as the third truth value', () => {
        expect(
            renderCellDisplay(value([{ type: 'truthValue', value: 'unknown' }])),
        ).toMatchObject({ text: 'UNKNOWN', kind: 'unknown' });
        expect(
            renderCellDisplay(value([{ type: 'truthValue', value: 'true' }])),
        ).toMatchObject({ text: 'TRUE', kind: 'boolean' });
    });

    test('a multi-value stack renders stacked with a lint hint (plan §8.1)', () => {
        const display = renderCellDisplay(value([num('1'), num('2')]));
        expect(display.text).toBe('1\n2');
        expect(display.kind).toBe('stack');
        expect(display.detail).toContain('2');
    });

    test('an empty final stack renders empty', () => {
        expect(renderCellDisplay(value([]))).toMatchObject({ text: '', kind: 'empty' });
    });

    test('error, cyclic, and blocked states are visually distinct', () => {
        expect(
            renderCellDisplay({ kind: 'error', message: 'Unknown word: NOPE' }),
        ).toMatchObject({ kind: 'error', detail: 'Unknown word: NOPE' });
        expect(renderCellDisplay({ kind: 'cyclic' }).kind).toBe('cyclic');
        expect(renderCellDisplay({ kind: 'blocked' }).kind).toBe('blocked');
    });
});
