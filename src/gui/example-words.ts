import type { UserWord } from '../wasm-interpreter-types';


export const EXAMPLE_WORDS_VERSION = 10;

export const EXAMPLE_USER_WORDS: UserWord[] = [
    {
        name: 'SAY-HELLO',
        definition: "'Hello' ,, PRINT",
    },
    {
        name: 'SAY-WORLD',
        definition: "'World' ,, PRINT",
    },
    {
        name: 'SAY-BANG',
        definition: "'!' ,, PRINT",
    },
    {
        name: 'GREET',
        definition: "{ [ 1 ] = } { SAY-HELLO } { [ 2 ] = } { SAY-WORLD } { IDLE } { SAY-BANG } COND",
    },
    {
        name: 'GREET-ALL',
        definition: '{ GREET } MAP',
    },
];
