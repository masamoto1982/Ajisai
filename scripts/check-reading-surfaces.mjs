// LANG.AUTHORITY.PRESENT — the vocabulary half of the clause.
//
// The README, the Reference, and the Specification describe the language as it
// currently is, and "every Word it names must exist in the vocabulary
// registry". This gate reads the registry rather than a hand-kept blocklist, so
// a Word deleted from spec/words.json can never be left behind in prose: the
// check fails the moment a reading surface names something the language does
// not have.
//
// It found `<code>ALGO</code>` ("SORT is owned by the ALGO module") surviving
// three manual review passes of the Reference, which is the case it exists for.

import { readFileSync } from 'node:fs';

const manifest = JSON.parse(readFileSync('docs/word-manifest.json', 'utf8'));

const known = new Set();
for (const entry of manifest.entries) {
  known.add(entry.canonical);
  // The manifest field is `surface` (singular). Reading a plural that no entry
  // has meant the registered *symbols* were never in this set — harmless while
  // only word-shaped tokens were checked, and wrong the moment they are not.
  known.add(entry.surface);
  for (const surface of entry.surfaces ?? []) known.add(surface);
}

// Names the documents introduce themselves: user-defined words and dictionaries
// from worked examples, and string contents that happen to be upper-case. These
// are the documents' own inventions, not vocabulary claims, so each is listed
// deliberately. A new example word is a one-line addition here.
const EXAMPLE_NAMES = new Set([
  // user words defined in the Reference's own examples
  'ADD10', 'GREET', 'APPLY-GAIN', 'SAY-HELLO', 'SAY-WORLD', 'SAY-BANG', 'FIZZBUZZ',
  // dictionaries and qualified paths from the resolution examples
  'EXAMPLE', 'AUDIOLIB', 'DICT@WORD', 'EXAMPLE@ADD10', 'EXAMPLE@GREET', 'AUDIOLIB@GREET',
  // literal string contents shown on the stack
  'TEST', "T'ES'T", 'AB', 'CD',
  // placeholder names, introduced by the Vector clause to show that `[ FOO ]`
  // holds the text FOO whether or not FOO is a defined Word
  'FOO', 'BAR',
  // a deliberate typo, introduced by the Defining Words clause to show that a
  // body's names resolve when the word is called rather than when it is defined
  'TOTALL',
  // user words defined by the block-frame and two-argument examples
  'ADDW', 'PSUM', 'PDIFF', 'PMAX',
  // the relaxation loop of the Iterative Numerics clause
  'DELTA', 'STEP',
  // the identity word the Stack clause uses to show KEEP copying any value
  'ID', 'DOUBLE',
  // binding names from the Naming a Value clause. These are locals, not Words:
  // the clause exists to say a binding is not a dictionary entry, so the
  // registry is correctly the wrong place to look for them.
  'W', 'B', 'A', 'T', 'LIMIT', 'DX', 'XS', 'K', 'I', 'YS',
  // binding names from the pattern clause, likewise locals rather than Words
  'P', 'M', 'D',
  // the seed and count of the RANDOM clause, written as names to show the shape
  'SEED', 'N',
]);

// Names the *host* allocates rather than the language: typed alone into the
// editor, they are acted on by the GUI and never reach the interpreter. The
// presentation profile has to be able to name them — a control nobody can
// discover is not documented — and they are correctly absent from the
// vocabulary registry, which is the whole point being made about them.
const HOST_COMMANDS = new Set(['RESET', 'CLEAR']);

const SURFACES = ['README.md', 'public/docs/index.html', 'SPECIFICATION.html'];

