// `RESET`, `STACK-CLEAR`, `EDITOR-CLEAR` and a lookup are answered by the
// host, not the language. The two rules that keep them from eating a program
// are what these tests pin: the request must be the whole input, and a User
// Word of that name wins.

import { describe, expect, test } from 'vitest';
import { resolveHostCommand } from './host-commands';

const noUserWords = () => false;
const userWords = (...names: string[]) => (name: string) => names.includes(name);

describe('resolveHostCommand', () => {
    test('recognizes the three bare host commands typed alone', () => {
        expect(resolveHostCommand('RESET', noUserWords)).toEqual({ kind: 'RESET' });
        expect(resolveHostCommand('STACK-CLEAR', noUserWords)).toEqual({ kind: 'STACK-CLEAR' });
        expect(resolveHostCommand('EDITOR-CLEAR', noUserWords)).toEqual({ kind: 'EDITOR-CLEAR' });
    });

    test('matches case-insensitively, as Word lookup does', () => {
        expect(resolveHostCommand('stack-clear', noUserWords)).toEqual({ kind: 'STACK-CLEAR' });
        expect(resolveHostCommand('editor-clear', noUserWords)).toEqual({ kind: 'EDITOR-CLEAR' });
        expect(resolveHostCommand('  Reset  ', noUserWords)).toEqual({ kind: 'RESET' });
    });

    test('a name inside a program is not a host command', () => {
        expect(resolveHostCommand('1 STACK-CLEAR', noUserWords)).toBeNull();
        expect(resolveHostCommand('STACK-CLEAR STACK-CLEAR', noUserWords)).toBeNull();
        expect(resolveHostCommand("{ EDITOR-CLEAR } 'X' DEF", noUserWords)).toBeNull();
    });

    test('a User Word of the same name wins', () => {
        expect(resolveHostCommand('STACK-CLEAR', userWords('STACK-CLEAR'))).toBeNull();
        expect(resolveHostCommand('EDITOR-CLEAR', userWords('EDITOR-CLEAR'))).toBeNull();
        expect(resolveHostCommand('RESET', userWords('RESET'))).toBeNull();
        // …and only that name: an unrelated definition changes nothing.
        expect(resolveHostCommand('STACK-CLEAR', userWords('DBL'))).toEqual({ kind: 'STACK-CLEAR' });
    });

    // ── Looking a Word up ─────────────────────────────────────────────────
    // This was the Word `LOOKUP`, whose whole result was a side channel to the
    // editor. The spellings people already know still work.

    test('reads both spellings of a lookup, quoted as the reference teaches', () => {
        expect(resolveHostCommand("'ADD' ?", noUserWords)).toEqual({
            kind: 'LOOKUP',
            name: 'ADD'
        });
        expect(resolveHostCommand("'ADD' LOOKUP", noUserWords)).toEqual({
            kind: 'LOOKUP',
            name: 'ADD'
        });
        expect(resolveHostCommand("  'ADD'   lookup  ", noUserWords)).toEqual({
            kind: 'LOOKUP',
            name: 'ADD'
        });
    });

    test('reads a bare name, the quotes having been a language construct', () => {
        expect(resolveHostCommand('ADD ?', noUserWords)).toEqual({
            kind: 'LOOKUP',
            name: 'ADD'
        });
        // A name that ends in `?` is a name, not a mark: the two are separated
        // by whitespace, so `NIL?` survives intact.
        expect(resolveHostCommand('NIL? ?', noUserWords)).toEqual({
            kind: 'LOOKUP',
            name: 'NIL?'
        });
    });

    test('the name looked up is never treated as a definition to shadow', () => {
        // Only the *mark* can be shadowed. Asking about `STACK-CLEAR` asks
        // about `STACK-CLEAR`, whether or not the session defines it.
        expect(resolveHostCommand("'STACK-CLEAR' ?", userWords('STACK-CLEAR'))).toEqual({
            kind: 'LOOKUP',
            name: 'STACK-CLEAR'
        });
        expect(resolveHostCommand("'ADD' ?", userWords('?'))).toBeNull();
        expect(resolveHostCommand("'ADD' LOOKUP", userWords('LOOKUP'))).toBeNull();
    });

    test('a lookup must be the whole input, like every other host command', () => {
        expect(resolveHostCommand("1 'ADD' ?", noUserWords)).toBeNull();
        expect(resolveHostCommand("'ADD' ? 2 MUL", noUserWords)).toBeNull();
        expect(resolveHostCommand("{ 'ADD' ? } 'X' DEF", noUserWords)).toBeNull();
    });

    test('a lookup with no name is not a lookup', () => {
        expect(resolveHostCommand('?', noUserWords)).toBeNull();
        expect(resolveHostCommand('LOOKUP', noUserWords)).toBeNull();
        expect(resolveHostCommand("'' ?", noUserWords)).toBeNull();
        expect(resolveHostCommand("'   ' ?", noUserWords)).toBeNull();
    });

    test('ordinary programs and empty input are not host commands', () => {
        expect(resolveHostCommand('1 2 ADD', noUserWords)).toBeNull();
        expect(resolveHostCommand('', noUserWords)).toBeNull();
        expect(resolveHostCommand('   ', noUserWords)).toBeNull();
        expect(resolveHostCommand('CLEARED', noUserWords)).toBeNull();
    });
});
