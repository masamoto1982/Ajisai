// Tests for the Ajisai source formatter. The formatter must tidy spacing and
// indentation while preserving meaning: line breaks (statement separators
// inside blocks) and the contents of strings and comments are never altered.

import { describe, expect, test } from 'vitest';
import { formatAjisaiSource } from './code-formatter';
import corpus from '../../tests/formatter-corpus.json';

// The shared corpus (tests/formatter-corpus.json) is the source of truth that
// pins this GUI formatter and the CLI formatter (rust/src/cli/fmt.rs) to the
// same behaviour, so the two implementations cannot drift (Phase 8A).
describe('formatAjisaiSource shared corpus', () => {
    for (const c of corpus.cases) {
        test(`corpus: ${c.name}`, () => {
            expect(formatAjisaiSource(c.input)).toBe(c.expected);
        });
    }
});

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

    test('pads the nil-coalesce marker', () => {
        // `^` is a special character in the lexer, so padding it cannot change
        // the token sequence. `~` is not: it belongs to the word around it, so
        // splitting `a~b` would turn one token into three.
        expect(formatAjisaiSource('a^c')).toBe('a ^ c');
        expect(formatAjisaiSource('a~b')).toBe('a~b');
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

    test('keeps a two-character comparison spelling intact', () => {
        // `>` is not split, so `>=` survives as one token rather than becoming
        // `> =` (GT followed by EQ).
        expect(formatAjisaiSource('[ 1 ] [ 3 ] >=')).toBe('[ 1 ] [ 3 ] >=');
        expect(formatAjisaiSource('[ 1 ] [ 3 ] <>')).toBe('[ 1 ] [ 3 ] <>');
    });

    test('formatting is idempotent on a multi-line block', () => {
        const messy = '{\n{ [ 5 ] >|[ \'big\' ] }\n} \'SIZE\' DEF';
        const once = formatAjisaiSource(messy);
        expect(formatAjisaiSource(once)).toBe(once);
    });
});

// The one layout rule Ajisai enforces is "one `|` clause per line": a COND
// written on one line does not run at all. Formatting it is therefore the
// formatter's most useful single action, and it used to be the one thing it
// declined to do.
describe('formatAjisaiSource COND clause splitting', () => {
    test('splits a one-line COND into one clause per line', () => {
        expect(formatAjisaiSource('{ { 2 LT | 1 * } { IDLE | 1 - FOO } COND } \'FOO\' DEF'))
            .toBe("{ { 2 LT | 1 * }\n  { IDLE | 1 - FOO } COND } 'FOO' DEF");
    });

    test('a single clause on a line is left alone', () => {
        const source = "5\n{ 3 LT | 'small' }\n{ IDLE | 'big' }\nCOND";
        expect(formatAjisaiSource(source)).toBe(source);
    });

    test('an ordinary block with no bar is not a clause and is not split', () => {
        expect(formatAjisaiSource('[ 1 2 ] { 1 * } MAP { 2 * } MAP'))
            .toBe('[ 1 2 ] { 1 * } MAP { 2 * } MAP');
    });

    test('clauses inside a word body are split, wherever they are nested', () => {
        // The rule counts every clause that *begins* on the physical line,
        // however deep, so the body's own clauses each get one.
        expect(formatAjisaiSource("{ { A | 1 } { B | 2 } COND } 'W' DEF"))
            .toBe("{ { A | 1 }\n  { B | 2 } COND } 'W' DEF");
    });
});
