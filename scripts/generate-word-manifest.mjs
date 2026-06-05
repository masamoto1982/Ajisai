#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const outputPath = resolve(repoRoot, 'docs/word-manifest.json');

function fail(message) {
  throw new Error(`[word-manifest] ${message}`);
}

function readRepo(path) {
  return readFileSync(resolve(repoRoot, path), 'utf8');
}

function constArrayBody(source, constName) {
  const startPattern = new RegExp(`(?:pub\\([^)]*\\)\\s+)?(?:pub\\s+)?const\\s+${constName}[^=]*=\\s*&\\[`);
  const start = source.search(startPattern);
  if (start < 0) fail(`could not find const array ${constName}`);
  const open = source.indexOf('[', source.indexOf('&[', start));
  const end = source.indexOf('\n];', open);
  if (end < 0) fail(`could not find end of const array ${constName}`);
  return source.slice(open + 1, end);
}

function slug(value) {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

function symbolSlug(value) {
  const names = {
    '+': 'plus',
    '-': 'minus',
    '*': 'asterisk',
    '/': 'slash',
    '%': 'percent',
    '=': 'equals',
    '<': 'less-than',
    '<=': 'less-than-or-equal',
    '>': 'greater-than',
    '>=': 'greater-than-or-equal',
    '<>': 'not-equal',
    '!': 'bang',
    '&': 'ampersand',
    '.': 'dot',
    '..': 'dot-dot',
    ',': 'comma',
    ',,': 'comma-comma',
    "'": 'quote',
    '?': 'question',
    '==': 'equals-equals',
    '=>': 'equals-greater',
    '#': 'hash',
    '$': 'dollar',
    '[': 'left-bracket',
    ']': 'right-bracket',
    '{': 'left-brace',
    '}': 'right-brace',
    ';': 'semicolon',
    ';;': 'semicolon-semicolon',
    '(': 'left-paren',
    ')': 'right-paren',
  };
  return names[value] ?? slug(value);
}

function rustEnumVariantToSnake(value) {
  return value.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase();
}

function extractCoreWords() {
  const sourcePath = 'rust/src/builtins/builtin_word_definitions.rs';
  const body = constArrayBody(readRepo(sourcePath), 'BUILTIN_SPECS');
  const entries = [];
  const pattern = /BuiltinSpec\s*{([\s\S]*?)(?=\n\s*BuiltinSpec\s*{|\n\s*\];)/g;
  for (const match of body.matchAll(pattern)) {
    const item = match[1];
    const name = item.match(/\bname:\s*"([^"]+)"/)?.[1];
    const category = item.match(/\bcategory:\s*"([^"]+)"/)?.[1];
    if (!name || !category) continue;
    entries.push({
      id: `core.${slug(name)}`,
      kind: 'coreword',
      surface: name,
      category,
      source: sourcePath,
    });
  }
  if (entries.length === 0) fail('no core words extracted');
  return entries;
}

function extractModuleWords() {
  const sourcePath = 'rust/src/interpreter/modules/module_builtins.rs';
  const source = readRepo(sourcePath);
  const moduleSpecsBody = constArrayBody(source, 'MODULE_SPECS');
  const wordsConstToModule = new Map();
  for (const match of moduleSpecsBody.matchAll(/ModuleSpec\s*{\s*name:\s*"([^"]+)"\s*,\s*words:\s*([A-Z_]+)_WORDS\s*,/g)) {
    wordsConstToModule.set(`${match[2]}_WORDS`, match[1]);
  }
  if (wordsConstToModule.size === 0) fail('no module specs extracted');

  const entries = [];
  for (const [wordsConst, moduleName] of wordsConstToModule) {
    const body = constArrayBody(source, wordsConst);
    for (const match of body.matchAll(/module_word!\(\s*"([^"]+)"/g)) {
      const shortName = match[1];
      const coverageAliases = {
        'TIME@NOW': ['NOW'],
        'CRYPTO@RANDOM': ['RANDOM'],
        'CRYPTO@CSPRNG': ['RANDOM'],
        'MATH@SQRT': [`'math' IMPORT SQRT`],
        'SERIAL@LIST-PORTS': ['SERIAL-*'],
        'SERIAL@OPEN': ['SERIAL-*'],
        'SERIAL@CONFIGURE': ['SERIAL-*'],
        'SERIAL@WRITE': ['SERIAL-*'],
        'SERIAL@READ': ['SERIAL-*'],
        'SERIAL@FLUSH': ['SERIAL-*'],
        'SERIAL@CLOSE': ['SERIAL-*'],
      }[`${moduleName}@${shortName}`];
      entries.push({
        id: `module.${slug(moduleName)}.${slug(shortName)}`,
        kind: 'moduleword',
        surface: `${moduleName}@${shortName}`,
        ...(coverageAliases ? { coverage_aliases: coverageAliases } : {}),
        short_surface: shortName,
        module: moduleName,
        category: moduleName.toLowerCase(),
        source: sourcePath,
      });
    }
  }
  if (entries.length === 0) fail('no module words extracted');
  return entries;
}

