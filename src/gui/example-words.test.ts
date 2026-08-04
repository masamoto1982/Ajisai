// The seeded Example Words are the first thing a fresh session shows, and they
// are the one place in the host that ships Ajisai source rather than reading it
// from the interpreter. Nothing type-checks them: `restore_user_words` only
// tokenizes a definition, so a body naming a word that no longer exists is
// defined without complaint and fails only when the user runs it.
//
// That is how every SAY word and GREET came to fail with "Unknown word: ,,"
// on first launch: `,,` was the symbol for KEEP, and it was retired when every
// symbol became one character. FIZZBUZZ's fallback branch carried the same
// token, so it errored on any input that was not a multiple of 3 or 5.
//
// These tests check the shape of every token the seed data ships, so retired
// residue is caught here instead of on someone's first run.

import { describe, expect, it } from 'vitest';
import { EXAMPLE_USER_WORDS } from './example-words';

// Symbols the lexer still accepts: one character each, plus the structural
// delimiters. Kept as data so a token that is no longer one of these is a
// visible diff rather than a silent runtime failure.
const SYMBOL_ALIASES = ['+', '-', '*', '/', '%', '=', '<', '>', '?', '^'];
const STRUCTURAL_TOKENS = ['[', ']', '{', '}', '|'];

// Two-character symbols and the force/negation marks, all retired.
const RETIRED_SYMBOLS = [',,', '<=', '>=', '<>', '&', '!', '..', '~'];

const WORD_NAME = /^[A-Z][A-Z0-9-]*$/;
// Integers, fractions and decimals; digits are required on both sides of a point.
const NUMBER_LITERAL = /^-?\d+(?:\/\d+|\.\d+)?$/;

// A string literal is opaque to this check: `'!'` is text, not the retired `!`.
const stripStringLiterals = (definition: string): string =>
    definition.replace(/'[^']*'/g, ' ');

const tokenize = (definition: string): string[] =>
    stripStringLiterals(definition).split(/\s+/).filter(Boolean);

const exampleWordNames = new Set(EXAMPLE_USER_WORDS.map(word => word.name));

describe('EXAMPLE_USER_WORDS', () => {
    it('ships a definition for every word', () => {
        for (const word of EXAMPLE_USER_WORDS) {
            expect(word.definition, `${word.name} has no definition`).toBeTruthy();
        }
    });

    it.each(EXAMPLE_USER_WORDS.map(w => [w.name, w.definition] as const))(
        '%s uses no retired symbol',
        (name, definition) => {
            const tokens = tokenize(definition ?? '');
            const retired = tokens.filter(token => RETIRED_SYMBOLS.includes(token));
            expect(retired, `${name} still uses retired symbol(s)`).toEqual([]);
        }
    );

    it.each(EXAMPLE_USER_WORDS.map(w => [w.name, w.definition] as const))(
        '%s is built only from tokens the lexer still accepts',
        (name, definition) => {
            const unrecognized = tokenize(definition ?? '').filter(token =>
                !SYMBOL_ALIASES.includes(token)
                && !STRUCTURAL_TOKENS.includes(token)
                && !NUMBER_LITERAL.test(token)
                && !WORD_NAME.test(token)
            );
            expect(unrecognized, `${name} uses token(s) of no known shape`).toEqual([]);
        }
    );

    it('only calls user words that are seeded alongside it', () => {
        // A word-shaped token is either a Core word or another Example Word.
        // Core names are not enumerated here, but a call to a *removed* example
        // (say GREET losing SAY-BANG) is caught: the dependency must be seeded.
        const greet = EXAMPLE_USER_WORDS.find(w => w.name === 'GREET');
        expect(greet).toBeDefined();
        for (const token of tokenize(greet!.definition ?? '')) {
            expect(exampleWordNames.has(token), `GREET calls unseeded ${token}`).toBe(true);
        }
    });
});
