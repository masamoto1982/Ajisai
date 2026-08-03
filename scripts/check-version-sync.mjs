#!/usr/bin/env node
// The beta ships one version number. `package.json` is the source of truth and
// every other manifest must repeat it exactly: the npm package, the
// `ajisai-core` crate (which `ajisai version` prints through
// `CARGO_PKG_VERSION`), the Tauri crate, and the Tauri application config.
//
// Distribution metadata that drifts is worse than no metadata: a bug report
// naming a version has to identify one build. This gate keeps them equal
// without a generator, so a release bump is one edit plus this check.
import { readFileSync } from 'node:fs';

const readVersion = (path, pattern) => {
  const source = readFileSync(path, 'utf8');
  const match = source.match(pattern);
  return match ? match[1] : null;
};

const expected = JSON.parse(readFileSync('package.json', 'utf8')).version;
// Cargo and npm both take SemVer, so a prerelease reads the same in both.
const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

const sources = [
  ['package.json', expected],
  ['rust/Cargo.toml', readVersion('rust/Cargo.toml', /^version = "([^"]+)"/m)],
  ['src-tauri/Cargo.toml', readVersion('src-tauri/Cargo.toml', /^version = "([^"]+)"/m)],
  ['src-tauri/tauri.conf.json', JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version],
];

const errors = [];
if (!SEMVER.test(expected)) errors.push(`package.json version ${expected} is not SemVer`);
for (const [path, version] of sources) {
  if (version === null) errors.push(`${path}: no version found`);
  else if (version !== expected) errors.push(`${path}: version ${version}; expected ${expected}`);
}

if (errors.length) {
  errors.forEach((error) => console.error(`[version-sync] ${error}`));
  process.exit(1);
}
console.log(`[version-sync] ${sources.length}/${sources.length} manifests declare ${expected}.`);
