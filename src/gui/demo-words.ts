import type { UserWord } from '../wasm-interpreter-types';


export const DEMO_WORDS_VERSION = 9;

export const DEMO_USER_WORDS: UserWord[] = [
    {
        name: 'SAY-HELLO',
        definition: "'Hello' ,, PRINT",
        description: 'Print "Hello" while passing input through. Any -> Any',
    },
    {
        name: 'SAY-WORLD',
        definition: "'World' ,, PRINT",
        description: 'Print "World" while passing input through. Any -> Any',
    },
    {
        name: 'SAY-BANG',
        definition: "'!' ,, PRINT",
        description: 'Print "!" while passing input through. Any -> Any',
    },
    {
        name: 'GREET',
        definition: "{ [ 1 ] = } { SAY-HELLO } { [ 2 ] = } { SAY-WORLD } { IDLE } { SAY-BANG } COND",
        description: 'Print Hello/World/! by value (1/2/other). Scalar -> Scalar',
    },
    {
        name: 'GREET-ALL',
        definition: '{ GREET } MAP',
        description: 'Apply GREET to each element. Vector -> Vector',
    },
];
