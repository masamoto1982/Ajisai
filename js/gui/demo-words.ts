import type { UserWord } from '../wasm-interpreter-types';


export const DEMO_WORDS_VERSION = 9;

export const DEMO_USER_WORDS: UserWord[] = [
    {
        name: 'SAY-HELLO',
        definition: "'Hello' ,, PRINT",
        description: 'Map: Print "Hello" while passing input through. Any -> Any',
    },
    {
        name: 'SAY-WORLD',
        definition: "'World' ,, PRINT",
        description: 'Map: Print "World" while passing input through. Any -> Any',
    },
    {
        name: 'SAY-BANG',
        definition: "'!' ,, PRINT",
        description: 'Map: Print "!" while passing input through. Any -> Any',
    },
    {
        name: 'GREET',
        definition: "{ [ 1 ] = } { SAY-HELLO } { [ 2 ] = } { SAY-WORLD } { IDLE } { SAY-BANG } COND",
        description: 'Form: Print Hello/World/! by value (1/2/other). Scalar -> Scalar',
    },
    {
        name: 'GREET-ALL',
        definition: '{ GREET } MAP',
        description: 'Form: Apply GREET to each element. Vector -> Vector',
    },
];
