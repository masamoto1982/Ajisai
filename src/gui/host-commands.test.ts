// `RESET` and `CLEAR` are answered by the host, not the language. The two rules
// that keep them from eating a program are what these tests pin: the name must
// be the whole input, and a User Word of that name wins.

import { describe, expect, test } from 'vitest';
import { resolveHostCommand } from './host-commands';

const noUserWords = () => false;
const userWords = (...names: string[]) => (name: string) => names.includes(name);

describe('resolveHostCommand', () => {
    test('recognizes the two host commands typed alone', () => {
        expect(resolveHostCommand('RESET', noUserWords)).toBe('RESET');
        expect(resolveHostCommand('CLEAR', noUserWords)).toBe('CLEAR');
    });

    test('matches case-insensitively, as Word lookup does', () => {
        expect(resolveHostCommand('clear', noUserWords)).toBe('CLEAR');
        expect(resolveHostCommand('  Reset  ', noUserWords)).toBe('RESET');
    });

    test('a name inside a program is not a host command', () => {
        expect(resolveHostCommand('1 CLEAR', noUserWords)).toBeNull();
        expect(resolveHostCommand('CLEAR CLEAR', noUserWords)).toBeNull();
        expect(resolveHostCommand("{ CLEAR } 'X' DEF", noUserWords)).toBeNull();
    });

    test('a User Word of the same name wins', () => {
        expect(resolveHostCommand('CLEAR', userWords('CLEAR'))).toBeNull();
        expect(resolveHostCommand('RESET', userWords('RESET'))).toBeNull();
        // …and only that name: an unrelated definition changes nothing.
        expect(resolveHostCommand('CLEAR', userWords('DBL'))).toBe('CLEAR');
    });

    test('ordinary programs and empty input are not host commands', () => {
        expect(resolveHostCommand('1 2 ADD', noUserWords)).toBeNull();
        expect(resolveHostCommand('', noUserWords)).toBeNull();
        expect(resolveHostCommand('   ', noUserWords)).toBeNull();
        expect(resolveHostCommand('CLEARED', noUserWords)).toBeNull();
    });
});
