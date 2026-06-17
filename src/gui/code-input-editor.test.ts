// Auto-indent helper for the code editor: the insertion produced when the
// user presses plain Enter must preserve the current line's indentation and
// deepen it after an opening bracket, without disturbing the input-assist
// (suggestion) path which owns Enter while the panel is open.

import { describe, expect, test } from 'vitest';
import { computeAutoIndentInsertion } from './code-input-editor';

describe('computeAutoIndentInsertion', () => {
    test('inserts a bare newline at the start of an empty buffer', () => {
        expect(computeAutoIndentInsertion('')).toBe('\n');
    });

    test('inserts a bare newline after an unindented line', () => {
        expect(computeAutoIndentInsertion('[ 1 2 3 ]')).toBe('\n');
    });

    test('carries over leading spaces of the current line', () => {
        expect(computeAutoIndentInsertion('  [ 1 ]')).toBe('\n  ');
    });

    test('carries over leading tabs of the current line', () => {
        expect(computeAutoIndentInsertion('\t\tSTEP-A')).toBe('\n\t\t');
    });

    test('uses only the current (last) line for indentation', () => {
        expect(computeAutoIndentInsertion('first\n    second')).toBe('\n    ');
    });

    test('adds one indent level for an unclosed square bracket', () => {
        expect(computeAutoIndentInsertion('[')).toBe('\n  ');
    });

    test('adds one indent level for an unclosed brace', () => {
        expect(computeAutoIndentInsertion('{')).toBe('\n  ');
    });

    test('adds one indent level for an unclosed paren', () => {
        expect(computeAutoIndentInsertion('(')).toBe('\n  ');
    });

    test('keeps depth for an unclosed bracket with content and trailing space', () => {
        expect(computeAutoIndentInsertion('  [   ')).toBe('\n    ');
    });

    test('does not deepen when an opening bracket is already closed', () => {
        expect(computeAutoIndentInsertion('  [ 1 ]')).toBe('\n  ');
    });

    test('combines existing indentation with the extra bracket level', () => {
        expect(computeAutoIndentInsertion('    {')).toBe('\n      ');
    });

    test('deepens by the net unclosed bracket count mid-line', () => {
        // { still open, both [ ] pairs balanced -> one extra level.
        expect(computeAutoIndentInsertion('{ [ 1 ] [ 2 ] +')).toBe('\n  ');
    });

    test('adds one level per still-open bracket', () => {
        // { and [ both open -> two extra levels.
        expect(computeAutoIndentInsertion('{ [ 1')).toBe('\n    ');
    });

    test('handles nested tensor opening brackets', () => {
        expect(computeAutoIndentInsertion('[[ 1 2')).toBe('\n    ');
    });

    test('returns to zero extra indent when all brackets close', () => {
        expect(computeAutoIndentInsertion('{ [ ] }')).toBe('\n');
    });

    test('ignores brackets inside single-quoted string literals', () => {
        expect(computeAutoIndentInsertion("'[ not code'")).toBe('\n');
    });

    test('does not go negative on a leading closing bracket', () => {
        // The ] closes an earlier line; the trailing [ leaves one open.
        expect(computeAutoIndentInsertion('  ] [')).toBe('\n    ');
    });
});
