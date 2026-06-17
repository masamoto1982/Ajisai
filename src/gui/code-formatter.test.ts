// Tests for the Ajisai source formatter. The formatter must tidy spacing and
// indentation while preserving meaning: line breaks (statement separators
// inside blocks) and the contents of strings and comments are never altered.

import { describe, expect, test } from 'vitest';
import { formatAjisaiSource } from './code-formatter';

describe('formatAjisaiSource', () => {
    test('returns empty string unchanged', () => {
        expect(formatAjisaiSource('')).toBe('');
    });

    test('pads the inside of vector brackets', () => {
        expect(formatAjisaiSource('[1 2 3]')).toBe('[ 1 2 3 ]');
    });

    test('separates nested brackets into standalone tokens', () => {
        expect(formatAjisaiSource('[[1 2][3 4]]')).toBe('[ [ 1 2 ] [ 3 4 ] ]');
    });

    test('collapses runs of spaces between tokens', () => {
        expect(formatAjisaiSource('[ 1    2   3 ]')).toBe('[ 1 2 3 ]');
    });

    test('trims leading and trailing whitespace on a line', () => {
        expect(formatAjisaiSource('   [ 1 ] PRINT   ')).toBe('[ 1 ] PRINT');
    });

    test('splits brackets that are glued to adjacent words', () => {
        expect(formatAjisaiSource('[1 2 3]PRINT')).toBe('[ 1 2 3 ] PRINT');
    });

    test('pads the pipeline and nil-coalesce markers', () => {
        expect(formatAjisaiSource('a~b^c')).toBe('a ~ b ^ c');
    });

    test('is idempotent on already-canonical input', () => {
        const canonical = '{ [ 1 ] [ 2 ] + } \'ADD12\' DEF';
        expect(formatAjisaiSource(canonical)).toBe(canonical);
    });

    test('keeps the contents of a string literal verbatim', () => {
        expect(formatAjisaiSource("[ 'a  b   c' ]")).toBe("[ 'a  b   c' ]");
    });

    test('does not pad brackets that live inside a string', () => {
        expect(formatAjisaiSource("'[not code]'")).toBe("'[not code]'");
    });

    test('keeps comment text (including spacing) verbatim', () => {
        expect(formatAjisaiSource('[ 1 ]   #   keep   spacing'))
            .toBe('[ 1 ] #   keep   spacing');
    });

    test('preserves a comment-only line', () => {
        expect(formatAjisaiSource('# ===== header ====='))
            .toBe('# ===== header =====');
    });

    test('preserves significant line breaks between statements', () => {
        const input = '[1]PRINT\n[2]PRINT';
        expect(formatAjisaiSource(input)).toBe('[ 1 ] PRINT\n[ 2 ] PRINT');
    });

    test('indents the body of a multi-line block and dedents its close', () => {
        const input = [
            '{',
            '{ [ 5 ] > | [ \'big\' ] }',
            '{ IDLE   | [ \'small\' ] } COND',
            '} \'SIZE\' DEF',
        ].join('\n');
        const expected = [
            '{',
            '  { [ 5 ] > | [ \'big\' ] }',
            '  { IDLE | [ \'small\' ] } COND',
            '} \'SIZE\' DEF',
        ].join('\n');
        expect(formatAjisaiSource(input)).toBe(expected);
    });

    test('indents nested multi-line blocks by depth', () => {
        const input = '{\n{\n[ 1 ]\n}\n}';
        const expected = '{\n  {\n    [ 1 ]\n  }\n}';
        expect(formatAjisaiSource(input)).toBe(expected);
    });

    test('collapses multiple blank lines and trims surrounding blanks', () => {
        const input = '\n\n[ 1 ]\n\n\n[ 2 ]\n\n';
        expect(formatAjisaiSource(input)).toBe('[ 1 ]\n\n[ 2 ]');
    });

    test('leaves an unterminated string untouched', () => {
        const input = "[ 'oops ]";
        expect(formatAjisaiSource(input)).toBe(input);
    });

    test('leaves a newline inside a string untouched', () => {
        const input = "'line one\nline two'";
        expect(formatAjisaiSource(input)).toBe(input);
    });

    test('does not expand the ; modifier sugar', () => {
        expect(formatAjisaiSource('[ 1 ] ;')).toBe('[ 1 ] ;');
    });

    test('keeps a conversion word such as >CF intact', () => {
        expect(formatAjisaiSource('[ 1 ] [ 3 ] / >CF')).toBe('[ 1 ] [ 3 ] / >CF');
    });

    test('formatting is idempotent on a multi-line block', () => {
        const messy = '{\n{ [ 5 ] >|[ \'big\' ] }\n} \'SIZE\' DEF';
        const once = formatAjisaiSource(messy);
        expect(formatAjisaiSource(once)).toBe(once);
    });
});
