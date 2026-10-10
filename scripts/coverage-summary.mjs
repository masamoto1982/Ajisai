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
// The summary only reports. The gate is scripts/check-coverage-ratchet.mjs,
// which fails CI when a QL-A file loses branch or line coverage against
// docs/quality/coverage-baseline.json; every other figure here is evidence
// for a reviewer, not a threshold.
//
// Usage: node scripts/coverage-summary.mjs <coverage.json>

import { readCoverageExport, tracedRustFiles } from './lib/coverage.mjs';

const exportPath = process.argv[2];
if (!exportPath) {
  console.error('usage: node scripts/coverage-summary.mjs <coverage.json>');
  process.exit(2);
}

const { totals: total, files: byPath } = readCoverageExport(exportPath, 'coverage-summary');
const traced = tracedRustFiles();

const pct = ({ count, covered }) => (count === 0 ? '—' : `${((100 * covered) / count).toFixed(1)}%`);
const cell = (metric) => `${pct(metric)} (${metric.count - metric.covered} missed)`;

const out = [];
out.push('## Rust coverage (`cargo llvm-cov --branch`, pinned nightly)', '');
out.push('QL-A files are gated: CI fails if one loses branch or line coverage against `docs/quality/coverage-baseline.json`. Every other figure is reported only.', '');
out.push('| Scope | Branches | Lines | Regions | Functions |', '|---|---|---|---|---|');
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
