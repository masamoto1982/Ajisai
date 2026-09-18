import type { UserWord } from '../wasm-interpreter-types';

// `KEEP` is spelled out. It used to have the symbol `,,`, which every one of
// these definitions was written against; once every symbol became one
// character `,,` stopped being a name the dictionary holds, so the seeded
// words defined fine (a body is only tokenized at DEF time) and then failed
// with "Unknown word: ,," the moment they ran.
export const EXAMPLE_USER_WORDS: UserWord[] = [
    // Hello-World family: teaches how words depend on other words.
    // GREET is built purely by chaining the three SAY words, so editing or
    // deleting any of them ripples up to GREET through the dependency graph.
    {
        name: 'SAY-HELLO',
        definition: "'Hello' KEEP PRINT",
    },
    {
        name: 'SAY-WORLD',
        definition: "'World' KEEP PRINT",
    },
    {
        name: 'SAY-BANG',
        definition: "'!' KEEP PRINT",
    },
    {
        name: 'GREET',
        definition: 'SAY-HELLO SAY-WORLD SAY-BANG',
    },
    // FizzBuzz: teaches SELECT. Each SELECT chooses between two values that
    // already exist, so the three tests are written as nested choices rather
    // than as a run of clauses tried in order: the 15 case is the outermost
    // one because it is the one that wins, and that precedence is visible as
    // the shape of the phrase instead of as a rule about reading order.
    //
    // `N 3 MOD 1 LT` is "divisible by 3": the remainder of a non-negative
    // dividend is 0, 1 or 2, so "less than 1" is "equal to 0" without needing
    // an element-wise equality.
    {
        name: 'FIZZBUZZ',
        definition:
            "'N' BIND 'FizzBuzz' 'Fizz' 'Buzz' N STR N 5 MOD 1 LT SELECT N 3 MOD 1 LT SELECT N 15 MOD 1 LT SELECT",
        // Unlike GREET and the SAY-* words above, FIZZBUZZ is not runnable on
        // its own: it names the value beneath it, so running it with an empty
        // stack is a Stack underflow, not a bug. This is the same
        // `#:contract` directive text a user could write above their own `DEF`
        // to get this same hover (see `execute_def::
        // extract_pending_word_descriptions`), pre-filled here since a
        // restored Example Word never passes through source `execute()`.
        description: '( 1 -- 1 ) observable may-nil — push the number to test first, e.g. 9 FIZZBUZZ',
    },
    // CLAMP-LOW: teaches that the choice is made lane by lane. One SELECT
    // rewrites every negative element of a whole Vector, with no MAP and no
    // block — the mask comes from a comparison that lifts the same way.
    {
        name: 'CLAMP-LOW',
        definition: "'V' BIND [ 0 ] V V [ 0 ] LT SELECT",
        description: '( 1 -- 1 ) pure — push the vector first, e.g. [ -3 5 -1 ] CLAMP-LOW',
    },
];
