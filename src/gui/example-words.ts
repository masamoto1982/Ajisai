import type { UserWord } from '../wasm-interpreter-types';

// These four words exist for one demonstration: a Word button's border colour
// shows what a Word depends on, and that is only visible once one seeded word
// calls others. GREET over the three SAY words is the whole of it, so nothing
// else is seeded — a fresh dictionary is for the reader to fill.
export const EXAMPLE_USER_WORDS: UserWord[] = [
    // Hello-World family: teaches how words depend on other words.
    // GREET is built purely by chaining the three SAY words, so editing or
    // deleting any of them ripples up to GREET through the dependency graph.
    {
        name: 'SAY-HELLO',
        definition: "'Hello' PRINT",
    },
    {
        name: 'SAY-WORLD',
        definition: "'World' PRINT",
    },
    {
        name: 'SAY-BANG',
        definition: "'!' PRINT",
    },
    {
        name: 'GREET',
        definition: 'SAY-HELLO SAY-WORLD SAY-BANG',
    },
];
