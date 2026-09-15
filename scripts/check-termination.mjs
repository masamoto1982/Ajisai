#!/usr/bin/env node
// Gate for spec/termination.json, the canonical termination argument.
//
// The argument's weak point is not any single step of it — it is going stale.
// A new Word that evaluates a supplied code block adds a place where evaluation
// descends into more evaluation, and if nobody adds it to the argument, the
// argument quietly stops covering the language while still reading as though it
// does. That is the failure this gate exists to make impossible, and it uses a
// fact spec/words.json already records to do it: a Word whose `cost.steps.class`
// is `unbounded` is exactly a Word whose step count is decided by a block it is
// given rather than by itself. So:
//
//   every Word with unbounded step cost  <->  every Word-shaped recursion site
//
// Both directions. Adding such a Word without declaring the site fails here;
// declaring a site for a Word that does not evaluate a block fails here too.
//
// The rest is resolution: every clause, measure component and outcome category
// the argument names must exist. The behavioural half — that the witnesses
// actually behave as claimed, and that no ceiling is what stops them — is in
// rust/tests/termination_laws.rs, which runs them against the real engine.
//
// Usage:
//   node scripts/check-termination.mjs

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const read = (path) => JSON.parse(readFileSync(resolve(repoRoot, path), 'utf8'));

const failures = [];
const fail = (message) => failures.push(message);

const termination = read('spec/termination.json');
const words = read('spec/words.json').entries;
const outcomes = read('spec/outcomes.json');
const semantics = readFileSync(resolve(repoRoot, 'spec/language-semantics.md'), 'utf8');

const byName = new Map(words.map((w) => [w.name, w]));

// ------------------------------------------------------- site closure

const blockEvaluatingWords = new Set(
  words.filter((w) => w.cost?.steps?.class === 'unbounded').map((w) => w.name),
);
const declaredWordSites = new Set(
  termination.recursionSites.filter((s) => s.kind === 'word').map((s) => s.id),
);

for (const name of blockEvaluatingWords) {
  if (!declaredWordSites.has(name)) {
    fail(
      `${name} has cost.steps.class "unbounded" — its step count is decided by a block it is given — ` +
        'but spec/termination.json declares no recursion site for it, so the termination argument ' +
        'does not cover it',
    );
  }
}
for (const id of declaredWordSites) {
  if (!byName.has(id)) {
    fail(`recursion site "${id}" is declared with kind "word" but spec/words.json has no such Word`);
    continue;
  }
  if (!blockEvaluatingWords.has(id)) {
    fail(
      `recursion site "${id}" is declared with kind "word", but its cost.steps.class is ` +
        `"${byName.get(id).cost?.steps?.class}" rather than "unbounded" — a Word that does not ` +
        'evaluate a supplied block is not a place evaluation descends',
    );
  }
}

// ------------------------------------------------------- resolution

const componentIds = new Set(termination.measure.components.map((c) => c.id));
for (const site of termination.recursionSites) {
  if (!componentIds.has(site.decreases)) {
    fail(
      `recursion site "${site.id}" says it decreases "${site.decreases}", which the measure does not define`,
    );
  }
  for (const key of ['descendsInto', 'why', 'witness']) {
    if (typeof site[key] !== 'string' || site[key].trim() === '') {
      fail(`recursion site "${site.id}" is missing a non-empty ${key}`);
    }
  }
}
for (const id of componentIds) {
  if (!termination.recursionSites.some((s) => s.decreases === id)) {
    fail(
      `the measure declares component "${id}" that no recursion site decreases — a component ` +
        'nothing uses is not part of the argument',
    );
  }
}

for (const clause of termination.clauses) {
  if (!semantics.includes(clause)) {
    fail(`clauses names "${clause}", which spec/language-semantics.md does not define`);
  }
}

const errorCategories = new Set(outcomes.errorCategories.map((c) => c.id));
if (!errorCategories.has(termination.acyclicity.errorCategory)) {
  fail(
    `acyclicity names error category "${termination.acyclicity.errorCategory}", which ` +
      'spec/outcomes.json does not declare',
  );
}
for (const id of termination.ceilings.notLoadBearing) {
  if (!errorCategories.has(id)) {
    fail(`ceilings.notLoadBearing names "${id}", which spec/outcomes.json does not declare`);
  }
}

// A ceiling the argument calls not-load-bearing must be one that can reach any
// Word regardless of its contract — otherwise it is some Word's own declared
// condition and calling it a ceiling is a category error.
const machineCategories = new Set(
  outcomes.errorCategories
    .filter((c) => c.kind === 'structural' && c.attribution === 'machine')
    .map((c) => c.id),
);
for (const id of termination.ceilings.notLoadBearing) {
  if (errorCategories.has(id) && !machineCategories.has(id)) {
    fail(
      `ceilings.notLoadBearing names "${id}", but spec/outcomes.json does not attribute it to the ` +
        'machine — a ceiling is not a Word\'s own operand-level condition',
    );
  }
}

// ------------------------------------------------------- witnesses present

const witnessGroups = [
  ['invariant.witnesses.refused', termination.invariant.witnesses.refused],
  ['invariant.witnesses.accepted', termination.invariant.witnesses.accepted],
  ['acyclicity.witnesses.refused', termination.acyclicity.witnesses.refused],
  ['acyclicity.witnesses.accepted', termination.acyclicity.witnesses.accepted],
];
for (const [label, group] of witnessGroups) {
  if (!Array.isArray(group) || group.length === 0) {
    fail(`${label} declares no witness — a claim with no witness is not checkable`);
    continue;
  }
  for (const [i, w] of group.entries()) {
    if (typeof w.source !== 'string' || w.source.trim() === '') {
      fail(`${label}[${i}] has no source program`);
    }
  }
}
for (const w of termination.invariant.witnesses.refused) {
  if (typeof w.expect !== 'string' || !w.expect.startsWith('error:')) {
    fail(`invariant refused witness ${JSON.stringify(w.source)} must declare an error: outcome`);
  } else if (!errorCategories.has(w.expect.slice(6))) {
    fail(`invariant refused witness expects "${w.expect}", which spec/outcomes.json does not declare`);
  }
}

// ------------------------------------------------------------- report

if (failures.length > 0) {
  for (const message of failures) console.error(`[termination] ${message}`);
  console.error(`[termination] ${failures.length} failure(s)`);
  process.exit(1);
}

console.log(
  `[termination] ${termination.recursionSites.length} recursion sites (${declaredWordSites.size} Words, ` +
    `closed both ways against unbounded step cost), ${componentIds.size} measure components, ` +
    `${witnessGroups.reduce((n, [, g]) => n + g.length, 0)} witnesses declared.`,
);
