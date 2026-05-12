import type { UserWord } from '../wasm-interpreter-types';

export const DEMO_WORDS_VERSION = 11;

// Phase 2 demo words exercise stack words, the Register, comparison, and logic.
// Conditional branching arrives in Phase 3; these demos stay branch-free.
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
        name: 'DEC',
        definition: '1 -',
        description: 'Decrement the top of the stack. Number -> Number',
    },
    {
        name: 'AVG2',
        definition: '+ 2 /',
        description: 'Average of the top two numbers. Number Number -> Number',
    },
    {
        name: 'ROT',
        definition: 'STORE SWAP RECALL SWAP',
        description: 'Rotate the top three: [a b c] -> [b c a]. Demonstrates the Register as a one-slot scratch.',
    },
    {
        name: 'POSITIVE?',
        definition: '0 >',
        description: 'Is the top strictly greater than 0? Number -> 1/0/Nil',
    },
    {
        name: 'BETWEEN?',
        definition: 'SWAP STORE SWAP PEEK LE SWAP RECALL SWAP LE AND',
        description: 'Is val within [low, high] (inclusive)? low val high -> 1/0/Nil. Avoids ROT because ROT uses Register.',
    },
];
