import type { UserWord } from '../wasm-interpreter-types';


export const EXAMPLE_WORDS_VERSION = 11;

export const EXAMPLE_USER_WORDS: UserWord[] = [
    // Hello-World family: teaches how words depend on other words.
    // GREET is built purely by chaining the three SAY words, so editing or
    // deleting any of them ripples up to GREET through the dependency graph.
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
        definition: 'SAY-HELLO SAY-WORLD SAY-BANG',
    },
    // FizzBuzz: teaches COND. The value on the stack is offered to each guard
    // in turn; the first guard that leaves TRUE runs its body. The 15 guard
    // must come before 3 and 5 because a multiple of 15 also matches them.
    {
        name: 'FIZZBUZZ',
        definition:
            "{ [ 15 ] MOD [ 0 ] = } { 'FizzBuzz' PRINT } { [ 3 ] MOD [ 0 ] = } { 'Fizz' PRINT } { [ 5 ] MOD [ 0 ] = } { 'Buzz' PRINT } { TRUE } { ,, PRINT } COND",
    },
];
