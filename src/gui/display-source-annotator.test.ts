import { describe, expect, test } from 'vitest';
import { annotateStackDisplaySources } from './display-source-annotator';
import type { Value } from '../wasm-interpreter-types';

const num = (numerator: string, denominator: string = '1'): Value => ({
    type: 'number',
    value: { numerator, denominator }
});

const vec = (value: Value[]): Value => ({
    type: 'vector',
    value
});

describe('annotateStackDisplaySources', () => {
    test('adds displaySource to literal-only numeric stack suffixes', () => {
        const annotated = annotateStackDisplaySources([num('1', '2')], '0.5');
        expect(annotated?.[0]?.value.displaySource).toBe('0.5');
    });

    test('preserves nested vector numeric literal spellings', () => {
        const annotated = annotateStackDisplaySources(
            [vec([num('1', '2'), vec([num('2', '1')])])],
            '[ 0.5 [ 2/1 ] ]'
        );

        const root = annotated?.[0]?.value as Value[];
        const inner = root[1]!.value as Value[];
        expect(root[0]!.value.displaySource).toBe('0.5');
        expect(inner[0]!.value.displaySource).toBe('2/1');
    });

    test('does not annotate explicit operation results', () => {
        expect(annotateStackDisplaySources([num('1')], '0.5 0.5 ADD')).toBeNull();
    });
});
