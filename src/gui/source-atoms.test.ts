// The GUI's one scanner of Ajisai source, checked against the rules of
// rust/src/tokenizer.rs: whitespace is the sole delimiter, and `#` and `'` are
// special only at the start of an atom.

import { describe, expect, test } from 'vitest';
import {
    countLeadingSourceWhitespace,
    isSourceWhitespace,
    scanAtoms,
    trimSource,
    trimSourceEnd
} from './source-atoms';

const texts = (source: string): string[] => scanAtoms(source).map((a) => a.text);

describe('scanAtoms', () => {
    test('splits words on whitespace of any kind', () => {
        expect(texts('1 \t2\n\nADD')).toEqual(['1', '2', 'ADD']);
    });

    test('a word runs to whitespace and nothing else splits it', () => {
        expect(texts('[1 2]')).toEqual(['[1', '2]']);
        expect(texts('a^b >= x')).toEqual(['a^b', '>=', 'x']);
    });

    test('a hash at the start of an atom is a comment to end of line', () => {
        const atoms = scanAtoms('1 # two 3\n4');
        expect(atoms.map((a) => [a.kind, a.text])).toEqual([
            ['word', '1'],
            ['comment', '# two 3'],
            ['word', '4']
        ]);
    });

    test('a hash inside a word is part of the word', () => {
        expect(scanAtoms('a#b 1').map((a) => [a.kind, a.text])).toEqual([
            ['word', 'a#b'],
            ['word', '1']
        ]);
    });

    test('a string holds whitespace and closes at a quote before whitespace', () => {
        expect(texts("'hello world' PRINT")).toEqual(["'hello world'", 'PRINT']);
        expect(texts("'it''s' x")).toEqual(["'it''s'", 'x']);
    });

    test('a quote followed by anything but whitespace does not close', () => {
        const [atom] = scanAtoms("'foo'[1] 2");
        expect(atom).toMatchObject({ kind: 'string', text: "'foo'[1] 2", closed: false });
    });

    test('a quote inside a word is part of the word', () => {
        expect(scanAtoms("a'b c").map((a) => [a.kind, a.text])).toEqual([
            ['word', "a'b"],
            ['word', 'c']
        ]);
    });

    test('an unclosed string runs to the end and says so', () => {
        const atoms = scanAtoms("1 'oops");
        expect(atoms[1]).toEqual({ kind: 'string', text: "'oops", start: 2, end: 7, closed: false });
    });

    test('every atom is exactly the source it points at', () => {
        const source = "[ 1 ] # c\n'a b' x#y";
        for (const atom of scanAtoms(source)) {
            expect(source.slice(atom.start, atom.end)).toBe(atom.text);
        }
    });
});

// spec/grammar.json, characterClasses.whitespace: Unicode White_Space,
// enumerated because host predicates disagree at the edges — ECMAScript's `\s`
// counts U+FEFF as whitespace and U+0085 as not, and the grammar says the
// opposite of both.
describe('isSourceWhitespace', () => {
    test('is the grammar\'s enumerated class', () => {
        for (const ws of ['\t', '\n', '\v', '\f', '\r', ' ', '\u0085', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', '　']) {
            expect(isSourceWhitespace(ws)).toBe(true);
        }
        for (const name of ['﻿', 'a', '1', '[', '​', '、']) {
            expect(isSourceWhitespace(name)).toBe(false);
        }
    });

    test('a byte-order mark glues to the first word, as the grammar\'s note says', () => {
        expect(texts('﻿1 2 ADD')).toEqual(['﻿1', '2', 'ADD']);
    });

    test('U+0085 separates words and closes a string, as any whitespace does', () => {
        expect(texts('1\u00852')).toEqual(['1', '2']);
        expect(texts("'a'\u0085b")).toEqual(["'a'", 'b']);
    });
});

describe('trimSource', () => {
    test('strips the grammar\'s whitespace from both ends', () => {
        expect(trimSource(' \n　 1 2 ADD\u0085\t')).toBe('1 2 ADD');
        expect(trimSource(' \n ')).toBe('');
        expect(trimSource('')).toBe('');
    });

    test('keeps a byte-order mark, which is part of the first word', () => {
        expect(trimSource(' ﻿1 2 ADD ')).toBe('﻿1 2 ADD');
        expect(countLeadingSourceWhitespace(' ﻿1')).toBe(1);
    });

    test('trimSourceEnd strips the end only', () => {
        expect(trimSourceEnd('  # a comment \r')).toBe('  # a comment');
    });
});
