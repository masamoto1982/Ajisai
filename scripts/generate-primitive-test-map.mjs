#!/usr/bin/env node
// Primitive -> test reverse index.
//
// `formalization-coverage.json` records, per word, which algebra primitives it
// `derived_from` and which `law_tests` / `conformance_cases` exercise it. That
// is a forward map (word -> primitives, word -> tests). This generator inverts
// it into a primitive -> tests map so each declared semantic primitive can be
// traced to the concrete tests that exercise the words resting on it. It turns
// the review's "few primitives, traceable derived words" goal into a queryable
// artifact (docs/primitive-test-map.json).
//
// Usage:
//   node scripts/generate-primitive-test-map.mjs            # write JSON artifact
//   node scripts/generate-primitive-test-map.mjs --stdout   # print JSON
//   node scripts/generate-primitive-test-map.mjs --markdown # print a table
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const coveragePath = resolve(repoRoot, 'docs/formalization-coverage.json');
const outputPath = resolve(repoRoot, 'docs/primitive-test-map.json');

function fail(message) {
  throw new Error(`[primitive-test-map] ${message}`);
}

const coverage = JSON.parse(readFileSync(coveragePath, 'utf8'));
if (!Array.isArray(coverage.algebra_primitives)) {
  fail('coverage has no algebra_primitives registry to invert');
}
if (!Array.isArray(coverage.entries)) fail('coverage entries must be an array');

const sorted = (set) => [...set].sort((a, b) => a.localeCompare(b));

// Registry order is the canonical order; preserve it so diffs stay stable.
const acc = new Map(
  coverage.algebra_primitives.map((primitive) => [
    primitive.id,
    {
      primitive,
      derivedWords: new Set(),
      lawTests: new Set(),
      conformanceCases: new Set(),
    },
  ]),
);

for (const entry of coverage.entries) {
  const refs = Array.isArray(entry.derived_from) ? entry.derived_from : [];
  for (const ref of refs) {
    const bucket = acc.get(ref);
    if (!bucket) continue; // unknown refs are the validator's job, not ours
    bucket.derivedWords.add(entry.id);
    for (const test of Array.isArray(entry.law_tests) ? entry.law_tests : []) {
      if (typeof test === 'string' && test.trim() !== '') bucket.lawTests.add(test);
    }
    for (const c of Array.isArray(entry.conformance_cases) ? entry.conformance_cases : []) {
      if (typeof c === 'string' && c.trim() !== '') bucket.conformanceCases.add(c);
    }
  }
}

const primitives = [...acc.values()].map(({ primitive, derivedWords, lawTests, conformanceCases }) => ({
  id: primitive.id,
  algebraic_family: primitive.algebraic_family,
  kind: primitive.kind,
  status: primitive.status,
  derived_word_count: derivedWords.size,
  law_test_count: lawTests.size,
  conformance_case_count: conformanceCases.size,
  derived_words: sorted(derivedWords),
  law_tests: sorted(lawTests),
  conformance_cases: sorted(conformanceCases),
}));

const untested = primitives.filter((p) => p.law_test_count === 0 && p.conformance_case_count === 0);
const allLawTests = new Set();
const allConformanceCases = new Set();
for (const p of primitives) {
  for (const t of p.law_tests) allLawTests.add(t);
  for (const c of p.conformance_cases) allConformanceCases.add(c);
}

const map = {
  version: 1,
  generated_from: ['docs/formalization-coverage.json'],
  summary: {
    primitives: primitives.length,
    primitives_without_tests: untested.length,
    distinct_law_test_files: allLawTests.size,
    distinct_conformance_cases: allConformanceCases.size,
  },
  primitives,
};

if (process.argv.includes('--markdown')) {
  const rows = primitives
    .map((p) => `| \`${p.id}\` | ${p.algebraic_family} | ${p.derived_word_count} | ${p.law_test_count} | ${p.conformance_case_count} |`)
    .join('\n');
  process.stdout.write(
    `# Primitive -> test reverse index\n\n` +
      `Generated from docs/formalization-coverage.json. ` +
      `${primitives.length} primitives, ${untested.length} without tests.\n\n` +
      `| Primitive | Family | Words | Law tests | Conformance cases |\n` +
      `| --- | --- | --- | --- | --- |\n${rows}\n`,
  );
} else if (process.argv.includes('--stdout')) {
  process.stdout.write(`${JSON.stringify(map, null, 2)}\n`);
} else {
  writeFileSync(outputPath, `${JSON.stringify(map, null, 2)}\n`);
  console.log(
    `[primitive-test-map] wrote ${primitives.length} primitives to docs/primitive-test-map.json` +
      (untested.length > 0 ? ` (${untested.length} without tests: ${untested.map((p) => p.id).join(', ')})` : ''),
  );
}
