#!/usr/bin/env node
// QL-A / QL-B coverage ratchet: no file the traceability matrix names as the
// implementation of a QL-A or QL-B requirement may lose branch or line
// coverage. (The matrix has no QL-C or QL-D rows; a gated level added to
// GATED_LEVELS is all it takes to cover one.)
//
// docs/quality/coverage-baseline.json records, per gated file, the branches
// and lines covered out of those counted. A run fails when a file's covered
// fraction falls below its recorded one — compared exactly, as
// covered·baseCount < baseCovered·count, so no rounding decides a verdict.
// It is a ratchet, not a fixed threshold: new code in a gated file must be
// covered at least as well as the file already was, and a file that gets
// better can be locked in with --update, in the same change, where a
// reviewer sees the baseline move.
//
// The baseline must match the matrix in both directions: a gated file with no
// baseline entry fails (a new requirement row needs a recorded floor), and an
// entry for a file that is no longer a gated implementation fails (a floor
// must not outlive its subject). A gated file the native build does not
// compile (feature-gated, such as the wasm bindings) is skipped and named;
// its verification is the suite that builds it.
//
// The figures are only comparable under the toolchain and inputs that
// produced the baseline: CI pins the nightly named in the baseline's
// `toolchain` and fixes proptest's seed (PROPTEST_RNG_SEED), so the same tree
// measures the same counts on every run.
//
// Usage:
//   node scripts/check-coverage-ratchet.mjs <coverage.json>            check
//   node scripts/check-coverage-ratchet.mjs <coverage.json> --update   re-record

import { existsSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { readJson, repoRoot, reporter } from './lib/common.mjs';
import { readCoverageExport, tracedRustFiles } from './lib/coverage.mjs';

const TAG = 'coverage-ratchet';
const baselinePath = 'docs/quality/coverage-baseline.json';
const METRICS = ['branches', 'lines'];
const TOOLCHAIN = 'nightly-2026-10-09';
const GATED_LEVELS = new Set(['QL-A', 'QL-B']);

const args = process.argv.slice(2);
const update = args.includes('--update');
const exportPath = args.find((arg) => !arg.startsWith('--'));
if (!exportPath) {
  console.error('usage: node scripts/check-coverage-ratchet.mjs <coverage.json> [--update]');
  process.exit(2);
}

const report = reporter(TAG);
const { files } = readCoverageExport(exportPath, TAG);
const gated = [...tracedRustFiles()]
  .filter(([, { level }]) => GATED_LEVELS.has(level))
  .map(([path]) => path);
const measured = gated.filter((path) => files.has(path));
const unbuilt = gated.filter((path) => !files.has(path));

const pick = (summary) =>
  Object.fromEntries(METRICS.map((m) => [m, { covered: summary[m].covered, count: summary[m].count }]));
const pct = ({ covered, count }) => (count === 0 ? '100.0%' : `${((100 * covered) / count).toFixed(1)}%`);

if (update) {
  const recorded = {
    _comment: [
      'Per-file branch and line coverage floor for every Rust file the traceability',
      'matrix names as the implementation of a QL-A or QL-B requirement. CI fails when a',
      "file's covered fraction drops below its entry (scripts/check-coverage-ratchet.mjs).",
      'Measured with `cargo +<toolchain> llvm-cov --branch --workspace` and',
      'PROPTEST_RNG_SEED fixed, as the Rust branch coverage step in',
      '.github/workflows/test.yml runs it. Re-record, in the change that moves a',
      'figure, with: node scripts/check-coverage-ratchet.mjs <coverage.json> --update',
    ],
    toolchain: TOOLCHAIN,
    files: Object.fromEntries(measured.map((path) => [path, pick(files.get(path))])),
  };
  writeFileSync(resolve(repoRoot, baselinePath), `${JSON.stringify(recorded, null, 2)}\n`);
  console.log(`[${TAG}] recorded ${measured.length} QL-A/QL-B file(s) in ${baselinePath}`);
  process.exit(0);
}

if (!existsSync(resolve(repoRoot, baselinePath))) {
  report.fatal(`${baselinePath} is missing; record it with --update`);
}
const baseline = readJson(baselinePath);
if (baseline.toolchain !== TOOLCHAIN) {
  report.fail(
    `${baselinePath} was recorded with ${baseline.toolchain}, but this gate measures with ${TOOLCHAIN}; ` +
      're-record the baseline under the new toolchain in the change that moves it',
  );
}

const improved = [];
for (const path of measured) {
  const base = baseline.files?.[path];
  if (!base) {
    report.fail(`${path} implements a QL-A or QL-B requirement but has no entry in ${baselinePath}; record one with --update`);
    continue;
  }
  const now = pick(files.get(path));
  for (const metric of METRICS) {
    const [n, b] = [now[metric], base[metric]];
    // n.covered / n.count < b.covered / b.count, without division.
    const nCount = n.count || 1;
    const bCount = b.count || 1;
    const nCovered = n.count ? n.covered : 1;
    const bCovered = b.count ? b.covered : 1;
    if (nCovered * bCount < bCovered * nCount) {
      report.fail(
        `${path}: ${metric} coverage fell from ${pct(b)} (${b.covered}/${b.count}) to ${pct(n)} (${n.covered}/${n.count})`,
      );
    } else if (nCovered * bCount > bCovered * nCount) {
      improved.push(`${path} ${metric} ${pct(b)} -> ${pct(n)}`);
    }
  }
}
for (const path of Object.keys(baseline.files ?? {})) {
  if (!measured.includes(path)) {
    report.fail(`${baselinePath} has an entry for ${path}, which is not a measured QL-A or QL-B implementation file; remove it with --update`);
  }
}

for (const path of unbuilt) console.log(`[${TAG}] skipped ${path}: not in the native build`);
for (const line of improved) console.log(`[${TAG}] improved: ${line} (lock it in with --update)`);
report.done(`${measured.length} QL-A/QL-B file(s) at or above their recorded branch and line coverage`);
