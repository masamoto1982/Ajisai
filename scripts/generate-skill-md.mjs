#!/usr/bin/env node
// Generate SKILL.md — the thin "read this, then write Ajisai" protocol for AI
// agents — from machine sources, never by hand.
//
// Inputs:
//   - docs/word-manifest.json            (the surface inventory gate: §9)
//   - spec/words.json                    (canonical Word documentation)
//   - curated snippet data in this file  (§6 examples, §7 errors, §8 forbidden)
//
// Every snippet is executed through the real `ajisai` CLI and the *actual*
// `--json` output (stackDisplay / output / diagnosis fields) is embedded.
// If language behavior changes, regeneration changes SKILL.md and the
// `check:skill` CI step fails until the committed copy is refreshed — the
// guide cannot drift from the implementation.
//
// Usage:
//   node scripts/generate-skill-md.mjs            # write SKILL.md
//   node scripts/generate-skill-md.mjs --check    # fail if SKILL.md is stale
//   AJISAI_BIN=/path/to/ajisai ...                # override CLI binary

import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fatal, readJson, repoRoot, resolveAjisaiBin, spawnAgent, words, writeOrCheck } from './lib/common.mjs';

const fail = (message) => fatal('skill-md', message);

// ---------------------------------------------------------------------------
// CLI harness
// ---------------------------------------------------------------------------

const ajisaiBin = resolveAjisaiBin('skill-md');

function runSnippet(code, command = 'compute') {
  // `check` reads no limit profile (a flag a command does not read is a usage
  // error), so it is spawned without the `--limits trusted` every compute gets.
  const proc = command === 'check'
    ? spawnSync(ajisaiBin, ['agent', 'check', '-'], { input: `${code}\n`, encoding: 'utf8' })
    : spawnAgent(ajisaiBin, code, { command });
  if (proc.error) fail(`failed to spawn ajisai CLI: ${proc.error.message}`);
  let json = null;
  try {
    json = JSON.parse(proc.stdout);
  } catch {
    fail(`CLI stdout for ${JSON.stringify(code)} is not valid JSON:\n${proc.stdout}`);
  }
  return { exit: proc.status, json };
}

function expectOk(code) {
  const { exit, json } = runSnippet(code);
  if (exit !== 0) fail(`snippet must succeed but failed (${json.message}): ${code}`);
  return json;
}

/** Run `ajisai agent check` on `code`; it must pass (exit 0). */
function expectCheckOk(code) {
  const { exit, json } = runSnippet(code, 'check');
  if (exit !== 0) fail(`check must pass but failed (${json.message}): ${code}`);
  return json;
}

function expectError(code) {
  const { exit, json } = runSnippet(code);
  if (exit !== 1) fail(`snippet must fail with exit 1 but exited ${exit}: ${code}`);
  if (!json.diagnosis || !Array.isArray(json.diagnosis.nextChecks)) {
    fail(`error snippet missing diagnosis/nextChecks: ${code}`);
  }
  return json;
}

// ---------------------------------------------------------------------------
// Word inventory (§9) — surfaces from the manifest, contracts from words.json
// ---------------------------------------------------------------------------

// Vocabulary counts read from the manifest, so the sentence introducing the
// table cannot drift from the table itself.
function readVocabularyCounts() {
  const manifest = readJson('docs/word-manifest.json');
  const { canonicalWords, semanticKernelWords, standardWords } = manifest.counts;
  for (const [name, value] of Object.entries({ canonicalWords, semanticKernelWords, standardWords })) {
    if (typeof value !== 'number') fail(`docs/word-manifest.json counts.${name} is missing`);
  }
  if (semanticKernelWords + standardWords !== canonicalWords) {
    fail('docs/word-manifest.json kernel + standard counts do not sum to the canonical inventory');
  }
  return { canonicalWords, semanticKernelWords, standardWords };
}

