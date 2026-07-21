#!/usr/bin/env node
// Verify that every repo file path referenced in backticks in the quality
// traceability matrix actually exists. This catches drift like a renamed or
// removed source file leaving a dangling reference in the matrix (the matrix is
// a machine-checkable audit surface, so a stale path silently weakens it).
//
// Only backtick spans that look like a repo file path are checked: a token with
// a `/` and a known file extension, optionally followed by a Rust item path
// (`::module::test_name`), which is stripped before the existence check. Code
// snippets and identifiers in backticks (no `/`, or no file extension) are
// ignored.

import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const matrixPath = resolve(repoRoot, 'docs/quality/TRACEABILITY_MATRIX.md');

const FILE_EXT = /\.(rs|mjs|js|ts|md|json|html|toml|ya?ml|sh)$/;

function extractPaths(markdown) {
  const paths = new Set();
  for (const match of markdown.matchAll(/`([^`]+)`/g)) {
    // Strip a Rust item path suffix (`file.rs::mod::test`) to the file path.
    const token = match[1].split('::')[0].trim();
    if (token.includes('/') && FILE_EXT.test(token)) {
      paths.add(token);
    }
  }
  return [...paths];
}

function main() {
  if (!existsSync(matrixPath)) {
    console.error(`[traceability] matrix not found at ${matrixPath}`);
    process.exit(1);
  }
  const markdown = readFileSync(matrixPath, 'utf8');
  const paths = extractPaths(markdown);
  const missing = paths.filter((p) => !existsSync(resolve(repoRoot, p)));

  if (missing.length > 0) {
    console.error(`[traceability] ${missing.length} referenced path(s) do not exist:`);
    for (const p of missing.sort()) console.error(`  - ${p}`);
    console.error('Fix the reference in docs/quality/TRACEABILITY_MATRIX.md to a real path.');
    process.exit(1);
  }

  console.log(`[traceability] ${paths.length} referenced file path(s) all exist.`);
}

main();
