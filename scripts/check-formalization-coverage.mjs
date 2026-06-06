#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const coveragePath = resolve(repoRoot, 'docs/formalization-coverage.json');
const conformancePath = resolve(repoRoot, 'tests/conformance/index.html');
const wordManifestPath = resolve(repoRoot, 'docs/word-manifest.json');
const allowedStatuses = new Set([
  'Formalized',
  'Sketched',
  'HostedEffect',
  'Exploratory',
  'NotPortableYet',
]);

const allowedSemanticRoles = new Set([
  'Primitive',
  'Derived',
  'Sugar',
  'HostedEffect',
  'Exploratory',
  'NotPortableYet',
]);

const allowedAlgebraicFamilies = new Set([
  'state-transformer',
  'stack',
  'dictionary',
  'exact-scalar',
  'exact-arithmetic',
  'k3-truth',
  'bubble',
  'modifier',
  'structure-lift',
  'hosted-effect',
  'syntax-sugar',
  'observation',
]);

function fail(message) {
  throw new Error(`[formalization-coverage] ${message}`);
}

function normalizeSurface(value) {
  return typeof value === 'string' ? value.trim().toUpperCase() : '';
}

function coverageSurfaces(entry) {
  const surfaces = [];
  if (typeof entry.surface === 'string') surfaces.push(entry.surface);
  if (Array.isArray(entry.surfaces)) surfaces.push(...entry.surfaces.filter((v) => typeof v === 'string'));
  return surfaces;
}

