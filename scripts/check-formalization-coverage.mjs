#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const coveragePath = resolve(repoRoot, 'docs/formalization-coverage.json');
const conformancePath = resolve(repoRoot, 'tests/conformance/index.html');
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

const coverage = JSON.parse(readFileSync(coveragePath, 'utf8'));
if (coverage.version !== 1) fail('version must be 1');
if (!Array.isArray(coverage.entries)) fail('entries must be an array');

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
