#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const KERNEL = new Set(`TRUE FALSE AND NOT EQ LT GT
ADD MUL DIV FLOOR NEG SQRT
GET LENGTH CONCAT COLLECT RANGE FOLD
CHARS JOIN NUM STR
COND EXEC PROBE NIL NIL? NIL-REASON VENT KEEP BIND DEF DEL PRINT REFLECT RANDOM`.split(/\s+/));
const STANDARD = new Set(`OR NEQ LTE GTE SUB MOD ROUND QUANTIZE ABS MIN MAX
TAKE REVERSE FILL SORT ORDER UNIQUE TALLY ZIP SUM PUT GROUP INDEX-OF MAP FILTER ANY ALL
TRIM TOKENIZE`.split(/\s+/));
// The alpha Words retired in the phase-1 vocabulary freeze. `UNIQUE` is not
// among them any more: it was cut as one of several overlapping collection
// Words and has come back on its own terms, with a contract, a law witness and
// a conformance case — see `core.unique` in the coverage manifest.
const REMOVED = new Set(`CEIL SIGN INSERT REPLACE REMOVE SPLIT REORDER CONTAINS
STARTS-WITH? ENDS-WITH? CHR EAT`.split(/\s+/));
const STANDARD_RELATIONS = new Set(['derivable', 'operational']);
const STANDARD_KINDS = new Set(['shorthand', 'namedPattern', 'algorithm', 'operational']);
const OPERATIONAL_LAW_TEST = 'rust/tests/standard_operational_laws.rs';
const DERIVATION_LAW_TEST = 'rust/tests/standard_derivation_laws.rs';
const DERIVABLE = new Set(`OR NEQ LTE GTE SUB MOD ROUND QUANTIZE ABS MIN MAX
TAKE REVERSE INDEX-OF TRIM TOKENIZE SUM`.split(/\s+/));
const OPERATIONAL = new Set('MAP FILTER ANY ALL FILL SORT ORDER UNIQUE TALLY ZIP PUT GROUP'.split(/\s+/));

const contracts = JSON.parse(readFileSync('spec/words.json', 'utf8'));
const words = contracts.entries;
const coverage = JSON.parse(readFileSync('docs/formalization-coverage.json', 'utf8'));
const wordNames = new Set(words.map((word) => word.name));
const entries = coverage.entries.filter((entry) => wordNames.has(entry.surface));
const primitives = new Set(coverage.algebra_primitives.map((entry) => entry.id));
const bySurface = new Map();
const errors = [];
const setDifference = (left, right) => [...left].filter((item) => !right.has(item));

if (words.length !== wordNames.size) errors.push('canonical inventory contains duplicate names');
for (const entry of entries) {
  if (bySurface.has(entry.surface)) errors.push(`duplicate Core Word witness: ${entry.surface}`);
  bySurface.set(entry.surface, entry);
}

const kernelWords = new Set(words.filter((word) => word.vocabularyTier === 'kernel').map((word) => word.name));
const standardWords = new Set(words.filter((word) => word.vocabularyTier === 'standard').map((word) => word.name));
for (const name of setDifference(KERNEL, kernelWords)) errors.push(`${name}: missing Semantic Kernel classification`);
for (const name of setDifference(kernelWords, KERNEL)) errors.push(`${name}: unexpected Semantic Kernel classification`);
for (const name of setDifference(STANDARD, standardWords)) errors.push(`${name}: missing Standard classification`);
for (const name of setDifference(standardWords, STANDARD)) errors.push(`${name}: unexpected Standard classification`);
if (kernelWords.size !== 37) errors.push(`Semantic Kernel has ${kernelWords.size} Words; expected 37`);
if (standardWords.size !== 29) errors.push(`Standard vocabulary has ${standardWords.size} Words; expected 29`);

const isPhaseOne = contracts.migration?.betaFreezePhase === 1;
if (isPhaseOne) {
  if (words.length !== 70) errors.push(`phase 1 transitional inventory has ${words.length} Words; expected 70`);
  const pendingRemoval = new Set(words.filter((word) => !word.vocabularyTier).map((word) => word.name));
  for (const name of setDifference(REMOVED, pendingRemoval)) errors.push(`${name}: missing from phase 1 removal set`);
  for (const name of setDifference(pendingRemoval, REMOVED)) errors.push(`${name}: unclassified Word is not in the removal set`);
} else {
  if (words.length !== 66) errors.push(`canonical inventory has ${words.length} Words; expected 66`);
  for (const name of REMOVED) if (wordNames.has(name)) errors.push(`${name}: removed Word remains canonical`);
}

for (const word of words) {
  const witness = bySurface.get(word.name);
  if (!witness) {
    errors.push(`${word.name}: missing Minimal Core witness`);
    continue;
  }
  if (!['Formalized', 'HostedEffect'].includes(witness.status)) errors.push(`${word.name}: status is ${witness.status}`);
  if (!witness.law_tests?.length) errors.push(`${word.name}: no executable law test`);
  for (const testPath of witness.law_tests ?? []) {
    if (!existsSync(testPath)) errors.push(`${word.name}: missing law test file ${testPath}`);
  }
  if (!['identity', 'flow', 'material'].includes(witness.core_tier)) errors.push(`${word.name}: invalid core_tier ${witness.core_tier}`);
  if (witness.core_tier === word.vocabularyTier) errors.push(`${word.name}: core_tier is confused with vocabularyTier`);
  if (witness.semantic_role === 'Primitive') {
    if (!witness.primitive) errors.push(`${word.name}: Primitive role is not marked primitive`);
  } else if (witness.semantic_role === 'Derived') {
    if (witness.primitive) errors.push(`${word.name}: Derived role is marked primitive`);
    if (!witness.derived_from?.length) errors.push(`${word.name}: derived Word has no algebra basis`);
  } else if (witness.semantic_role !== 'HostedEffect') {
    errors.push(`${word.name}: unsupported semantic role ${witness.semantic_role}`);
  }
  for (const dependency of witness.derived_from ?? []) {
    if (!primitives.has(dependency)) errors.push(`${word.name}: unknown algebra primitive ${dependency}`);
  }
  if (word.vocabularyTier === 'standard') {
    if (!STANDARD_KINDS.has(word.standardKind)) errors.push(`${word.name}: invalid or missing standardKind`);
    if (!STANDARD_RELATIONS.has(witness.standard_relation)) errors.push(`${word.name}: invalid or missing Standard relation`);
    if (!witness.conformance_cases?.length) errors.push(`${word.name}: Standard Word has no conformance case`);
    if (witness.standard_relation === 'operational' && !witness.native_retention_reason) {
      errors.push(`${word.name}: operational Standard has no native retention reason`);
    }
    if (witness.standard_relation === 'operational' && !witness.law_tests.includes(OPERATIONAL_LAW_TEST)) {
      errors.push(`${word.name}: operational Standard is not covered by ${OPERATIONAL_LAW_TEST}`);
    }
    if (witness.standard_relation === 'derivable' && !witness.law_tests.includes(DERIVATION_LAW_TEST)) {
      errors.push(`${word.name}: derivable Standard has no Kernel witness in ${DERIVATION_LAW_TEST}`);
    }
    if (DERIVABLE.has(word.name) && witness.standard_relation !== 'derivable') {
      errors.push(`${word.name}: expected the derivable relation, found ${witness.standard_relation}`);
    }
    if (OPERATIONAL.has(word.name) && witness.standard_relation !== 'operational') {
      errors.push(`${word.name}: expected the operational relation, found ${witness.standard_relation}`);
    }
  }
}

const derivableWords = new Set(
  words.filter((word) => bySurface.get(word.name)?.standard_relation === 'derivable').map((word) => word.name),
);
const operationalWords = new Set(
  words.filter((word) => bySurface.get(word.name)?.standard_relation === 'operational').map((word) => word.name),
);
if (derivableWords.size !== DERIVABLE.size) {
  errors.push(`${derivableWords.size} derivable Standards declared; expected ${DERIVABLE.size}`);
}
if (operationalWords.size !== OPERATIONAL.size) {
  errors.push(`${operationalWords.size} operational Standards declared; expected ${OPERATIONAL.size}`);
}

for (const entry of coverage.entries.filter((entry) => entry.kind === 'coreword')) {
  if (!wordNames.has(entry.surface)) errors.push(`${entry.surface}: witness has no canonical Word`);
}
if (bySurface.size !== words.length) errors.push(`witness inventory has ${bySurface.size} entries; expected ${words.length}`);

if (errors.length) {
  errors.forEach((error) => console.error(`[minimal-core] ${error}`));
  process.exit(1);
}
console.log(
  `[minimal-core] ${kernelWords.size}/${kernelWords.size} Semantic Kernel Words have executable witnesses.`,
);
console.log(
  `[minimal-core] ${standardWords.size}/${standardWords.size} Standard Words have complete contracts and law witnesses.`,
);
console.log(
  `[minimal-core] ${DERIVABLE.size}/${DERIVABLE.size} derivable Standards carry a Kernel-only witness; ` +
    `${OPERATIONAL.size}/${OPERATIONAL.size} operational Standards state a native retention reason.`,
);
if (isPhaseOne) {
  console.log(`[minimal-core] phase 1: ${REMOVED.size} alpha Words remain explicitly pending removal.`);
}
