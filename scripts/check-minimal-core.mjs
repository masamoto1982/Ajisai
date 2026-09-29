#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';

const KERNEL = new Set(`TRUE FALSE AND NOT EQ LT GT
ADD MUL DIV FLOOR SQRT POW
GET LENGTH CONCAT COLLECT RANGE FOLD MAP SHAPE RESHAPE FLATTEN DEPTH
RECORD KEYS VALUES WITHOUT HAS? MERGE PUT
CHARS JOIN NUM STR
SELECT EXEC CONTRACT FAIL NIL NIL? NIL-REASON ABSENT BIND DEF DEL DIGEST PRINT`.split(/\s+/));
const STANDARD = new Set(`SUB ROUND MIN MAX GCD RATIO
TAKE DROP REVERSE FILL SORT ORDER UNIQUE TALLY ZIP GROUP INDEX-OF MEMBER? BSEARCH FILTER SCAN
TRIM TOKENIZE SEARCH REPLACE UPPER LOWER FORMAT JSON-DECODE JSON-ENCODE`.split(/\s+/));
// The retired names live in spec/retired-words.json, the one representation
// this gate and rust/tests/beta_removed_words.rs both read: this side asserts
// none of them is canonical, the Rust side asserts the runtime does not
// resolve them. The reasons travel with the names there.
const retired = JSON.parse(readFileSync('spec/retired-words.json', 'utf8'));
const REMOVED_LIST = retired.groups.flatMap((group) => group.names);
const REMOVED = new Set(REMOVED_LIST);
const STANDARD_RELATIONS = new Set(['derivable', 'operational']);
const STANDARD_KINDS = new Set(['shorthand', 'namedPattern', 'algorithm', 'operational']);
const OPERATIONAL_LAW_TEST = 'rust/tests/standard_operational_laws.rs';
const DERIVATION_LAW_TEST = 'rust/tests/standard_derivation_laws.rs';
const DERIVABLE = new Set(`SUB ROUND MIN MAX
TAKE DROP REVERSE INDEX-OF MEMBER? TRIM TOKENIZE`.split(/\s+/));
// `FORMAT` and the JSON pair are Phase 6 of the vocabulary-100 work order:
// `FORMAT` is the one rounding boundary (a FLOOR-and-STR spelling would
// scatter it), and JSON nesting is input-dependent repetition no definition
// can write, so all three are operational rather than derivable.
// Phase 7 closes the number concept: `GCD` is input-dependent repetition
// (Euclid) and `RATIO` reads representation the language otherwise hides;
// both operational.
const OPERATIONAL = new Set('FILTER SCAN FILL SORT ORDER UNIQUE TALLY ZIP GROUP BSEARCH SEARCH REPLACE FORMAT JSON-DECODE JSON-ENCODE GCD RATIO UPPER LOWER'.split(/\s+/));

// The order in which the vocabulary would give up Words, cheapest to lose first.
//
// The vocabulary is held at 78, so an addition has to take a slot from
// something, and a candidate takes the head of this list. Every entry is a
// derivable Standard, checked below: an operational Word cannot be written in
// the language at all (LANG.AUTHORITY.FREEDOM's consequence for a total,
// non-recursive language), so retiring one would cut capability rather than
// spelling, and a Kernel Word is not on the table.
//
// The order is use, least first, as counted in the lexicon pilot's programs
// (tools/lexicon-emergence/runs/pilot-2026-09-23): MEMBER? 0 (as MEMBER, whose
// one-index batch form was its only non-derivable use and is gone), INDEX-OF 0, REVERSE 6,
// TRIM 10, TOKENIZE 12, MAX 13, MIN 14, TAKE 16, DROP 32, SUB 54 (written `-`).
// `ROUND` is deliberately absent though it is derivable: its Kernel phrase
// branches on the sign of its operand, which is the one of these a reader is
// likely to write wrongly by hand.
const RETIREMENT_QUEUE = 'MEMBER? INDEX-OF REVERSE TRIM TOKENIZE MAX MIN TAKE DROP SUB'.split(/\s+/);

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
if (kernelWords.size !== 48) errors.push(`Semantic Kernel has ${kernelWords.size} Words; expected 48`);
if (standardWords.size !== 30) errors.push(`Standard vocabulary has ${standardWords.size} Words; expected 30`);

if (words.length !== 78) errors.push(`canonical inventory has ${words.length} Words; expected 78`);
for (const name of REMOVED) if (wordNames.has(name)) errors.push(`${name}: removed Word remains canonical`);
if (REMOVED.size !== REMOVED_LIST.length) errors.push('spec/retired-words.json names the same Word twice');

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

// The retirement queue has to stay the thing it claims to be: every derivable
// Standard but `ROUND`, each one a spelling of a Kernel phrase rather than a
// capability. A queue naming a Word that has already gone, or one whose
// relation drifted to operational, would promise room it cannot give.
for (const name of setDifference(DERIVABLE, new Set([...RETIREMENT_QUEUE, 'ROUND']))) {
  errors.push(`${name}: derivable Standard missing from the retirement queue`);
}
if (new Set(RETIREMENT_QUEUE).size !== RETIREMENT_QUEUE.length) {
  errors.push('retirement queue names the same Word twice');
}
for (const name of RETIREMENT_QUEUE) {
  if (!wordNames.has(name)) {
    errors.push(`${name}: queued for retirement but not in the canonical inventory`);
    continue;
  }
  if (!DERIVABLE.has(name)) {
    errors.push(`${name}: queued for retirement but not a derivable Standard`);
  }
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
console.log(
  `[minimal-core] ${RETIREMENT_QUEUE.length} Words queued for retirement, first out ${RETIREMENT_QUEUE[0]}: ` +
    RETIREMENT_QUEUE.join(' '),
);