function buildWordTable() {
  const manifest = readJson('docs/word-manifest.json');
  const { entries } = words();
  const contracts = new Map(entries.map((entry) => [entry.name, entry]));
  const rows = [];
  for (const entry of manifest.entries) {
    if (entry.kind === 'coreword') {
      const contract = contracts.get(entry.canonical);
      if (!contract) fail(`no contract found for coreword ${entry.surface}`);
      const syntax = contract.documentation.syntax ? ` — e.g. \`${contract.documentation.syntax}\`` : '';
      rows.push(`| \`${entry.surface}\` | ${entry.family} | ${contract.documentation.summary}${syntax} |`);
    }
  }
  return rows;
}

// ---------------------------------------------------------------------------
// Curated, execution-verified snippet data
// ---------------------------------------------------------------------------

const canonicalExamples = [
  { id: 'scalar', title: 'Push a number: a bare scalar', code: '42' },
  { id: 'div', title: 'Exact rational division — no floats, ever', code: '1 3 DIV' },
  { title: 'Elementwise vector arithmetic', code: '[ 1 2 3 ] [ 4 5 6 ] ADD' },
  { title: 'Scalar broadcast over a vector', code: '5 [ 1 2 3 ] MUL' },
  { title: 'Remainder: name the operands, then a - b * floor(a/b)', code: "10 'A' BIND 3 'B' BIND A A B DIV FLOOR B MUL SUB" },
  { title: 'Comparison pushes a boolean', code: '1 2 LT' },
  { title: 'Comparison lifts over vectors element-wise', code: '[ 1 2 ] [ 3 1 ] LT' },
  { title: 'Range: start end, both included', code: '0 5 RANGE' },
  { title: 'A stride is a multiplication of a range', code: '0 5 RANGE 2 MUL' },
  { title: 'Fill a shape with one number: [ shape ] value', code: '[ 2 2 ] 7 FILL' },
  { title: 'MAP with a [ ] code block', code: '0 4 RANGE [ 2 MUL ] MAP' },
  { title: 'FILTER keeps matching elements', code: '0 10 RANGE [ 5 GT ] FILTER' },
  { id: 'fold', title: 'FOLD needs an explicit initial value', code: '[ 1 2 3 ] 0 [ ADD ] FOLD' },
  {
    id: 'record-basic',
    title: 'A Record from a Vector of keys and a Vector of values',
    code: "[ 'x' 'y' ] [ 1 2 ] RECORD",
  },
  {
    id: 'def-basic',
    title: 'Define a user word: [ body ] then name, then DEF',
    code: "[ 1 2 ADD ] 'MY-SUM' DEF MY-SUM",
  },
  {
    id: 'select-basic',
    title: 'SELECT: the two candidates, then the truth that chooses between them',
    code: "'non-negative' 'negative' 4 0 LT NOT SELECT PRINT",
  },
  {
    id: 'select-lanes',
    title: 'SELECT chooses lane by lane, so a whole vector branches at once',
    code: '0 [ -3 5 -1 ] [ -3 5 -1 ] 0 LT SELECT',
  },
  { title: 'Strings are bare \'...\' literals; CHARS/JOIN convert', code: "'hello' CHARS REVERSE JOIN" },
  { title: 'Cast a string to an exact number', code: "'42' NUM" },
  { title: 'PRINT pops and emits to output (not the stack)', code: '[ 1 2 3 ] PRINT' },
  { title: 'Sorting is a plain Core word', code: '[ 3 1 2 ] SORT' },
  { title: 'Exact square root takes a bare scalar', code: '2 SQRT' },
  { title: 'A value used twice is named with BIND', code: "5 'N' BIND N N 1 ADD" },
];

// `#:contract` declarations: the one feature README's "Why Ajisai" leads
// with that no reading surface showed the syntax of. Each is run through the
// real check, so the grammar written here is the grammar the checker parses.
const contractDeclarations = [
  {
    id: 'contract-verified',
    title: 'Declare what your Word does; check verifies it before anything runs',
    code: "#:contract DOUBLE inputs=1 outputs=1 purity=pure partiality=total\n[ 2 MUL ] 'DOUBLE' DEF 21 DOUBLE",
  },
];

