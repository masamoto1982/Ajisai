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
]);

const SURFACES = ['README.md', 'public/docs/index.html', 'SPECIFICATION.html'];

// Word-shaped: upper-case initial, then the characters an Ajisai name may use.
// Anything else inside <code> is a literal, a fragment, or punctuation.
const WORD_SHAPED = /^[A-Z][A-Z0-9@?!'-]*$/;

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
    for (const token of span.trim().split(/\s+/)) {
      if (!WORD_SHAPED.test(token)) continue;
      if (known.has(token) || EXAMPLE_NAMES.has(token)) continue;
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
  process.exitCode = 1;
} else {
  console.log(
    `[reading-surfaces] ${SURFACES.length} surfaces name only the ${manifest.entries.length} registered surfaces and ${EXAMPLE_NAMES.size} declared example names.`,
  );
}
