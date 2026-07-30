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
const lines = language.split('\n').length;
if (lines > 400) fail(`language-semantics.md has ${lines} lines (maximum 400)`);

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
if (words.entries.length > 69) fail(`${words.entries.length} canonical Words (maximum 69)`);
if (aliases > 16) fail(`${aliases} aliases (maximum 16)`);

if (!process.exitCode) {
  console.log(
    `[semantic-kernel] ${lines} lines, ${clauseIds.size} clauses, ${familyIds.size} families, ${words.entries.length} Words, ${aliases} aliases.`,
  );
}