const commonErrors = [
  {
    title: 'Typo / unknown word',
    code: '[ 1 ] ADDD',
    fix: 'Grep §9 for the word you meant (here: `ADD`). Word names are upper-cased automatically.',
  },
  {
    title: 'Stack underflow: operands must be pushed first',
    code: 'ADD',
    fix: 'Push both operands before the operator: `1 2 ADD`. Ajisai is postfix; there is no infix form.',
  },
  {
    title: 'FOLD without an initial value',
    code: '[ 1 2 3 ] [ ADD ] FOLD',
    fix: 'FOLD is `vector init [ op ] FOLD`: `[ 1 2 3 ] 0 [ ADD ] FOLD`.',
  },
  {
    title: 'A block that leaves more than its one result',
    code: '[ 1 2 3 ] [ 2 MUL 7 ] MAP',
    fix: 'A MAP / FILTER / FOLD / SCAN block leaves exactly one value — its result. A surplus is not discarded, it is this error. Consume what you computed before the block ends, or name it with `BIND`, which leaves nothing: `[ 2 MUL ]`, or `[ \'X\' BIND X X MUL ]`.',
  },
  {
    title: 'A `#:contract` declaration the body contradicts',
    code: "#:contract DOUBLE inputs=2 outputs=1\n[ 2 MUL ] 'DOUBLE' DEF 5 DOUBLE",
    fix: 'The declaration is checked before anything runs, and a violated one stops the run (`contractDecls.findings` lists every finding). Fix the body or the line; `ajisai agent infer-contracts` answers a paste-ready `suggested` line for the Word as written.',
  },
  {
    title: 'SELECT takes three operands: both candidates, then the truth',
    code: "[ 'big' ] [ 5 ] [ 3 ] GT SELECT",
    fix: "SELECT is `whenTrue whenFalse truth SELECT` — push both candidates before the test that chooses between them: `[ 'big' ] [ 'small' ] [ 5 ] [ 3 ] GT SELECT`. It chooses between values, never running either one, so an effect goes after it: `... SELECT PRINT`.",
  },
  {
    title: 'SELECT needs a truth value, not a number',
    code: "[ 'y' ] [ 'n' ] 1 SELECT",
    fix: 'The third operand must be TRUE, FALSE or an absence — a scalar is not a truth value (§4). Write the test: `1 0 EQ NOT`.',
  },
  {
    title: 'Broadcast shape mismatch',
    code: '[ 1 2 ] [ 1 2 3 ] ADD',
    fix: 'Elementwise ops need equal or broadcastable shapes (a scalar `5`, or a one-element vector, broadcasts; `[2]` vs `[3]` does not).',
  },
  {
    title: 'NUM casts strings, not booleans',
    code: 'TRUE NUM',
    fix: "NUM accepts strings: `'42' NUM`. There is no boolean→number cast.",
  },
  {
    title: 'Old one-vector RANGE form',
    code: '[ 0 5 ] RANGE',
    fix: 'RANGE takes two bounds: `0 5 RANGE`. A stride is a multiplication: `0 5 RANGE 2 MUL`.',
  },
];

// Programs that succeed while meaning something other than they look like they
// mean. `commonErrors` cannot hold these: nothing raises, so a reader — and an
// AI in particular — has no signal that anything went wrong, which is exactly
// what makes them worth writing down. Both the wrong and the right form are
// executed, and the two stacks are printed side by side.
const silentMistakes = [
  {
    title: 'A one-element vector where a Word wants an element',
    wrong: '[ 1 2 3 ] [ 1 ] [ 9 ] PUT',
    right: '[ 1 2 3 ] 1 9 PUT',
    fix: 'PUT, GET and INDEX-OF take an *element*, not a one-element vector holding it: `[ 9 ]` is that vector, so it is stored as one. Write a scalar bare (§2) and this cannot happen; a one-element vector is a vector, everywhere, and no error says so.',
  },
];

