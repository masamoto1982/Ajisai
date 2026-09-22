// The Stack panel and the engine are two implementations of one display.
//
// A Record crosses the protocol as two aligned arrays of nodes
// (LANG.OBSERVATION.PROTOCOL), so the panel re-renders it from those arrays
// rather than receiving the engine's string. Nothing but this test stops the
// two drifting, and they have drifted before: the panel printed `[]` for an
// empty Vector where the engine printed `[ ]`, which went unnoticed while a
// display was not expected to be source. It is now — `[]` is a source error,
// because a bracket must stand alone (`spec/grammar.json`,
// `bracketMustStandAlone`).
//
// Every expectation below is a string captured from the engine by running the
// named program through `ajisai run`, not one written by hand to match the
// panel.

import { describe, expect, test } from 'vitest';
import { formatValue } from './output-display-renderer';

type Node = Parameters<typeof formatValue>[0];

const num = (n: number): Node =>
    ({ type: 'number', value: { numerator: String(n), denominator: '1' } }) as Node;
const str = (s: string): Node => ({ type: 'string', value: s }) as Node;
const vec = (...items: Node[]): Node => ({ type: 'vector', value: items }) as Node;
const rec = (keys: Node[], values: Node[]): Node =>
    ({ type: 'record', value: { keys, values } }) as Node;

const render = (node: Node): string => formatValue(node, 0);

describe('a Record renders as the call that builds it', () => {
    test("[ 'x' 'y' ] [ 1 2 ] RECORD", () => {
        expect(render(rec([str('x'), str('y')], [num(1), num(2)]))).toBe(
            "[ 'x' 'y' ] [ 1/1 2/1 ] RECORD"
        );
    });

    test('the empty Record is two empty Vectors', () => {
        expect(render(rec([], []))).toBe('[ ] [ ] RECORD');
    });

    test('a Record whose value is a Vector — the shape GROUP answers', () => {
        expect(
            render(rec([str('a'), str('b')], [vec(num(1), num(3)), vec(num(2))]))
        ).toBe("[ 'a' 'b' ] [ [ 1/1 3/1 ] [ 2/1 ] ] RECORD");
    });
});

describe('a Vector holding a Record renders as the COLLECT phrase', () => {
    // A bracket literal does not evaluate what is written inside it, so the
    // literal form would read back as a different value: two Vectors and the
    // name RECORD, not one Record.
    test('one Record', () => {
        expect(render(vec(rec([str('a')], [num(1)])))).toBe(
            "[ 'a' ] [ 1/1 ] RECORD 1 COLLECT"
        );
    });

    test('a Record beside an ordinary value', () => {
        expect(render(vec(num(1), rec([str('a')], [num(2)])))).toBe(
            "1/1 [ 'a' ] [ 2/1 ] RECORD 2 COLLECT"
        );
    });

    test('the phrase nests, because each fragment nets one stack value', () => {
        expect(render(vec(vec(rec([str('a')], [num(1)]))))).toBe(
            "[ 'a' ] [ 1/1 ] RECORD 1 COLLECT 1 COLLECT"
        );
    });
});

describe('a Vector without a Record is still a literal', () => {
    test('a flat Vector', () => {
        expect(render(vec(num(1), num(2), num(3)))).toBe('[ 1/1 2/1 3/1 ]');
    });

    test('a nested Vector', () => {
        expect(render(vec(vec(num(1)), vec(num(2))))).toBe('[ [ 1/1 ] [ 2/1 ] ]');
    });

    test('the empty Vector is spaced, because a bracket must stand alone', () => {
        expect(render(vec())).toBe('[ ]');
    });
});
