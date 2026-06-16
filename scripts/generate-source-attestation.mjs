#!/usr/bin/env node
// Source provenance attestation (design: docs/dev/source-provenance-attestation-design.md).
//
// Content-addresses the trust-critical source surface of the language, the same
// way SPECIFICATION §8.6 content-addresses individual words — lifted from one
// word to the whole implementation. A backdoor changes file content, the file
// digest changes, the aggregated root identity changes, and `--check` (the
// drift guard wired into CI) fails. That drift is the injection tripwire.
//
// Security note: unlike the §8.6 word-identity polynomial digest (which is for
// cheap deterministic identity, not adversarial collision resistance), this uses
// SHA-256 via Node's built-in `node:crypto` — no new dependency — because
// provenance faces a deliberate attacker.

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const outputPath = resolve(repoRoot, 'docs/provenance/source-attestation.json');
const pinPath = resolve(repoRoot, 'docs/provenance/source-root.txt');

// --- Tracked surface -------------------------------------------------------
// Declared explicitly so the trusted base is auditable in review. Keep this in
// sync with docs/dev/source-provenance-attestation-design.md when it changes.
//
// The candidate set is enumerated from `git ls-files`, then narrowed to these
// roots. Hashing the git index blobs — rather than working-tree bytes — makes
// the attestation deterministic across environments: build
// steps that drop gitignored files (e.g. cargo creating rust/Cargo.lock, or
// node_modules) cannot perturb the root. A backdoor that adds a new source file
// must `git add` it for the build to use it, which also enrolls it here.
const TRACKED_DIR_PREFIXES = ['rust/src/', 'src/', 'src-tauri/src/', 'scripts/'];
const TRACKED_FILES = new Set([
  'SPECIFICATION.html',
  'package.json',
  'rust/Cargo.toml',
  'src-tauri/Cargo.toml',
  'src-tauri/tauri.conf.json',
  'vite.config.ts',
  'tsconfig.json',
  'eslint.config.js',
  '.github/workflows/build.yml',
  '.github/workflows/test.yml',
]);

// Path prefixes (repo-relative, posix) excluded because they are generated from
// tracked source (their integrity is covered by that source).
const EXCLUDE_PREFIXES = ['src/wasm/generated/'];

function fail(message) {
  console.error(`[provenance] ${message}`);
  process.exit(1);
}

function isTracked(relPosix) {
  if (EXCLUDE_PREFIXES.some((prefix) => relPosix.startsWith(prefix))) return false;
  if (TRACKED_FILES.has(relPosix)) return true;
  return TRACKED_DIR_PREFIXES.some((prefix) => relPosix.startsWith(prefix));
}

function sha256Hex(buffer) {
  return createHash('sha256').update(buffer).digest('hex');
}

function gitTrackedFiles() {
  // -z: NUL-separated, so paths with unusual characters stay intact.
  const out = execFileSync('git', ['-C', repoRoot, 'ls-files', '-z'], {
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  return out.split('\0').filter(Boolean);
}

function readGitIndexBlob(relPosix) {
  try {
    return execFileSync('git', ['-C', repoRoot, 'show', `:${relPosix}`], {
      encoding: 'buffer',
      maxBuffer: 64 * 1024 * 1024,
    });
  } catch {
    fail(`tracked file missing from git index: ${relPosix}`);
  }
}

function collectFiles() {
  const paths = gitTrackedFiles().filter(isTracked);
  // Deterministic order independent of git's enumeration.
  const unique = [...new Set(paths)].sort();
  return unique.map((relPosix) => {
    const bytes = readGitIndexBlob(relPosix);
    return { path: relPosix, sha256: sha256Hex(bytes), bytes: bytes.length };
  });
}

// Merkle-style root over the sorted (path, digest) list — the §8.6 idea of
// combining member digests in canonical order, here with SHA-256.
function computeRoot(files) {
  const h = createHash('sha256');
  for (const f of files) {
    h.update(f.path);
    h.update('\0');
    h.update(f.sha256);
    h.update('\n');
  }
  return `sha256:${h.digest('hex')}`;
}

function buildManifest() {
  const files = collectFiles();
  if (files.length === 0) fail('no tracked files collected');
  const rootIdentity = computeRoot(files);
  return {
    schemaVersion: 1,
    purpose:
      'Content-addressed provenance of the trust-critical source surface. A drift in rootIdentity is a backdoor-injection tripwire (docs/dev/source-provenance-attestation-design.md).',
    algorithm: 'sha256',
    rootIdentity,
    fileCount: files.length,
    enumeratedFrom: 'git ls-files with git index blob contents',
    trackedDirPrefixes: TRACKED_DIR_PREFIXES,
    trackedFiles: [...TRACKED_FILES].sort(),
    excludePrefixes: EXCLUDE_PREFIXES,
    files,
  };
}

function manifestJson(manifest) {
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

function pinContent(manifest) {
  return `${manifest.rootIdentity}\n`;
}

function ensureOutputDir() {
  const dir = dirname(outputPath);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
}

const manifest = buildManifest();
const json = manifestJson(manifest);
const pin = pinContent(manifest);

if (process.argv.includes('--check')) {
  // Drift guard / injection tripwire: fail if the committed attestation no
  // longer matches what the git index produces, or if the pin disagrees with
  // the recomputed root. See the design note for external anchoring of the pin.
  let failed = false;

  const existingManifest = existsSync(outputPath) ? readFileSync(outputPath, 'utf8') : '';
  if (existingManifest !== json) {
    console.error(
      '[provenance] docs/provenance/source-attestation.json is stale. ' +
        'Run `npm run provenance:attest` and commit the result.',
    );
    failed = true;
  }

  const existingPin = existsSync(pinPath) ? readFileSync(pinPath, 'utf8').trim() : '';
  if (existingPin !== manifest.rootIdentity) {
    console.error(
      `[provenance] root pin mismatch: docs/provenance/source-root.txt has "${existingPin || '(absent)'}" ` +
        `but the tracked source hashes to "${manifest.rootIdentity}". ` +
        'If the change is intentional, run `npm run provenance:attest` and commit; ' +
        'otherwise this is a tripwire for an unexpected source change.',
    );
    failed = true;
  }

  if (failed) process.exit(1);
  console.log(
    `[provenance] attestation up to date: ${manifest.fileCount} files, root ${manifest.rootIdentity}.`,
  );
} else if (process.argv.includes('--stdout')) {
  process.stdout.write(json);
} else {
  ensureOutputDir();
  writeFileSync(outputPath, json);
  writeFileSync(pinPath, pin);
  console.log(
    `[provenance] wrote ${manifest.fileCount} files to docs/provenance/source-attestation.json ` +
      `(root ${manifest.rootIdentity}).`,
  );
}