const forbiddenPatterns = [
  {
    pattern: 'DUP / SWAP / DROP / OVER / ROT',
    code: 'DUP',
    why: 'Forth-style stack shufflers do not exist. Every Word consumes the operands it reads; name a value with `BIND` to use it more than once.',
  },
  {
    pattern: 'IF / ELSE / THEN / WHILE',
    code: '[ 1 ] IF',
    why: 'No structured keywords, and no loops. Branch with SELECT over two values; iterate with MAP / FILTER / FOLD / SCAN.',
  },
  {
    pattern: 'A word calling itself',
    code: "[ REC ] 'REC' DEF",
    why: 'The User dictionary is acyclic: `DEF` refuses a body that names the word being defined, directly or through other user words, so this fails at definition time rather than the call. Repetition is expressed only through MAP / FILTER / FOLD / SCAN over an already-finite vector.',
  },
  {
    pattern: 'Parentheses ( )',
    code: '( 1 2 )',
    why: 'Reserved; not valid in source. `[ ]` is the sole bracket, for vectors and code alike.',
  },
  {
    pattern: 'Double-quoted strings',
    code: '"hello" PRINT',
    why: "Strings use single quotes: 'hello'.",
  },
  {
    pattern: '// line comments',
    code: '// comment',
    why: 'Comments start with `#`.',
  },
];

// ---------------------------------------------------------------------------
// Section renderers
// ---------------------------------------------------------------------------

function renderResult(json) {
  const parts = [];
  if (json.output.length > 0) parts.push(`prints \`${json.output.join(' ⏎ ')}\``);
  if (json.stackDisplay.length > 0) parts.push(`stack: \`${json.stackDisplay.join('  ')}\``);
  if (parts.length === 0) parts.push('stack: (empty)');
  return parts.join('; ');
}

// §2/§3 used to restate a runnable-looking syntax shape (`[ body ] 'NAME'
// DEF`, `[ a ] [ b ] truth SELECT`) as its own hand-typed
// backtick span, independent of the identical, generator-executed example a
// few sections later in §6 — exactly the "same fact in two places, only one
// of them cross-checked" pattern this repo's Phase-2 alignment pass looks
// for. `{ body } 'NAME' DEF` drifted from real syntax there and went
// undetected because nothing ran it. Pulling the literal code from the §6
// entry by id closes that gap structurally: there is only one hand-typed
// copy of the fact, and it is the one the generator already executes.
function canonicalExampleCode(id) {
  const found = canonicalExamples.find((entry) => entry.id === id);
  if (!found) fail(`no canonicalExamples entry with id ${JSON.stringify(id)}`);
  return found.code;
}

function renderCanonicalExamples() {
  return canonicalExamples
    .map((example) => {
      const json = expectOk(example.code);
      return `- ${example.title}\n  \`${example.code}\` → ${renderResult(json)}`;
    })
    .join('\n');
}

/** The display of an executed §6 example, by id — the one hand-typed copy. */
function canonicalExampleDisplay(id) {
  return expectOk(canonicalExampleCode(id)).stackDisplay.join('  ');
}

function renderContractDeclarations() {
  return contractDeclarations
    .map((entry) => {
      const checked = expectCheckOk(entry.code);
      const decls = checked.contractDecls;
      if (!decls || decls.outcome !== 'value') {
        fail(`declaration must verify (contractDecls.outcome value), got ${JSON.stringify(decls)}: ${entry.code}`);
      }
      const run = expectOk(entry.code);
      if (run.contractDecls?.outcome !== 'value') fail(`compute must carry the verified declaration: ${entry.code}`);
      return [
        `- ${entry.title}`,
        '  ```ajisai',
        ...entry.code.split('\n').map((line) => `  ${line}`),
        '  ```',
        `  \`check\` → exit 0, \`contractDecls: { outcome: "value", gapSummary: ${JSON.stringify(decls.gapSummary)} }\`; \`compute\` → ${renderResult(run)}, with the same \`contractDecls\`.`,
      ].join('\n');
    })
    .join('\n');
}

