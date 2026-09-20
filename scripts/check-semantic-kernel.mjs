import { readFileSync } from 'node:fs';

const language = readFileSync('spec/language-semantics.md', 'utf8');
const families = JSON.parse(readFileSync('spec/semantic-families.json', 'utf8'));
const words = JSON.parse(readFileSync('spec/words.json', 'utf8'));

const fail = (message) => {
  console.error(`[semantic-kernel] ${message}`);
  process.exitCode = 1;
};

// The kernel is a ceiling, not a floor: a shorter specification is always an
// improvement, a longer one is the regression this gate exists to catch.
//
// The budget is normally slack, and then one day it is not. Adding
// LANG.SOURCE.FRAME took the file to the ceiling exactly, so the next clause
// added here fails this check on arrival. That is the gate working: the answer
// is to shorten or merge an existing clause, which is the question "does the
// specification need this, or does it already say it elsewhere?" asked at the
// only moment anyone will actually ask it.
//
// Two answers that look like fixes are not. Raising the number right after
// hitting it retires the brake; and reflowing prose to put more words on fewer
// lines keeps the count down while the specification grows, which is the thing
// being measured. Raising it deliberately, when the language genuinely has more
// to say, is a different act and a fine one — it just wants to be a decision
// rather than a reflex.
//
// `headroom` below is reported on every green run, so how close the file is
// sitting is visible without reading this comment.
//
// Raised to 404 for LANG.DICTIONARY.ACYCLIC: the User dictionary's reference
// graph became acyclic by rule (no definition may name itself, directly or
// through a chain of other User words), retiring the guarded-tail-recursion
// feature and its own clause text in LANG.SOURCE.FRAME. Net new content, not
// reflowed old content — the language now has something it did not have
// before (every evaluation is structurally finite, not merely
// resource-bounded) — so this is the deliberate raise the comment above asks
// for, not a reflex.
//
// Raised to 408 for LANG.RECORDS.STRUCTURE: the vocabulary-100 work order
// (docs/dev/vocabulary-100-work-order-2026-09.md §2.3) admitted a seventh
// value domain, the keyed correspondence, and a domain needs its own clause —
// its structure, its identity, its containment rule — where the earlier
// phases of that work order fit into existing clauses. Four lines: the
// heading, the paragraph, and their separators. Net new content again.
const LINE_BUDGET = 408;
const lines = language.split('\n').length;
if (lines > LINE_BUDGET) {
  fail(
    `language-semantics.md has ${lines} lines (maximum ${LINE_BUDGET}). ` +
      'Shorten or merge a clause rather than raising the budget; see the note above this check.',
  );
}

const clauseIds = new Set([...language.matchAll(/id="[^"]+">(LANG\.[A-Z.]+)/g)].map((match) => match[1]));
if (clauseIds.size === 0) fail('no language clause IDs found');

const familyIds = new Set();
for (const family of families.families) {
  if (familyIds.has(family.id)) fail(`duplicate semantic family: ${family.id}`);
  familyIds.add(family.id);
  for (const clause of family.clauses) {
    if (!clauseIds.has(clause)) fail(`family ${family.id} references missing clause ${clause}`);
  }
}
if (familyIds.size > 12) fail(`${familyIds.size} semantic families (maximum 12)`);

const names = new Set();
for (const word of words.entries) {
  if (names.has(word.name)) fail(`duplicate Word: ${word.name}`);
  names.add(word.name);
  if (!familyIds.has(word.family)) fail(`Word ${word.name} references missing family ${word.family}`);
  for (const clause of word.clauses) {
    if (!clauseIds.has(clause)) fail(`Word ${word.name} references missing clause ${clause}`);
  }
}
for (const family of familyIds) {
  if (![...words.entries].some((word) => word.family === family)) fail(`semantic family ${family} has no Words`);
}

// Vocabulary growth is the failure mode this project is guarding against, so the
// count is a budget rather than a fixed inventory: shrinking is free, growing is
// a deliberate specification change.
const aliases = words.entries.reduce((total, word) => total + word.aliases.length, 0);
// Raised from 70 to 100 for the vocabulary-100 work order
// (docs/dev/vocabulary-100-work-order-2026-09.md): the owner's decision to
// remake the vocabulary as ten concepts and 100 Words, each admitted on one of
// two grounds the work order states (inexpressible in a total, non-recursive
// language, or the closure of a small symmetric family). The number is the
// work order's ceiling, not a target — its §1 forbids padding to reach it —
// so this is the deliberate raise the comment above asks for.
if (words.entries.length > 100) fail(`${words.entries.length} canonical Words (maximum 100)`);
if (aliases > 16) fail(`${aliases} aliases (maximum 16)`);

if (!process.exitCode) {
  const headroom = LINE_BUDGET - lines;
  const budget = headroom === 0 ? 'at the line budget' : `${headroom} lines under budget`;
  console.log(
    `[semantic-kernel] ${lines} lines (${budget}), ${clauseIds.size} clauses, ${familyIds.size} families, ${words.entries.length} Words, ${aliases} aliases.`,
  );
}
