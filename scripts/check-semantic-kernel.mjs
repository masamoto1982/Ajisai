import { readFileSync } from 'node:fs';

const language = readFileSync('spec/language-semantics.md', 'utf8');
const families = JSON.parse(readFileSync('spec/semantic-families.json', 'utf8'));
const legacy = JSON.parse(readFileSync('spec/legacy-clause-map.json', 'utf8'));
const manifest = JSON.parse(readFileSync('docs/word-manifest.json', 'utf8'));

const fail = (message) => {
  console.error(`[semantic-kernel] ${message}`);
  process.exitCode = 1;
};

const lines = language.split('\n').length;
if (lines < 350) fail(`language-semantics.md has ${lines} lines (compact-kernel target minimum 350)`);
if (lines > 500) fail(`language-semantics.md has ${lines} lines (maximum 500)`);

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

const archived = readFileSync(legacy.legacySnapshot, 'utf8');
const archivedAnchors = new Set([...archived.matchAll(/<h[2-5] id="([^"]+)"/g)].map((match) => match[1]));
const mappedAnchors = new Set(legacy.mappings.map((mapping) => mapping.legacyAnchor));
for (const anchor of archivedAnchors) {
  if (!mappedAnchors.has(anchor)) fail(`legacy heading is unmapped: ${anchor}`);
}
if (mappedAnchors.size !== archivedAnchors.size || legacy.headingCount !== archivedAnchors.size) {
  fail(`legacy map count differs: archive=${archivedAnchors.size}, map=${mappedAnchors.size}`);
}

if (manifest.counts.total !== 224) fail(`visible vocabulary changed to ${manifest.counts.total}`);

if (!process.exitCode) {
  console.log(`[semantic-kernel] ${lines} lines, ${clauseIds.size} clauses, ${familyIds.size} families, ${mappedAnchors.size} legacy headings, ${manifest.counts.total} surfaces.`);
}
