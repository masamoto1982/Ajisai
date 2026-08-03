#!/usr/bin/env node
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const outputPath = resolve(repoRoot, 'docs/word-manifest.json');
const coveragePath = resolve(repoRoot, 'docs/formalization-coverage.json');
const contractsPath = resolve(repoRoot, 'spec/words.json');

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
    '~': 'tilde',
    '^': 'caret',
    '#': 'hash',
    '|': 'pipe',
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


function normalizeSurface(value) {
  return typeof value === 'string' ? value.trim().toUpperCase() : '';
}

function coverageSurfaces(entry) {
  const surfaces = [];
  if (typeof entry.surface === 'string') surfaces.push(entry.surface);
  if (Array.isArray(entry.surfaces)) surfaces.push(...entry.surfaces.filter((value) => typeof value === 'string'));
  return surfaces;
}

function manifestEntryMatchesCoverage(entry, coverageEntry) {
  if (entry.id === coverageEntry.id) return true;
  const manifestSurface = normalizeSurface(entry.surface);
  const aliases = Array.isArray(entry.coverage_aliases)
    ? entry.coverage_aliases.map(normalizeSurface).filter(Boolean)
    : [];
  for (const surface of coverageSurfaces(coverageEntry)) {
    const coverageSurface = normalizeSurface(surface);
    if (coverageSurface === manifestSurface || aliases.includes(coverageSurface)) return true;
    const tokens = coverageSurface.match(/[A-Z0-9@?>=<!&+*/%.,;#$'\[\]{}()-]+/g) ?? [];
    if (tokens.includes(manifestSurface) || aliases.some((alias) => tokens.includes(alias))) return true;
  }
  return false;
}

function loadCoverageEntries() {
  if (!existsSync(coveragePath)) return [];
  const coverage = JSON.parse(readFileSync(coveragePath, 'utf8'));
  if (!Array.isArray(coverage.entries)) return [];
  return coverage.entries;
}

function canonicalForEntry(entry, coverageEntry) {
  if (typeof entry.canonical === 'string' && entry.canonical.trim() !== '') return entry.canonical;
  if (typeof coverageEntry?.canonical === 'string' && coverageEntry.canonical.trim() !== '') return coverageEntry.canonical;
  if (typeof coverageEntry?.desugars_to === 'string' && coverageEntry.desugars_to.trim() !== '') return coverageEntry.desugars_to;
  if (typeof entry.concept === 'string' && entry.concept.trim() !== '') return entry.concept;
  return entry.surface;
}

function semanticMetadataForEntry(entry, coverageEntries) {
  const exact = coverageEntries.find((candidate) => candidate.id === entry.id);
  const coverageEntry = exact ?? coverageEntries.find((candidate) => manifestEntryMatchesCoverage(entry, candidate));
  const metadata = {
    canonical: canonicalForEntry(entry, coverageEntry),
  };
  if (coverageEntry) {
    metadata.coverage_entry_id = coverageEntry.id;
    for (const key of [
      'semantic_role',
      'algebraic_family',
      'core_tier',
      'derived_from',
      'desugars_to',
      'capability',
      'effect_schema',
      'reason',
      'exit_options',
      'review_gate',
      'implementation_schema',
      'classification',
    ]) {
      if (key in coverageEntry) metadata[key] = coverageEntry[key];
    }
  }
  return metadata;
}

function rustEnumVariantToSnake(value) {
  return value.replace(/([a-z0-9])([A-Z])/g, '$1_$2').toLowerCase();
}

function extractCoreWords() {
  const sourcePath = 'spec/words.json';
  const parsed = JSON.parse(readRepo(sourcePath)).entries.map((word) => ({
    name: word.name,
    category: word.category,
    vocabularyTier: word.vocabularyTier,
  }));
  if (parsed.length === 0) fail('no core words extracted');

  const baseCounts = new Map();
  for (const { name } of parsed) {
    const base = slug(name);
    baseCounts.set(base, (baseCounts.get(base) ?? 0) + 1);
  }
  return parsed.map(({ name, category, vocabularyTier }) => {
    const base = slug(name);
    const dropped = name.replace(/[a-zA-Z0-9]+/g, '');
    let id = `core.${base}`;
    if (baseCounts.get(base) > 1 && dropped) {
      id = `core.${base}${dropped.includes('?') ? '-p' : `-${slug(dropped) || 'x'}`}`;
    }
    return { id, kind: 'coreword', surface: name, category, vocabularyTier, source: sourcePath };
  });
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
  ...extractAliases(),
  ...extractSurfaceForms(),
];

const contracts = JSON.parse(readFileSync(contractsPath, 'utf8'));
if (contracts.migration?.completeInventory !== true) {
  fail('spec/words.json does not declare a complete canonical inventory');
}
const contractNames = new Set(contracts.entries.map((entry) => entry.name));
if (contractNames.size !== contracts.entries.length) fail('duplicate canonical name in spec/words.json');
const generatedCanonicalNames = new Set(entries
  .filter((entry) => entry.kind === 'coreword')
  .map((entry) => entry.surface));
for (const name of contractNames) {
  if (!generatedCanonicalNames.has(name)) fail(`Word contract ${name} is absent from the implementation catalog`);
}
for (const name of generatedCanonicalNames) {
  if (!contractNames.has(name)) fail(`implementation catalog Word ${name} is absent from spec/words.json`);
}

const coverageEntries = loadCoverageEntries();
for (const entry of entries) {
  Object.assign(entry, semanticMetadataForEntry(entry, coverageEntries));
}

const seen = new Set();
for (const entry of entries) {
  if (seen.has(entry.id)) fail(`duplicate manifest id ${entry.id}`);
  seen.add(entry.id);
}

const manifest = {
  schemaVersion: 2,
  generatedFrom: [
    'spec/words.json',
    'rust/src/core_word_aliases.rs',
    'rust/src/surface_forms.rs',
  ],
  implementationCatalogValidatedAgainst: [
    'spec/words.json',
    'rust/src/kernel/generated/word_registry.rs',
  ],
  semanticMetadataFrom: 'docs/formalization-coverage.json',
  counts: {
    canonicalWords: contracts.entries.length,
    semanticKernelWords: contracts.entries.filter((entry) => entry.vocabularyTier === 'kernel').length,
    standardWords: contracts.entries.filter((entry) => entry.vocabularyTier === 'standard').length,
    corewords: entries.filter((entry) => entry.kind === 'coreword').length,
    aliases: entries.filter((entry) => ['symbol_alias', 'syntax_sugar', 'input_helper'].includes(entry.kind)).length,
    surface_forms: entries.filter((entry) => !['coreword', 'symbol_alias', 'syntax_sugar', 'input_helper'].includes(entry.kind)).length,
    total: entries.length,
  },
  entries,
};

const json = `${JSON.stringify(manifest, null, 2)}\n`;
if (process.argv.includes('--check')) {
  // CI drift guard: fail if the committed manifest is out of sync with what
  // the generator now produces, so a new/last BuiltinSpec (e.g. SUPERVISE)
  // can never silently go missing again.
  const existing = existsSync(outputPath) ? readFileSync(outputPath, 'utf8') : '';
  if (existing !== json) {
    console.error(
      '[word-manifest] docs/word-manifest.json is stale. Run `npm run word:manifest` and commit the result.',
    );
    process.exit(1);
  }
  console.log(`[word-manifest] docs/word-manifest.json is up to date (${entries.length} entries).`);
} else if (process.argv.includes('--stdout')) {
  process.stdout.write(json);
} else {
  writeFileSync(outputPath, json);
  console.log(`[word-manifest] wrote ${entries.length} entries to docs/word-manifest.json`);
}
