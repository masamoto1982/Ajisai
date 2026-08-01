#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';

const legacyDirectory = 'rust/src/builtins/builtin_specs';
const forbidden = [
  ['rust/src/builtins/mod.rs', /\bmod\s+builtin_specs\s*;/, 'legacy builtin_specs module declaration'],
  ['rust/src/builtins', /\bRuntimeSpec\b/, 'parallel RuntimeSpec metadata'],
  ['rust/src/builtins', /\bSPEC_DEFAULT\b/, 'parallel RuntimeSpec default'],
];
const errors = [];

function rustFiles(path) {
  if (!existsSync(path)) return [];
  return readdirSync(path, { withFileTypes: true }).flatMap((entry) => {
    const child = join(path, entry.name);
    if (entry.isDirectory()) return rustFiles(child);
    return entry.isFile() && child.endsWith('.rs') ? [child] : [];
  });
}

if (existsSync(legacyDirectory)) {
  errors.push(`${legacyDirectory}: retired authored metadata directory must not exist`);
}

for (const [path, pattern, label] of forbidden) {
  const files = path.endsWith('.rs') ? [path] : rustFiles(path);
  for (const file of files) {
    if (pattern.test(readFileSync(file, 'utf8'))) errors.push(`${file}: contains ${label}`);
  }
}

if (errors.length) {
  errors.forEach((error) => console.error(`[runtime-metadata] ${error}`));
  process.exit(1);
}

console.log('[runtime-metadata] canonical generated metadata is the only builtin metadata source.');
