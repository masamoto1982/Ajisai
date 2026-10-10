// What scripts/coverage-summary.mjs and scripts/check-coverage-ratchet.mjs
// both read: the Rust files the traceability matrix names as a requirement's
// implementation, and a `cargo llvm-cov report --json --summary-only` export
// keyed by repository-relative path. One reading of each, so the summary a
// reviewer sees and the gate CI enforces cannot disagree about which files
// are QL-A or what was measured for them.

import { readFileSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { readText, repoRoot } from './common.mjs';

/**
 * Rust implementation files of every requirement row in
 * docs/quality/TRACEABILITY_MATRIX.md, in matrix order:
 * `path -> { requirements: string[], level: 'QL-A' | ... }`.
 */
export function tracedRustFiles() {
  const traced = new Map();
  // Requirement rows: | **AQ-REQ-00N** | statement | implementation | verification | QL |
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
  return traced;
}

/**
 * An llvm-cov JSON export: `{ totals, files }`, with `files` a map from the
 * repository-relative path to that file's summary. Exits on anything else, so
 * a gate never passes for want of data.
 */
export function readCoverageExport(exportPath, tag) {
  const exported = JSON.parse(readFileSync(resolve(exportPath), 'utf8'));
  const data = exported.data?.[0];
  if (!data?.files || !data?.totals) {
    console.error(`[${tag}] ${exportPath} is not an llvm-cov JSON export (no data[0].files / totals)`);
    process.exit(1);
  }
  const files = new Map(
    data.files.map((file) => [relative(repoRoot, file.filename).split('\\').join('/'), file.summary]),
  );
  return { totals: data.totals, files };
}