function extractAliases() {
  const sourcePath = 'rust/src/core_word_aliases.rs';
  const body = constArrayBody(readRepo(sourcePath), 'CORE_WORD_ALIASES');
  const entries = [];
  const pattern = /CoreWordAlias\s*{([\s\S]*?)(?=\n\s*CoreWordAlias\s*{|\n\s*\];)/g;
  for (const match of body.matchAll(pattern)) {
    const item = match[1];
    const alias = item.match(/\balias:\s*"([^"]+)"/)?.[1];
    const canonicalMatch = item.match(/\bcanonical:\s*(Some\("([^"]+)"\)|None)/);
    const kind = item.match(/\bkind:\s*CoreWordAliasKind::([A-Za-z0-9_]+)/)?.[1];
    if (!alias || !canonicalMatch || !kind) continue;
    entries.push({
      id: `alias.${symbolSlug(alias)}`,
      kind: rustEnumVariantToSnake(kind),
      surface: alias,
      canonical: canonicalMatch[2] ?? null,
      source: sourcePath,
    });
  }
  if (entries.length === 0) fail('no aliases extracted');
  return entries;
}

function extractSurfaceForms() {
  const sourcePath = 'rust/src/surface_forms.rs';
  const body = constArrayBody(readRepo(sourcePath), 'SURFACE_FORMS');
  const entries = [];
  const pattern = /SurfaceForm\s*{([\s\S]*?)(?=\n\s*SurfaceForm\s*{|\n\s*\];)/g;
  for (const match of body.matchAll(pattern)) {
    const item = match[1];
    const surface = item.match(/\bsurface:\s*"([^"]+)"/)?.[1];
    const concept = item.match(/\bconcept:\s*"([^"]+)"/)?.[1];
    const kind = item.match(/\bkind:\s*SurfaceFormKind::([A-Za-z0-9_]+)/)?.[1];
    const runtimeWord = item.match(/\bruntime_word:\s*(true|false)/)?.[1];
    if (!surface || !concept || !kind || !runtimeWord) continue;
    entries.push({
      id: `surface.${symbolSlug(surface)}`,
      kind: rustEnumVariantToSnake(kind),
      surface,
      concept,
      runtime_word: runtimeWord === 'true',
      source: sourcePath,
    });
  }
  if (entries.length === 0) fail('no surface forms extracted');
  return entries;
}

const entries = [
  ...extractCoreWords(),
  ...extractModuleWords(),
  ...extractAliases(),
  ...extractSurfaceForms(),
];

const seen = new Set();
for (const entry of entries) {
  if (seen.has(entry.id)) fail(`duplicate manifest id ${entry.id}`);
  seen.add(entry.id);
}

const manifest = {
  schemaVersion: 1,
  generatedFrom: [
    'rust/src/builtins/builtin_word_definitions.rs',
    'rust/src/interpreter/modules/module_builtins.rs',
    'rust/src/core_word_aliases.rs',
    'rust/src/surface_forms.rs',
  ],
  counts: {
    corewords: entries.filter((entry) => entry.kind === 'coreword').length,
    modulewords: entries.filter((entry) => entry.kind === 'moduleword').length,
    aliases: entries.filter((entry) => ['symbol_alias', 'syntax_sugar', 'input_helper'].includes(entry.kind)).length,
    surface_forms: entries.filter((entry) => !['coreword', 'moduleword', 'symbol_alias', 'syntax_sugar', 'input_helper'].includes(entry.kind)).length,
    total: entries.length,
  },
  entries,
};

const json = `${JSON.stringify(manifest, null, 2)}\n`;
if (process.argv.includes('--stdout')) {
  process.stdout.write(json);
} else {
  writeFileSync(outputPath, json);
  console.log(`[word-manifest] wrote ${entries.length} entries to docs/word-manifest.json`);
}