function renderCommonErrors() {
  return commonErrors
    .map((entry) => {
      const json = expectError(entry.code);
      const d = json.diagnosis;
      // The stable half of a next-check is its code; the display text is
      // localized and free to be reworded.
      const firstCheck = d.nextChecks[0]?.code ?? '';
      const candidates = d.candidates?.length
        ? ` \`diagnosis.candidates: ${JSON.stringify(d.candidates)}\`.`
        : '';
      return [
        `- **${entry.title}** — \`${entry.code}\``,
        `  → exit 1, \`message: ${JSON.stringify(json.message)}\`, \`diagnosis: { when: "${d.when}", why: "${d.why}" }\`,`,
        `  \`aiDiagnostic: { category: "${json.aiDiagnostic.category}"${json.aiDiagnostic.repair ? `, repair: "${json.aiDiagnostic.repair}"` : ''} }\`, first nextCheck code: \`${firstCheck}\`.${candidates}`,
        `  Fix: ${entry.fix}`,
      ].join('\n');
    })
    .join('\n');
}

function renderSilentMistakes() {
  return silentMistakes
    .map((entry) => {
      const wrong = expectOk(entry.wrong);
      const right = expectOk(entry.right);
      if (wrong.stackDisplay.join(' ') === right.stackDisplay.join(' ')) {
        fail(`silent mistake ${JSON.stringify(entry.wrong)} no longer differs from the correct form`);
      }
      return [
        `- **${entry.title}** — both of these succeed (exit 0):`,
        `  \`${entry.wrong}\` → stack \`${wrong.stackDisplay.join(' ')}\``,
        `  \`${entry.right}\` → stack \`${right.stackDisplay.join(' ')}\``,
        `  Fix: ${entry.fix}`,
      ].join('\n');
    })
    .join('\n');
}

function renderForbiddenPatterns() {
  return forbiddenPatterns
    .map((entry) => {
      expectError(entry.code); // verified: really rejected by the implementation
      return `- **${entry.pattern}** (\`${entry.code}\` fails) — ${entry.why}`;
    })
    .join('\n');
}

function verifiedNilSection() {
  // Verify the documented NIL behavior against the real CLI before writing it.
  const projected = expectOk('-1 SQRT');
  if (projected.stackDisplay.join(' ') !== 'NIL') fail('a negative radicand must project to NIL');
  const event = projected.errorFlowTrace.find((e) => e.kind === 'nilProduced');
  if (!event || event.absence?.reason !== 'domainMiss') fail('nilProduced trace event missing');
  const fallback = expectOk("-1 SQRT 'S' BIND 99 S S NIL? SELECT");
  if (fallback.stackDisplay.join(' ') !== '99/1') fail('the fallback must replace NIL');
  // Lifted over a vector the same law projects lane by lane, so the top stays
  // a vector. Written with a scalar operand it would read as `NIL`, which
  // taught the collapse rather than the lane law.
  const lifted = expectOk('[ 4 -1 ] SQRT');
  if (lifted.stackDisplay.join(' ') !== '[ 2/1 NIL ]') {
    fail('a negative radicand must empty only its own lane');
  }
  // Division is not where NIL comes from: a quotient by zero is a number.
  const byZero = expectOk('100 0 DIV -5 0 DIV 0 0 DIV');
  if (byZero.stackDisplay.join(' ') !== '1/0 -1/0 0/0') fail('a quotient by zero is its sign over zero');
  return {
    reason: event.absence.reason,
    fallbackStack: fallback.stackDisplay[0],
    liftedStack: lifted.stackDisplay[0],
    byZeroStack: byZero.stackDisplay.join(' '),
  };
}

