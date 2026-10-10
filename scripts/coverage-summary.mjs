#!/usr/bin/env node
// Renders a `cargo llvm-cov report --json --summary-only` export as Markdown
// for the CI job summary: the workspace totals, then every Rust file the
// traceability matrix names as the implementation of a requirement, with the
// requirement and quality level beside it.
//
// The file list is read from docs/quality/TRACEABILITY_MATRIX.md rather than
// written down here, so a requirement row added or retargeted there shows up
// in the summary without touching this script.
//
// This reports; it does not gate. VERIFICATION_PLAN.md makes no coverage
// percentage a merge threshold, so the numbers are evidence for a reviewer —
// a figure that drops on a QL-A file is a question to ask, not a red check.
//
// Usage: node scripts/coverage-summary.mjs <coverage.json>

import { readFileSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { readText, repoRoot } from './lib/common.mjs';

const exportPath = process.argv[2];
if (!exportPath) {
  console.error('usage: node scripts/coverage-summary.mjs <coverage.json>');
  process.exit(2);
}

const exported = JSON.parse(readFileSync(resolve(exportPath), 'utf8'));
const data = exported.data?.[0];
if (!data?.files || !data?.totals) {
  console.error(`${exportPath} is not an llvm-cov JSON export (no data[0].files / totals)`);
  process.exit(1);
}

// Requirement rows: | **AQ-REQ-00N** | statement | implementation | verification | QL |
const traced = new Map();
for (const line of readText('docs/quality/TRACEABILITY_MATRIX.md').split('\n')) {
  const cells = line.split('|').map((cell) => cell.trim());
  const requirement = cells[1]?.match(/AQ-REQ-\d{3}/)?.[0];
  if (!requirement || cells.length < 6) continue;
  const level = cells[5];
  for (const [, path] of cells[3].matchAll(/`(rust\/src\/[^`]+\.rs)`/g)) {
    const entry = traced.get(path) ?? { requirements: [], level };
    entry.requirements.push(requirement);
    traced.set(path, entry);
  }
}

const byPath = new Map(
  data.files.map((file) => [relative(repoRoot, file.filename).split('\\').join('/'), file.summary]),
);

const pct = ({ count, covered }) => (count === 0 ? '—' : `${((100 * covered) / count).toFixed(1)}%`);
const cell = (metric) => `${pct(metric)} (${metric.count - metric.covered} missed)`;

const out = [];
out.push('## Rust coverage (`cargo +nightly llvm-cov --branch`)', '');
out.push('Reported, not gated: no coverage figure is a merge threshold (`docs/quality/VERIFICATION_PLAN.md`).', '');
out.push('| Scope | Branches | Lines | Regions | Functions |', '|---|---|---|---|---|');
const total = data.totals;
out.push(
  `| **Workspace** | ${cell(total.branches)} | ${cell(total.lines)} | ${cell(total.regions)} | ${pct(total.functions)} |`,
);
out.push('', '### Files implementing a traced requirement', '');
out.push('| File | Requirement | QL | Branches | Lines | Regions |', '|---|---|---|---|---|---|');
for (const [path, { requirements, level }] of traced) {
  const summary = byPath.get(path);
  const where = `\`${path.replace(/^rust\//, '')}\``;
  if (!summary) {
    out.push(`| ${where} | ${requirements.join(', ')} | ${level} | not in the native build (feature-gated) | | |`);
    continue;
  }
  out.push(
    `| ${where} | ${requirements.join(', ')} | ${level} | ${cell(summary.branches)} | ${cell(summary.lines)} | ${cell(summary.regions)} |`,
  );
}
out.push('');
console.log(out.join('\n'));
