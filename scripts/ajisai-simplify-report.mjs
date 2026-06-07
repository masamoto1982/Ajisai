#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const manifest = JSON.parse(readFileSync(resolve(repoRoot, 'docs/word-manifest.json'), 'utf8'));
const coverage = JSON.parse(readFileSync(resolve(repoRoot, 'docs/formalization-coverage.json'), 'utf8'));

function hasText(value) {
  return typeof value === 'string' && value.trim() !== '';
}

function hasSchemaValue(value) {
  if (hasText(value)) return true;
  if (Array.isArray(value)) return value.length > 0 && value.every(hasText);
  return value && typeof value === 'object' && !Array.isArray(value) && Object.keys(value).length > 0;
}

function list(items, empty = 'None detected.') {
  return items.length === 0 ? `- ${empty}` : items.map((item) => `- ${item}`).join('\n');
}

const primitiveIds = new Set((coverage.algebra_primitives ?? []).map((primitive) => primitive.id));
const primitiveUse = new Map([...primitiveIds].map((id) => [id, []]));
const duplicatePrimitiveIds = [];
const seenPrimitiveIds = new Set();
const primitiveMetadataGaps = [];

for (const primitive of coverage.algebra_primitives ?? []) {
  if (seenPrimitiveIds.has(primitive.id)) duplicatePrimitiveIds.push(primitive.id);
  seenPrimitiveIds.add(primitive.id);
  for (const key of ['description', 'kind', 'admission_reason', 'status']) {
    if (!hasText(primitive[key])) primitiveMetadataGaps.push(`${primitive.id}: missing ${key}`);
  }
  for (const key of ['introduced_by', 'can_derive']) {
    if (!Array.isArray(primitive[key]) || primitive[key].length === 0) {
      primitiveMetadataGaps.push(`${primitive.id}: missing non-empty ${key}`);
    }
  }
}

const unclassifiedManifest = [];
const manifestGaps = [];
for (const entry of manifest.entries ?? []) {
  if (!hasText(entry.semantic_role)) unclassifiedManifest.push(entry.id);
  if (!hasText(entry.canonical)) manifestGaps.push(`${entry.id}: missing canonical`);
  if (entry.semantic_role === 'Sugar' && !hasText(entry.desugars_to)) {
    manifestGaps.push(`${entry.id}: Sugar missing desugars_to`);
  }
  if (entry.semantic_role === 'HostedEffect' && (!hasSchemaValue(entry.capability) || !hasSchemaValue(entry.effect_schema))) {
    manifestGaps.push(`${entry.id}: HostedEffect missing capability/effect_schema`);
  }
}

const derivedMissing = [];
const unknownPrimitiveRefs = [];
const sugarMissing = [];
const hostedMissing = [];
const hostedCore = [];
const exploratoryMissing = [];
const exploratoryCore = [];
const implementationCandidates = [];

for (const entry of coverage.entries ?? []) {
  const role = entry.semantic_role;
  if (!hasText(role)) unclassifiedManifest.push(entry.id);
  if (role === 'Derived') {
    if (!Array.isArray(entry.derived_from) || entry.derived_from.length === 0) {
      derivedMissing.push(entry.id);
    } else {
      for (const ref of entry.derived_from) {
        if (!primitiveIds.has(ref)) unknownPrimitiveRefs.push(`${entry.id} -> ${ref}`);
        else primitiveUse.get(ref)?.push(entry.id);
      }
    }
    const refs = Array.isArray(entry.derived_from) ? entry.derived_from : [];
    if (refs.length > 0 && !entry.implementation_schema && /^(core\.|module\.)/.test(entry.id)) {
      implementationCandidates.push(`${entry.id}: consider documenting implementation_schema for ${refs.join(', ')}`);
    }
  }
  if (role === 'Primitive') {
    for (const ref of Array.isArray(entry.derived_from) ? entry.derived_from : []) {
      if (!primitiveIds.has(ref)) unknownPrimitiveRefs.push(`${entry.id} -> ${ref}`);
      else primitiveUse.get(ref)?.push(entry.id);
    }
  }
  if (role === 'Sugar' && !hasText(entry.desugars_to)) sugarMissing.push(entry.id);
  if (role === 'HostedEffect') {
    if (!hasSchemaValue(entry.capability) || !hasSchemaValue(entry.effect_schema)) hostedMissing.push(entry.id);
    if (entry.classification === 'Core') hostedCore.push(entry.id);
    for (const ref of Array.isArray(entry.derived_from) ? entry.derived_from : []) {
      if (primitiveIds.has(ref)) primitiveUse.get(ref)?.push(entry.id);
    }
  }
  if (role === 'Exploratory') {
    if (!hasText(entry.reason) || !Array.isArray(entry.exit_options) || entry.exit_options.length === 0 || !hasText(entry.review_gate)) {
      exploratoryMissing.push(entry.id);
    }
    if (entry.classification === 'Core') exploratoryCore.push(entry.id);
    for (const ref of Array.isArray(entry.derived_from) ? entry.derived_from : []) {
      if (primitiveIds.has(ref)) primitiveUse.get(ref)?.push(entry.id);
    }
  }
}

const unusedPrimitives = [...primitiveUse.entries()]
  .filter(([, users]) => users.length === 0)
  .map(([id]) => id);

const roleCounts = new Map();
for (const entry of coverage.entries ?? []) {
  const role = entry.semantic_role ?? 'Unclassified';
  roleCounts.set(role, (roleCounts.get(role) ?? 0) + 1);
}

const report = `# Ajisai Simplification Report

Generated from:

- docs/word-manifest.json
- docs/formalization-coverage.json

## Semantic graph summary

- Manifest entries: ${(manifest.entries ?? []).length}
- Coverage entries: ${(coverage.entries ?? []).length}
- Algebra primitives: ${(coverage.algebra_primitives ?? []).length}
- Role counts: ${[...roleCounts.entries()].map(([role, count]) => `${role}=${count}`).join(', ')}

## Metadata gaps

### Manifest canonical / role gaps
${list([...new Set([...unclassifiedManifest, ...manifestGaps])])}

### Derived entries without registered derivation
${list(derivedMissing)}

### Unknown primitive references
${list(unknownPrimitiveRefs)}

### Sugar entries without expansion
${list(sugarMissing)}

### HostedEffect schema gaps
${list(hostedMissing)}

### HostedEffect classified as Core
${list(hostedCore)}

### Exploratory debt metadata gaps
${list(exploratoryMissing)}

### Exploratory classified as Core
${list(exploratoryCore)}

## Primitive registry review

### Primitive metadata gaps
${list(primitiveMetadataGaps)}

### Duplicate primitive IDs
${list(duplicatePrimitiveIds)}

### Unused primitives
${list(unusedPrimitives)}

## Candidate: Derived implementation compression

The following entries are classified as Derived and reference algebraic primitives. They are candidates for follow-up review to confirm their implementation routes through a shared schema rather than maintaining independent semantics.

${list(implementationCandidates.slice(0, 80))}
${implementationCandidates.length > 80 ? `\n- ...and ${implementationCandidates.length - 80} more candidate(s).` : ''}

## Recommendation

- Keep metadata validation failing for Core classification gaps and missing required semantic fields.
- Treat unused primitives and implementation-compression candidates as review warnings until a focused refactor PR owns each family.
- Prefer small follow-up PRs by algebraic family: exact arithmetic, K3 logic, comparison, structure lift, and higher-order traversal.
`;

process.stdout.write(report);