function verifiedExactnessSection() {
  // Comparison over the algebraic field is total: values built through
  // different histories compare equal when they denote the same real.
  const json = expectOk('8 SQRT 2 SQRT 2 SQRT ADD EQ');
  if (json.stackDisplay.join(' ') !== 'TRUE') fail('sqrt(8) must equal sqrt(2)+sqrt(2)');
  // POW answers inside the field and nowhere else: a cube root leaves it.
  const outside = expectOk('8 1/3 POW NIL-REASON');
  if (outside.stackDisplay.join(' ') !== "'domainMiss'") fail('8 1/3 POW must project domainMiss');
  return { decided: json.stackDisplay.join(' '), outside: outside.stackDisplay.join(' ') };
}

// ---------------------------------------------------------------------------
// Document assembly
// ---------------------------------------------------------------------------

function buildSkillMd() {
  const nil = verifiedNilSection();
  const exactness = verifiedExactnessSection();
  const wordRows = buildWordTable();
  const vocabulary = readVocabularyCounts();

  return `<!-- GENERATED FILE — do not edit by hand.
     Regenerate: npm run generate:skill   (verified against the ajisai CLI)
     Source of truth for semantics: SPECIFICATION.html.
     Generator: scripts/generate-skill-md.mjs -->

# Ajisai — Agent Writing Protocol (SKILL.md)

How to *write working Ajisai on the first try*. Every code line below was
executed by the generator against the real interpreter; results shown are
actual outputs. **If a word is not in the §9 table, it does not exist — when
unsure, grep §9 before writing.**

## 1. Run loop

\`\`\`sh
ajisai agent compute program.ajisai   # exit 0 = ok, 1 = language error, 2 = usage
ajisai agent check program.ajisai     # parse + resolve only, no execution
\`\`\`

Read the JSON in this order (contract: docs/dev/agent-cli-output-contract.md):
1. \`status\` / exit code. On ok: \`stackDisplay\` (final stack, bottom→top) and \`output\` (PRINT lines).
2. On error: \`diagnosis.why\` + \`diagnosis.where\` locate the failure; follow \`diagnosis.nextChecks\` in order; \`aiDiagnostic.category\` is the spec/outcomes.json error category, and \`aiDiagnostic.repair: "program"\` says the program is what to change (absent: an operand is wrong).
3. Even on ok, scan \`errorFlowTrace\` for \`nilProduced\` events if a NIL surprised you.

## 2. Minimal syntax

- Postfix, stack-based. Operands first, word last: \`1 2 ADD\`.
- Numbers are **exact rationals** (\`1/3\`, \`3.14\` → 157/50). No floats. Display shows \`3/1\` for 3.
- A scalar is written bare: \`${canonicalExampleCode('scalar')}\`, \`${canonicalExampleCode('div')}\`. Data lives in vectors: \`[ 1 2 3 ]\`, and vectors nest for ragged and grouped data. \`[ 42 ]\` is a one-element *vector*, not another way to write 42 — arithmetic broadcasts it like a scalar, but a Word that takes an *element* (\`PUT\`, \`GET\`, \`INDEX-OF\`) stores or reads the vector itself, and nothing errors (§7). Write scalars bare and the question never arises.
- Strings: \`'single quotes'\` (a value domain of its own, not a vector of codepoints). Booleans: \`TRUE\` / \`FALSE\`. Absence: \`NIL\`.
- Code blocks are quoted programs passed to MAP / FILTER / FOLD / DEF, written as an ordinary Vector (§6) — there is no separate block bracket. SELECT is not among them: it takes values, not code.
- Named data is a Record, built by \`RECORD\` from a Vector of keys and a Vector of values: \`${canonicalExampleCode('record-basic')}\`. It is not a Vector and is never code. It displays as \`${canonicalExampleDisplay('record-basic')}\` — the keys, the values and the Word that joins them, which is also valid source that rebuilds it.
- Define a user word with a body Vector, then a \`'NAME'\` string, then \`DEF\`, then call \`NAME\`: \`${canonicalExampleCode('def-basic')}\` (§6). Words are case-insensitive (canonicalized to upper case).
- Declare a user word's contract on a \`#:contract\` comment line — \`#:contract DOUBLE inputs=1 outputs=1 purity=pure\` — and \`check\` verifies it against the body before anything runs; \`compute\` runs the same check first and refuses a program whose declaration is violated (§2a).
- Comments: \`#\` to end of line.
- Every Word consumes the operands it reads. To use a value more than once, name it with \`BIND\` and read the name: \`5 'N' BIND N N 1 ADD\` leaves \`5/1  6/1\`.
- One word does one thing to the stack; there are **no** DUP/SWAP-style shufflers (§8).

## 2a. Declaring a contract (\`#:contract\`)

One comment line per Word, written in the keys and values of a contract Record
(the same vocabulary \`word_contract\` / \`CONTRACT\` answer in):

\`#:contract NAME [inputs=N] [outputs=N] [purity=pure|effectful] [partiality=total|partial|projecting] [field=closed|leaving] [determinism=deterministic|stateRelative|hostRelative] [cost steps=C numeric=C collection=C]\`
with each cost class \`C\` one of \`const\` \`linear\` \`superlinear\` \`unbounded\`.
\`field=closed\` promises the Word never answers a point over zero (\`1/0\` \`-1/0\`
\`0/0\`) from operands holding none — only \`DIV\`, \`POW\`, \`NUM\` and a literal over
zero leave the field, and a \`DIV\` by a non-zero literal (\`2 DIV\`) or a \`POW\` to a
non-negative literal (\`2 POW\`) does not. Inside \`closed\` code the field laws
(distributivity, \`x x SUB\` = 0) hold; a \`leaving\` report's \`fieldExits\` names
where the body leaves (LANG.CONTRACT.FIELD).

\`inputs\`/\`outputs\` must equal what the body does; every other key is an upper
bound the body must not exceed; a key left out is not checked. The check is
conservative: a body inference cannot read is reported as *cannot verify*
(a \`note\` with a \`gap.*\` code), never as a false violation. To write a
declaration without guessing, run \`infer_contracts\` first and paste its
\`suggested\` line.

${renderContractDeclarations()}

## 3. Control and iteration

- Branch: the two candidates, then the truth that chooses between them, then \`SELECT\`: \`${canonicalExampleCode('select-basic')}\` (§6). Both candidates are values the program already built, so neither is skipped and nothing is evaluated by SELECT itself. The choice is made lane by lane, so a Vector of truths branches a whole Vector at once: \`${canonicalExampleCode('select-lanes')}\`. An absent truth chooses neither and answers that same absence.
- Iterate data, not counters: \`MAP\` / \`FILTER\` / \`FOLD\` with block operands (examples in §6). \`FOLD\` requires an explicit initial value: \`${canonicalExampleCode('fold')}\`. The block leaves **exactly one** value — the mapped element, the truth, the next accumulator; leaving none or a surplus is a \`blockContractViolation\` (§7).
- Budget: a block iteration costs one step per element for every Word the block runs (literals are free), so under the MCP profile's 100,000-step budget a one-Word block (\`[ ADD ] FOLD\`) walks 100,000 elements, a two-Word block (\`[ 3 MUL 1 ADD ] MAP\`) 50,000, a three-Word block 33,000 — tens of thousands of elements divided by the Words in the block, not more. Beyond that, write the operation on whole vectors — \`V 3 MUL 1 ADD\` over 100,000 lanes is two steps — and leave no large intermediate on the stack.
- No recursion: \`DEF\` refuses a word whose body names itself, directly or through other user words (a diagnosed error at definition time, not at the call). Repetition is expressed only through MAP / FILTER / FOLD / SCAN over an already-finite vector.

## 4. NIL — absence is a value, not an exception

Failed partial operations *project to NIL*: \`-1 SQRT\` succeeds (exit 0) and
pushes \`NIL\` (reason: \`${nil.reason}\`). The projection is recorded in
\`errorFlowTrace\` as a \`nilProduced\` event with a full diagnosis, and the NIL
value itself carries \`semantics.absence.reason\` on the stack.

Division by zero is **not** a projection: every number is a pair over a
non-negative denominator, and a quotient by zero is the dividend's sign over
zero — \`100 0 DIV -5 0 DIV 0 0 DIV\` → stack \`${nil.byZeroStack}\`. The three
points \`1/0\`, \`-1/0\`, \`0/0\` are numbers: they add, multiply, divide and
compare by \`EQ\`. \`0/0\` absorbs everything it meets and has no place in the
order, so \`LT\` / \`GT\` / \`MIN\` / \`MAX\` / \`SORT\` project \`domainMiss\` on it.

- Provide a fallback with \`BIND\`, \`NIL?\` and \`SELECT\`: \`-1 SQRT 'S' BIND 99 S S NIL? SELECT\` → stack \`${nil.fallbackStack}\`. \`NIL?\` consumes its subject like every Word and answers whether it was absent, which is exactly where \`SELECT\` wants the truth — so name the subject once and read it twice: the phrase reads "S, or the fallback if S is absent".
- Over a vector the projection is **per lane, not per value**: \`[ 4 -1 ] SQRT\` → stack \`${nil.liftedStack}\`. The lane that had no root is the only one emptied.
- That makes the top a vector, not a NIL, so \`NIL?\` — which asks about the whole value — answers FALSE and the fallback is not chosen. Recover a lifted result inside the vector, not around it.
- NIL flows through later operations (NIL projection rule); check for it where it matters instead of letting it propagate to the end.

## 5. Exactness — comparison decides over the algebraic field

Numbers are exact rationals, closed under \`SQRT\`. Arithmetic never rounds,
coefficients are arbitrary-precision, and **every comparison of two scalars
built from rationals and \`SQRT\` decides**: there is no budget, no refinement
limit, and no undecided outcome.

\`\`\`ajisai
8 SQRT 2 SQRT 2 SQRT ADD EQ   # √8 vs √2+√2
\`\`\`

→ stack \`${exactness.decided}\` (exit 0). Values built through different
histories are the same value when they denote the same real.

That field is the whole numeric domain. \`POW\` answers inside it — an integer
exponent, or \`p/2\` over a non-negative rational — and projects NIL for any
other exponent rather than leave it:

\`\`\`ajisai
8 1/3 POW NIL-REASON
\`\`\`

→ stack \`${exactness.outside}\` (exit 0). Truth has three values: \`TRUE\`,
\`FALSE\`, and the logical UNKNOWN, which is what a NIL operand reads as in a
truth position (§4) — no comparison produces it of its own. An operation that
cannot produce a value produces NIL (§4); a malformed one raises an error.

## 6. Canonical examples (all verified by the generator)

${renderCanonicalExamples()}

## 7. Common errors — actual CLI output, and the fix

${renderCommonErrors()}

These raise. The next one does not — it succeeds and answers something other
than it looks like it answers, which is the harder kind to notice:

${renderSilentMistakes()}

## 8. Forbidden patterns (each verified to fail)

${renderForbiddenPatterns()}

## 9. Word quick reference

Generated from \`docs/word-manifest.json\` — the complete inventory:
${vocabulary.canonicalWords} canonical Words in one flat Core dictionary, of which
${vocabulary.semanticKernelWords} form the Semantic Kernel and ${vocabulary.standardWords} are Standard Words. Both are
ordinary Core Words called by their plain names; the split is a design
classification, not a namespace. A word absent here does not exist. There is
no module system and nothing to import.

| word | family | summary |
|---|---|---|
${wordRows.join('\n')}
`;
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

const content = buildSkillMd();

if (process.argv.includes('--stdout') && !process.argv.includes('--check')) {
  process.stdout.write(content);
} else {
  writeOrCheck(
    'skill-md',
    [{
      path: 'SKILL.md',
      content,
      missing: 'SKILL.md is missing; run `npm run generate:skill`',
      stale: 'SKILL.md is stale relative to the sources/CLI; run `npm run generate:skill` and commit the result',
    }],
    { current: 'SKILL.md is up to date.', wrote: `wrote ${resolve(repoRoot, 'SKILL.md')}` },
  );
}
