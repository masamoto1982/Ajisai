#!/usr/bin/env node
// Machine-readable Core Profile coverage of the conformance suite:
// the share of Core-classified surface words (docs/word-manifest.json)
// that appear in at least one `ajisai-source` program of
// tests/conformance/index.html.
//
// Usage:
//   node scripts/check-conformance-coverage.mjs            # human summary
//   node scripts/check-conformance-coverage.mjs --json     # JSON report
//   node scripts/check-conformance-coverage.mjs --suite F  # alternate suite file
//
// Sugar and alias surfaces are folded onto their canonical Core word using
// the alias entries of the word manifest (e.g. `+` counts as ADD, `^` as
// VENT, `~` as FLOW, `;`/`;;` as their modifier pairs), so a case written
// in sugar still covers the canonical word.

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const args = process.argv.slice(2);
const asJson = args.includes('--json');
const suiteFlag = args.indexOf('--suite');
const suitePath = suiteFlag !== -1 && args[suiteFlag + 1]
  ? resolve(args[suiteFlag + 1])
  : resolve(repoRoot, 'tests/conformance/index.html');

const manifest = JSON.parse(
  readFileSync(resolve(repoRoot, 'docs/word-manifest.json'), 'utf8'),
);

const coreWords = new Set(
  manifest.entries
    .filter((e) => e.classification === 'Core')
    .map((e) => e.surface),
);

// surface -> canonical word name(s), from the manifest's alias entries plus
// the control/modifier sugar the tokenizer folds before dictionary lookup.
const sugarMap = new Map();
for (const e of manifest.entries) {
  if (e.kind === 'symbol_alias' || e.kind === 'syntax_sugar') {
    sugarMap.set(e.surface, [e.canonical]);
  }
}
sugarMap.set('^', ['VENT']);
sugarMap.set(';', ['TOP', 'EAT']);
sugarMap.set(';;', ['STAK', 'KEEP']);

function decodeEntities(value) {
  return value
    .replaceAll('&lt;', '<')
    .replaceAll('&gt;', '>')
    .replaceAll('&quot;', '"')
    .replaceAll('&#39;', "'")
    .replaceAll('&apos;', "'")
    .replaceAll('&amp;', '&');
}

const html = readFileSync(suitePath, 'utf8');
const sources = [];
const srcPattern = /<pre class="ajisai-source">([\s\S]*?)<\/pre>/g;
for (const m of html.matchAll(srcPattern)) {
  sources.push(decodeEntities(m[1]));
}
if (sources.length === 0) {
  console.error(`no ajisai-source programs found in ${suitePath}`);
  process.exit(2);
}

const seen = new Set();
for (const src of sources) {
  for (let tok of src.split(/\s+/)) {
    if (!tok) continue;
    // fused modifier sugar: ';;ADD' covers STAK KEEP ADD, ';ADD' TOP EAT ADD
    if (tok.startsWith(';;') && tok.length > 2) {
      seen.add('STAK'); seen.add('KEEP');
      tok = tok.slice(2);
    } else if (tok.startsWith(';') && tok.length > 1) {
      seen.add('TOP'); seen.add('EAT');
      tok = tok.slice(1);
    }
    // The surface itself may be Core-classified (e.g. `/`), so record both
    // the raw token and its canonical fold.
    seen.add(tok);
    if (sugarMap.has(tok)) {
      for (const canon of sugarMap.get(tok)) seen.add(canon);
    }
  }
}

const covered = [...coreWords].filter((w) => seen.has(w)).sort();
const missing = [...coreWords].filter((w) => !seen.has(w)).sort();
const pct = (100 * covered.length) / coreWords.size;

if (asJson) {
  process.stdout.write(`${JSON.stringify({
    schemaVersion: 1,
    suite: suitePath,
    caseSources: sources.length,
    coreWords: coreWords.size,
    covered: covered.length,
    coveragePercent: Number(pct.toFixed(1)),
    missing,
  }, null, 2)}\n`);
} else {
  console.log(`conformance suite: ${sources.length} case sources`);
  console.log(`Core Profile words: ${coreWords.size}`);
  console.log(`covered: ${covered.length} (${pct.toFixed(1)}%)`);
  if (missing.length) {
    console.log(`missing (${missing.length}): ${missing.join(' ')}`);
  } else {
    console.log('missing: none');
  }
}
