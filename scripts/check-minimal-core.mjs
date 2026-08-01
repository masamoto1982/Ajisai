#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const words = JSON.parse(readFileSync('spec/words.json', 'utf8')).entries;
const coverage = JSON.parse(readFileSync('docs/formalization-coverage.json', 'utf8'));
const wordNames = new Set(words.map((word) => word.name));
const entries = coverage.entries.filter((entry) => wordNames.has(entry.surface));
const primitives = new Set(coverage.algebra_primitives.map((entry) => entry.id));
const bySurface = new Map();
const errors = [];

for (const entry of entries) {
  if (bySurface.has(entry.surface)) errors.push(`duplicate Core Word witness: ${entry.surface}`);
  bySurface.set(entry.surface, entry);
}

for (const word of words) {
  const witness = bySurface.get(word.name);
  if (!witness) {
    errors.push(`${word.name}: missing Minimal Core witness`);
    continue;
  }
  if (!['Formalized', 'HostedEffect'].includes(witness.status)) errors.push(`${word.name}: status is ${witness.status}`);
  if (!witness.law_tests?.length) errors.push(`${word.name}: no executable law test`);
  for (const testPath of witness.law_tests ?? []) {
    if (!existsSync(testPath)) errors.push(`${word.name}: missing law test file ${testPath}`);
  }
  if (!['identity', 'flow', 'material'].includes(witness.core_tier)) {
    errors.push(`${word.name}: invalid Core tier ${witness.core_tier}`);
  }
  if (witness.semantic_role === 'Primitive') {
    if (!witness.primitive) errors.push(`${word.name}: Primitive role is not marked primitive`);
    if (!['identity', 'flow'].includes(witness.core_tier)) {
      errors.push(`${word.name}: primitive lies outside Minimal Core`);
    }
  } else if (witness.semantic_role === 'Derived') {
    if (witness.primitive) errors.push(`${word.name}: Derived role is marked primitive`);
    if (!witness.derived_from?.length) errors.push(`${word.name}: derived Word has no algebra basis`);
  } else if (witness.semantic_role !== 'HostedEffect') {
    errors.push(`${word.name}: unsupported semantic role ${witness.semantic_role}`);
  }
  for (const dependency of witness.derived_from ?? []) {
    if (!primitives.has(dependency)) errors.push(`${word.name}: unknown algebra primitive ${dependency}`);
  }
}

for (const surface of bySurface.keys()) {
  if (!words.some((word) => word.name === surface)) errors.push(`${surface}: witness has no canonical Word`);
}

if (bySurface.size !== words.length) {
  errors.push(`witness inventory has ${bySurface.size} entries; expected ${words.length}`);
}

if (errors.length) {
  errors.forEach((error) => console.error(`[minimal-core] ${error}`));
  process.exit(1);
}
console.log(`[minimal-core] ${words.length}/${words.length} Core Words have a formalized, executable primitive/derivation witness.`);