// Word-shaped: upper-case initial, then the characters an Ajisai name may use.
// Anything else inside <code> is a literal, a fragment, or punctuation.
const WORD_SHAPED = /^[A-Z][A-Z0-9@?!'-]*$/;

// Punctuation-shaped: a token of nothing but punctuation is a *symbol* claim.
// This half of the check did not exist, and the whole retired set — `,,`, `<=`,
// `>=`, `<>` and `&` — sat in the in-app Reference presenting itself as usable
// sugar while the gate reported success, because a symbol is not word-shaped.
const SYMBOL_SHAPED = /^[^A-Za-z0-9\s]+$/;

// Characters a document names in order to say the language does *not* allocate
// them, plus prose placeholders inside shape sketches. Each is a deliberate
// mention rather than a claim, so each is listed rather than inferred.
const UNALLOCATED_MENTIONS = new Set([
  // the decimal-literal clause names the point to say a lone `.` is a name
  '.',
  // `{ ... }` sketches a block's shape; the ellipsis stands for a body
  '...',
  // the two-argument clause names the classic stack operators precisely to say
  // Ajisai has none of them: several values travel in a Vector, not on the
  // stack. Naming them is the shortest way to answer the question a reader
  // arrives with, and each is a statement that the language does not allocate
  // it — the case this list exists for.
  'DUP', 'SWAP', 'DROP', 'ROT',
]);

const errors = [];

for (const path of SURFACES) {
  const source = readFileSync(path, 'utf8');
  const seen = new Map();

  // A backticked markdown link label is a repository path, not a program
  // token — [`LICENSE`](LICENSE) names a file. Drop those spans first.
  const prose = source.replace(/\[`[^`]+`\]\([^)]*\)/g, '');

  // Markdown backticks and HTML <code> mark a token as belonging to the
  // program, so both are vocabulary claims.
  const spans = [
    ...[...prose.matchAll(/<code>([^<]+)<\/code>/g)].map((m) => m[1]),
    ...[...prose.matchAll(/`([^`\n]+)`/g)].map((m) => m[1]),
  ];

  for (const span of spans) {
    // A span may hold a whole program (`2 SQRT 2 LT`), so check every token.
    // Whitespace is only one of the boundaries the lexer has: a structural
    // delimiter is always a token of its own, so `][` is `]` then `[` rather
    // than one unknown symbol, and `[FOO]` names FOO.
    const tokens = span
      .trim()
      .split(/\s+/)
      .flatMap((part) => part.split(/([[\]{}()])/))
      .filter(Boolean);
    for (const raw of tokens) {
      // Markdown escapes a table-cell pipe as `\|`; the token is the pipe.
      const token = raw.replace(/\\([|`*_])/g, '$1');
      if (WORD_SHAPED.test(token)) {
        if (known.has(token) || EXAMPLE_NAMES.has(token)) continue;
        if (HOST_COMMANDS.has(token)) continue;
        // A name may also be written only to say the language does not have it.
        // That was a symbol-only case until the Reference had to answer "where
        // is DUP?", which it answers by naming DUP.
        if (UNALLOCATED_MENTIONS.has(token)) continue;
        seen.set(token, (seen.get(token) ?? 0) + 1);
        continue;
      }
      if (!SYMBOL_SHAPED.test(token)) continue;
      // A token opening with a quote is a string literal, and its content is
      // not vocabulary — `''` is the empty string, not a symbol.
      if (token.startsWith("'")) continue;
      if (known.has(token) || UNALLOCATED_MENTIONS.has(token)) continue;
      seen.set(token, (seen.get(token) ?? 0) + 1);
    }
  }

  for (const [token, count] of [...seen].sort()) {
    errors.push(`${path} names ${token} (${count}x), which is not in the vocabulary registry`);
  }
}

if (errors.length) {
  for (const error of errors) console.error(`[reading-surfaces] ${error}`);
  console.error('[reading-surfaces] LANG.AUTHORITY.PRESENT: a reading surface may name only Words the language has.');
  console.error('[reading-surfaces] If the name is a new worked example, add it to EXAMPLE_NAMES in this script.');
  console.error('[reading-surfaces] If a name is written only to say the language does not allocate it, add it to UNALLOCATED_MENTIONS.');
  process.exitCode = 1;
} else {
  console.log(
    `[reading-surfaces] ${SURFACES.length} surfaces name only the ${manifest.entries.length} registered surfaces, ${EXAMPLE_NAMES.size} declared example names, ${HOST_COMMANDS.size} host commands and ${UNALLOCATED_MENTIONS.size} declared unallocated mentions.`,
  );
}
