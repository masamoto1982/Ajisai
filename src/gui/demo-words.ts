import type { UserWord } from '../wasm-interpreter-types';

export const DEMO_WORDS_VERSION = 10;

// Phase 1 demo words exercise stack words and continued-fraction arithmetic.
export const DEMO_USER_WORDS: UserWord[] = [
    {
        name: 'DOUBLE',
        definition: 'DUP +',
        description: 'Double the top of the stack. Number -> Number',
    },
    {
        name: 'SQUARE',
        definition: 'DUP *',
        description: 'Square the top of the stack. Number -> Number',
    },
    {
        name: 'INC',
        definition: '1 +',
        description: 'Increment the top of the stack. Number -> Number',
    },
    {
        name: 'AVG2',
        definition: '+ 2 /',
        description: 'Average of the top two numbers. Number Number -> Number',
    },
];