function entryClassifiesSurface(entry, manifestEntry) {
  if (entry.id === manifestEntry.id) return true;
  const manifestSurface = normalizeSurface(manifestEntry.surface);
  const aliases = Array.isArray(manifestEntry.coverage_aliases)
    ? manifestEntry.coverage_aliases.map(normalizeSurface).filter(Boolean)
    : [];
  for (const surface of coverageSurfaces(entry)) {
    const coverageSurface = normalizeSurface(surface);
    if (coverageSurface === manifestSurface || aliases.includes(coverageSurface)) {
      return true;
    }
    const tokens = coverageSurface.match(/[A-Z0-9@?>=<!&+*/%.,;#$'\[\]{}()-]+/g) ?? [];
    if (tokens.includes(manifestSurface) || aliases.some((alias) => tokens.includes(alias))) {
      return true;
    }
  }
  return false;
}

function validateWordManifest(coverage) {
  const manifest = JSON.parse(readFileSync(wordManifestPath, 'utf8'));
  if (manifest.schemaVersion !== 1) fail('word manifest schemaVersion must be 1');
  if (!Array.isArray(manifest.entries)) fail('word manifest entries must be an array');

  const manifestById = new Map();
  const manifestBySurface = new Map();
  for (const [index, entry] of manifest.entries.entries()) {
    const where = entry?.id ?? `manifest entry #${index}`;
    if (!entry || typeof entry !== 'object') fail(`${where}: manifest entry must be an object`);
    for (const key of ['id', 'kind', 'surface']) {
      if (typeof entry[key] !== 'string' || entry[key].trim() === '') {
        fail(`${where}: manifest entry missing non-empty ${key}`);
      }
    }
    if (manifestById.has(entry.id)) fail(`${where}: duplicate manifest id`);
    manifestById.set(entry.id, entry);
    const surfaceKey = normalizeSurface(entry.surface);
    if (!manifestBySurface.has(surfaceKey)) manifestBySurface.set(surfaceKey, []);
    manifestBySurface.get(surfaceKey).push(entry);
    if (typeof entry.short_surface === 'string') {
      const shortKey = normalizeSurface(entry.short_surface);
      if (!manifestBySurface.has(shortKey)) manifestBySurface.set(shortKey, []);
      manifestBySurface.get(shortKey).push(entry);
    }
  }

  const coveredManifestIds = new Set();
  for (const entry of coverage.entries) {
    for (const manifestEntry of manifest.entries) {
      if (entryClassifiesSurface(entry, manifestEntry)) coveredManifestIds.add(manifestEntry.id);
    }

    const isSurfaceEntry = ['coreword', 'moduleword', 'symbol_alias', 'syntax_sugar', 'input_helper', 'delimiter_sugar', 'literal_sugar', 'modifier_sugar', 'source_directive', 'control_directive', 'reserved_marker', 'conversion_word'].includes(entry.kind);
    if (isSurfaceEntry && !manifestById.has(entry.id)) {
      const hasSurfaceMatch = coverageSurfaces(entry).some((surface) => {
        const normalized = normalizeSurface(surface);
        return [...manifestBySurface.keys()].some((manifestSurface) => normalized.includes(manifestSurface));
      });
      if (!hasSurfaceMatch) fail(`${entry.id}: coverage surface entry is not present in word manifest`);
    }
  }

  const allowlist = Array.isArray(coverage.unclassified_allowlist) ? coverage.unclassified_allowlist : [];
  const allowlistedIds = new Set();
  for (const id of allowlist) {
    if (typeof id !== 'string' || id.trim() === '') fail('unclassified_allowlist entries must be non-empty strings');
    if (!manifestById.has(id)) fail(`unclassified_allowlist references unknown manifest id ${id}`);
    if (allowlistedIds.has(id)) fail(`unclassified_allowlist contains duplicate id ${id}`);
    allowlistedIds.add(id);
  }

  const unclassified = [];
  for (const entry of manifest.entries) {
    if (!coveredManifestIds.has(entry.id)) unclassified.push(entry.id);
  }
  const unexpectedUnclassified = unclassified.filter((id) => !allowlistedIds.has(id));
  if (unexpectedUnclassified.length > 0) {
    fail(`${unexpectedUnclassified.length} surface word(s) are unclassified and not allowlisted: ${unexpectedUnclassified.join(', ')}`);
  }

  const staleAllowlist = [...allowlistedIds].filter((id) => !unclassified.includes(id));
  if (staleAllowlist.length > 0) {
    fail(`unclassified_allowlist contains already-classified id(s): ${staleAllowlist.join(', ')}`);
  }

  const classifiedCount = manifest.entries.length - unclassified.length;
  const percent = manifest.entries.length === 0 ? 100 : (classifiedCount / manifest.entries.length) * 100;
  console.log(`[formalization-coverage] ${classifiedCount}/${manifest.entries.length} surface words classified (${percent.toFixed(1)}%)`);
  if (unclassified.length > 0) {
    console.log(`[formalization-coverage] ${unclassified.length} unclassified surface word(s) currently allowlisted: ${unclassified.join(', ')}`);
  }
}

const coverage = JSON.parse(readFileSync(coveragePath, 'utf8'));
if (coverage.version !== 1) fail('version must be 1');
if (!Array.isArray(coverage.entries)) fail('entries must be an array');
validateWordManifest(coverage);

// Optional algebra-primitive registry. When present it closes the
// `derived_from` vocabulary: every derived word must resolve to a declared
// semantic primitive, so the derivation graph is checked rather than free-form.
// Absent => the reference check is skipped (backward compatible).
let primitiveIds = null;
if ('algebra_primitives' in coverage) {
  if (!Array.isArray(coverage.algebra_primitives)) fail('algebra_primitives must be an array');
  primitiveIds = new Set();
  for (const [index, prim] of coverage.algebra_primitives.entries()) {
    const pw = prim?.id ?? `algebra_primitive #${index}`;
    if (!prim || typeof prim !== 'object') fail(`${pw}: primitive must be an object`);
    if (typeof prim.id !== 'string' || prim.id.trim() === '') fail(`${pw}: missing non-empty id`);
    if (primitiveIds.has(prim.id)) fail(`${pw}: duplicate primitive id`);
    primitiveIds.add(prim.id);
    if (
      typeof prim.algebraic_family !== 'string' ||
      !allowedAlgebraicFamilies.has(prim.algebraic_family)
    ) {
      fail(`${pw}: primitive needs a known algebraic_family`);
    }
  }
}

const conformanceHtml = readFileSync(conformancePath, 'utf8');
const caseIds = new Set(
  [...conformanceHtml.matchAll(/<section\b[^>]*\bclass=["'][^"']*\bajisai-case\b[^"']*["'][^>]*\bid=["']([^"']+)["']/g)]
    .map((match) => match[1]),
);

const seenIds = new Set();
for (const [index, entry] of coverage.entries.entries()) {
  const where = entry?.id ?? `entry #${index}`;
  if (!entry || typeof entry !== 'object') fail(`${where}: entry must be an object`);
  for (const key of ['id', 'kind', 'status']) {
    if (typeof entry[key] !== 'string' || entry[key].trim() === '') {
      fail(`${where}: missing non-empty ${key}`);
    }
  }
  if (seenIds.has(entry.id)) fail(`${where}: duplicate id`);
  seenIds.add(entry.id);
  if (!allowedStatuses.has(entry.status)) fail(`${where}: invalid status ${entry.status}`);

  const formalizationSections = Array.isArray(entry.formalization_sections)
    ? entry.formalization_sections.filter(Boolean)
    : [];
  const conformanceCases = Array.isArray(entry.conformance_cases)
    ? entry.conformance_cases.filter(Boolean)
    : [];
  const lawTests = Array.isArray(entry.law_tests) ? entry.law_tests.filter(Boolean) : [];

  if (entry.status === 'Formalized') {
    if (formalizationSections.length === 0) {
      fail(`${where}: Formalized entries need formalization_sections`);
    }
    if (conformanceCases.length === 0 && lawTests.length === 0) {
      fail(`${where}: Formalized entries need conformance_cases or law_tests`);
    }
  }

  if (entry.status === 'NotPortableYet' && entry.classification === 'Core') {
    fail(`${where}: NotPortableYet entries must not be classified as Core`);
  }


  if ('semantic_role' in entry) {
    if (!allowedSemanticRoles.has(entry.semantic_role)) {
      fail(`${where}: invalid semantic_role ${entry.semantic_role}`);
    }

    if (entry.semantic_role === 'Derived') {
      if (!Array.isArray(entry.derived_from)) {
        fail(`${where}: Derived entries need derived_from`);
      }
      if (entry.derived_from.length === 0 && !/derived_from/i.test(entry.notes ?? '')) {
        fail(`${where}: Derived entries with empty derived_from must document the investigation gap in notes`);
      }
    }

    // When the registry is declared, every derived_from reference must
    // resolve to a known semantic primitive (closed derivation vocabulary).
    if (primitiveIds && Array.isArray(entry.derived_from)) {
      for (const ref of entry.derived_from) {
        if (!primitiveIds.has(ref)) {
          fail(`${where}: derived_from references unknown algebra primitive ${ref}`);
        }
      }
    }

    if (entry.semantic_role === 'Primitive' && entry.primitive !== true) {
      fail(`${where}: Primitive entries must set primitive to true`);
    }

    if (entry.semantic_role === 'HostedEffect' && entry.classification === 'Core') {
      fail(`${where}: HostedEffect semantic roles must not be classified as Core`);
    }

    if (entry.semantic_role === 'NotPortableYet' && entry.classification === 'Core') {
      fail(`${where}: NotPortableYet semantic roles must not be classified as Core`);
    }
  }

  if ('algebraic_family' in entry && !allowedAlgebraicFamilies.has(entry.algebraic_family)) {
    fail(`${where}: unknown algebraic_family ${entry.algebraic_family}`);
  }

  for (const caseId of conformanceCases) {
    if (!caseIds.has(caseId)) fail(`${where}: unknown conformance case ${caseId}`);
  }

  for (const lawTest of lawTests) {
    if (/\.(rs|ts|js|mjs)$/.test(lawTest) && !existsSync(resolve(repoRoot, lawTest))) {
      fail(`${where}: referenced law test file does not exist: ${lawTest}`);
    }
  }
}

if (primitiveIds) {
  const used = new Set();
  for (const entry of coverage.entries) {
    for (const ref of Array.isArray(entry.derived_from) ? entry.derived_from : []) used.add(ref);
  }
  const unused = [...primitiveIds].filter((id) => !used.has(id));
  if (unused.length > 0) {
    // Non-fatal: a declared-but-unused primitive is a hint of dead metadata,
    // not a hard error, so it does not break backward-compatible consumers.
    console.log(`[formalization-coverage] note: ${unused.length} declared primitive(s) unused: ${unused.join(', ')}`);
  }
  console.log(`[formalization-coverage] ${primitiveIds.size} algebra primitives declared`);
}

console.log(`[formalization-coverage] ${coverage.entries.length} entries validated`);
