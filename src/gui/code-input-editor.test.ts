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

    test('adds one indent level after a trailing opening square bracket', () => {
        expect(computeAutoIndentInsertion('[')).toBe('\n  ');
    });

    test('adds one indent level after a trailing opening brace', () => {
        expect(computeAutoIndentInsertion('{')).toBe('\n  ');
    });

    test('adds one indent level after a trailing opening paren', () => {
        expect(computeAutoIndentInsertion('(')).toBe('\n  ');
    });

    test('ignores trailing whitespace when detecting an opening bracket', () => {
        expect(computeAutoIndentInsertion('  [   ')).toBe('\n    ');
    });

    test('does not deepen when an opening bracket is already closed', () => {
        expect(computeAutoIndentInsertion('  [ 1 ]')).toBe('\n  ');
    });

    test('combines existing indentation with the extra bracket level', () => {
        expect(computeAutoIndentInsertion('    {')).toBe('\n      ');
    });
});
