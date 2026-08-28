#!/usr/bin/env node
// Generate SKILL.md — the thin "read this, then write Ajisai" protocol for
// AI agents — from machine sources, never by hand (docs/dev/
// ai-first-competitive-upgrade-instructions.md, Phase 2).
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

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const outputPath = resolve(repoRoot, 'SKILL.md');

function fail(message) {
  console.error(`[skill-md] ${message}`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// CLI harness
// ---------------------------------------------------------------------------

function resolveAjisaiBin() {
  if (process.env.AJISAI_BIN) {
    if (!existsSync(process.env.AJISAI_BIN)) fail(`AJISAI_BIN not found: ${process.env.AJISAI_BIN}`);
    return process.env.AJISAI_BIN;
  }
  const debugBin = resolve(repoRoot, 'rust/target/debug/ajisai');
  if (!existsSync(debugBin)) {
    console.error('[skill-md] building ajisai CLI (cargo build --bin ajisai)...');
    execFileSync('cargo', ['build', '--bin', 'ajisai'], {
      cwd: resolve(repoRoot, 'rust'),
      stdio: ['ignore', 'inherit', 'inherit'],
    });
  }
  if (!existsSync(debugBin)) fail('ajisai CLI binary not found after build');
  return debugBin;
}

const ajisaiBin = resolveAjisaiBin();
const scratchDir = mkdtempSync(join(tmpdir(), 'ajisai-skill-'));
process.on('exit', () => rmSync(scratchDir, { recursive: true, force: true }));

let snippetCounter = 0;
function runSnippet(code) {
  const file = join(scratchDir, `snippet-${snippetCounter++}.ajisai`);
  writeFileSync(file, `${code}\n`);
  const proc = spawnSync(ajisaiBin, ['run', file, '--json'], { encoding: 'utf8' });
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

function readRepo(path) {
  return readFileSync(resolve(repoRoot, path), 'utf8');
}

// Vocabulary counts read from the manifest, so the sentence introducing the
// table cannot drift from the table itself.
function readVocabularyCounts() {
  const manifest = JSON.parse(readRepo('docs/word-manifest.json'));
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
  const manifest = JSON.parse(readRepo('docs/word-manifest.json'));
  const words = JSON.parse(readRepo('spec/words.json'));
  if (words.migration?.completeInventory !== true) fail('spec/words.json inventory is incomplete');
  const contracts = new Map(words.entries.map((entry) => [entry.name, entry]));
  if (contracts.size !== words.entries.length) fail('spec/words.json contains duplicate canonical names');
  const rows = [];
  for (const entry of manifest.entries) {
    if (entry.kind === 'coreword') {
      const contract = contracts.get(entry.canonical);
      if (!contract) fail(`no contract found for coreword ${entry.surface}`);
      const syntax = contract.documentation.syntax ? ` — e.g. \`${contract.documentation.syntax}\`` : '';
      rows.push(`| \`${entry.surface}\` | ${entry.category} | ${contract.documentation.summary}${syntax} |`);
    } else if (entry.canonical) {
      const escaped = entry.surface.replace(/\|/g, '\\|');
      const concept = entry.canonical.replace(/\|/g, '\\|');
      // Only an alias is a shorthand *for* a Word you could have written out. A
      // delimiter, a directive, or a reserved marker is not: calling a reserved
      // marker "shorthand" would read as an invitation to use one.
      const note = {
        reserved_marker: `${concept} — reserved, never valid in source`,
        source_directive: `${concept} — consumed by the lexer, not a Word`,
        control_directive: `${concept} — only inside the construct that defines it`,
        delimiter_sugar: `${concept} — structural delimiter, not a Word`,
        literal_sugar: `${concept} — literal delimiter, not a Word`,
        input_helper: `${concept} — editor affordance, not a Word`,
      }[entry.kind] ?? `shorthand for \`${concept}\``;
      rows.push(`| \`${escaped}\` | ${entry.kind.replace(/_/g, ' ')} | ${note} |`);
    }
  }
  return rows;
}

// ---------------------------------------------------------------------------
// Curated, execution-verified snippet data
// ---------------------------------------------------------------------------

const canonicalExamples = [
  { title: 'Push a number (always inside a vector)', code: '[ 42 ]' },
  { title: 'Exact rational division — no floats, ever', code: '[ 1 ] [ 3 ] /' },
  { title: 'Elementwise vector arithmetic', code: '[ 1 2 3 ] [ 4 5 6 ] +' },
  { title: 'Scalar broadcast over a vector', code: '[ 5 ] [ 1 2 3 ] *' },
  { title: 'Remainder', code: '[ 10 ] [ 3 ] %' },
  { title: 'Comparison pushes a boolean', code: '1 2 <' },
  { title: 'Comparison lifts over vectors element-wise', code: '[ 1 2 ] [ 3 1 ] <' },
  { title: 'Range: one vector [ start end ] (inclusive)', code: '[ 0 5 ] RANGE' },
  { title: 'Range with step: [ start end step ]', code: '[ 0 10 2 ] RANGE' },
  { title: 'Fill a tensor: [ shape... value ]', code: '[ 2 2 7 ] FILL' },
  { title: 'MAP with a [ ] code block', code: '[ 0 4 ] RANGE [ [ 2 ] * ] MAP' },
  { title: 'FILTER keeps matching elements', code: '[ 0 10 ] RANGE [ 5 > ] FILTER' },
  { title: 'FOLD needs an explicit initial value', code: '[ 1 2 3 ] [ 0 ] [ + ] FOLD' },
  { title: 'ANY / ALL take predicate blocks', code: '[ 1 2 3 ] [ 1 > ] ANY' },
  { title: 'Define a user word: [ body ] then name, then DEF', code: "[ [ 1 ] [ 2 ] + ] 'MY-SUM' DEF MY-SUM" },
  {
    title: 'COND: value, then one [ ] of [ guard ] [ body ] pairs (use [ TRUE ] as else-guard)',
    code: "4 [ [ 0 GTE ] [ 'non-negative' PRINT ] [ TRUE ] [ 'negative' PRINT ] ] COND",
  },
  { title: 'Strings are bare \'...\' literals; CHARS/JOIN convert', code: "'hello' CHARS REVERSE JOIN" },
  { title: 'Cast a string to an exact number', code: "'42' NUM" },
  { title: 'PRINT pops and emits to output (not the stack)', code: '[ 1 2 3 ] PRINT' },
  { title: 'Sorting is a plain Core word', code: '[ 3 1 2 ] SORT' },
  { title: 'Exact square root takes a bare scalar', code: '2 SQRT' },
  { title: 'The KEEP modifier makes the next word non-consuming', code: '[ 5 ] KEEP PRINT' },
];

const commonErrors = [
  {
    title: 'Typo / unknown word',
    code: '[ 1 ] ADDD',
    fix: 'Grep §9 for the word you meant (here: `+` / `ADD`). Word names are upper-cased automatically.',
  },
  {
    title: 'Stack underflow: operands must be pushed first',
    code: '+',
    fix: 'Push both operands before the operator: `[ 1 ] [ 2 ] +`. Ajisai is postfix; there is no infix form.',
  },
  {
    title: 'FOLD without an initial value',
    code: '[ 1 2 3 ] [ + ] FOLD',
    fix: 'FOLD is `vector [ init ] [ op ] FOLD`: `[ 1 2 3 ] [ 0 ] [ + ] FOLD`.',
  },
  {
    title: 'COND clauses must be wrapped in a single [ ]',
    code: "5 [ 3 > ] [ 'big' PRINT ] COND",
    fix: "COND takes its clauses as one Vector, not a run of separate blocks: wrap them together, and give every body a guard — the else-branch is `[ TRUE ] [ ... ]`: `5 [ [ 3 > ] [ 'big' PRINT ] [ TRUE ] [ 'small' PRINT ] ] COND`.",
  },
  {
    title: 'COND guards must yield a boolean',
    code: 'TRUE [ [ [ 1 ] ] [ [ 2 ] ] ] COND',
    fix: 'The first block of each pair is a guard, not a value: it must leave TRUE/FALSE. Branch on a stack value with `[ x ] [ [ predicate ] [ body ] ... ] COND`.',
  },
  {
    title: 'Broadcast shape mismatch',
    code: '[ 1 2 ] [ 1 2 3 ] +',
    fix: 'Elementwise ops need equal or broadcastable shapes (scalar `[ 5 ]` broadcasts; `[2]` vs `[3]` does not).',
  },
  {
    title: 'NUM casts strings, not booleans',
    code: 'TRUE NUM',
    fix: "NUM accepts strings: `'42' NUM`. There is no boolean→number cast.",
  },
  {
    title: 'Old two-vector RANGE form',
    code: '[ 0 ] [ 5 ] RANGE',
    fix: 'RANGE takes one vector: `[ 0 5 ] RANGE` (or `[ start end step ]`).',
  },
  {
    title: 'Vector-wrapped string passed to a cast',
    code: "[ '42' ] NUM",
    fix: "String casts take the bare string: `'42' NUM`.",
  },
];

const forbiddenPatterns = [
  {
    pattern: 'DUP / SWAP / DROP / OVER / ROT',
    code: 'DUP',
    why: 'Forth-style stack shufflers do not exist. Use `KEEP` when the next word must retain its operands; consumption is the default.',
  },
  {
    pattern: 'IF / ELSE / THEN / WHILE',
    code: '[ 1 ] IF',
    why: 'No structured keywords. Branch with COND guard/body pairs; iterate with MAP / FILTER / FOLD or recursive user words.',
  },
  {
    pattern: 'Parentheses ( )',
    code: '( 1 2 )',
    why: 'Reserved; not valid in source. `[ ]` is the sole bracket, for vectors, code, and continued-fraction display alike.',
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
  // An algebraic slot's stack display is the SPEC §4.2.3 continued fraction,
  // truncated at a display budget: √2 ran to ~194 characters ending in `...]`,
  // which told a reader nothing about the value and left the impression that
  // an exact square root is a complicated object. `semantics.exactDisplay`
  // writes the same value short, and comes from the same verified `--json`
  // run — so this substitutes a shorter true rendering, never a claim the
  // interpreter did not make. The label says which rendering is being shown.
  const exact = (json.stack ?? []).map((slot) => slot?.semantics?.exactDisplay ?? null);
  if (exact.some(Boolean)) {
    const shown = exact.map((display, index) => display ?? json.stackDisplay[index]).join('  ');
    parts.push(`exact value: \`${shown}\` (the stack display is its continued fraction)`);
  } else if (json.stackDisplay.length > 0) {
    parts.push(`stack: \`${json.stackDisplay.join('  ')}\``);
  }
  if (parts.length === 0) parts.push('stack: (empty)');
  return parts.join('; ');
}

function renderCanonicalExamples() {
  return canonicalExamples
    .map((example) => {
      const json = expectOk(example.code);
      return `- ${example.title}\n  \`${example.code}\` → ${renderResult(json)}`;
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
        `  \`aiDiagnostic.recoverability: "${json.aiDiagnostic.recoverability}"\`, first nextCheck code: \`${firstCheck}\`.${candidates}`,
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
  const bubble = expectOk('[ 1 ] [ 0 ] DIV');
  if (bubble.stackDisplay.join(' ') !== 'NIL') fail('division by zero must bubble to NIL');
  const event = bubble.errorFlowTrace.find((e) => e.kind === 'nilProduced');
  if (!event || event.absence?.reason !== 'divisionByZero') fail('nilProduced trace event missing');
  const fallback = expectOk('[ 1 ] [ 0 ] DIV ^ [ 99 ]');
  if (fallback.stackDisplay.join(' ') !== '[ 99/1 ]') fail('^ fallback must replace NIL');
  return { reason: event.absence.reason, fallbackStack: fallback.stackDisplay[0] };
}

function verifiedExactnessSection() {
  // Comparison over the exact domain is total: values built through different
  // histories compare equal when they denote the same real.
  const json = expectOk('8 SQRT 2 SQRT 2 SQRT + =');
  if (json.stackDisplay.join(' ') !== 'TRUE') fail('sqrt(8) must equal sqrt(2)+sqrt(2)');
  return { decided: json.stackDisplay.join(' ') };
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
ajisai run program.ajisai --json     # exit 0 = ok, 1 = language error, 2 = usage
ajisai check program.ajisai --json   # parse + resolve only, no execution
\`\`\`

Read the JSON in this order (contract: docs/dev/agent-cli-output-contract.md):
1. \`status\` / exit code. On ok: \`stackDisplay\` (final stack, bottom→top) and \`output\` (PRINT lines).
2. On error: \`diagnosis.why\` + \`diagnosis.where\` locate the failure; follow \`diagnosis.nextChecks\` in order; \`aiDiagnostic.recoverability\` says what kind of change fixes it (\`fixProgram\` / \`fixInput\` / \`fixHost\` ...).
3. Even on ok, scan \`errorFlowTrace\` for \`nilProduced\` events if a NIL surprised you.

## 2. Minimal syntax

- Postfix, stack-based. Operands first, word last: \`[ 1 ] [ 2 ] +\`.
- Numbers are **exact rationals** (\`1/3\`, \`3.14\` → 157/50). No floats. Display shows \`3/1\` for 3.
- Data lives in vectors: \`[ 1 2 3 ]\`. Vectors nest for ragged and grouped data. A lone number like \`42\` is allowed but \`[ 42 ]\` is the idiomatic scalar.
- Strings: \`'single quotes'\` (a value domain of its own, not a vector of codepoints). Booleans: \`TRUE\` / \`FALSE\`. Absence: \`NIL\`.
- Code blocks: \`{ ... }\` — quoted programs passed to MAP / FILTER / FOLD / COND / DEF.
- User word: \`{ body } 'NAME' DEF\` then call \`NAME\`. Words are case-insensitive (canonicalized to upper case).
- Comments: \`#\` to end of line.
- One modifier, prefixing the *next word only*: \`KEEP\` (do not consume operands). Consumption is the default.
- One word does one thing to the stack; there are **no** DUP/SWAP-style shufflers (§8).

## 3. Control and iteration

- Branch: \`value { { guard } { body } { guard } { body } ... } COND\` — the clauses are one \`{ }\` of guard/body pairs. Guards see the value (it stays for each guard) and must leave TRUE/FALSE; use \`{ TRUE }\` as the final else-guard. The value remains on the stack after COND.
- Iterate data, not counters: \`MAP\` / \`FILTER\` / \`FOLD\` with \`{ }\` blocks (examples in §6). \`FOLD\` requires an explicit \`[ init ]\`.
- Predicates: \`ANY\` / \`ALL\` with a \`{ predicate }\` block.
- Recursion is allowed in user words (execution-step and depth limits apply; exceeding them is a diagnosed error, not a hang).

## 4. NIL — absence is a value, not an exception

Failed partial operations *bubble*: \`[ 1 ] [ 0 ] DIV\` succeeds (exit 0) and
pushes \`NIL\` (reason: \`${nil.reason}\`). The projection is recorded in
\`errorFlowTrace\` as a \`nilProduced\` event with a full diagnosis, and the NIL
value itself carries \`semantics.absence.reason\` on the stack.

- Provide a fallback with \`^\`: \`[ 1 ] [ 0 ] DIV ^ [ 99 ]\` → stack \`${nil.fallbackStack}\`.
- NIL flows through later operations (bubble rule); check for it where it matters instead of letting it propagate to the end.

## 5. Exactness — comparison always decides

Numbers are exact rationals, closed under \`SQRT\`. Arithmetic never rounds,
coefficients are arbitrary-precision, and **every comparison of two scalars
decides**: there is no budget, no refinement limit, and no undecided outcome.

\`\`\`ajisai
8 SQRT 2 SQRT 2 SQRT + =   # √8 vs √2+√2
\`\`\`

→ stack \`${exactness.decided}\` (exit 0). Values built through different
histories are the same value when they denote the same real. Truth is
two-valued: \`TRUE\` and \`FALSE\`, nothing else. An operation that cannot
produce a value produces NIL (§4); a malformed one raises an error.

## 6. Canonical examples (all verified by the generator)

${renderCanonicalExamples()}

## 7. Common errors — actual CLI output, and the fix

${renderCommonErrors()}

## 8. Forbidden patterns (each verified to fail)

${renderForbiddenPatterns()}

## 9. Word quick reference

Generated from \`docs/word-manifest.json\` — the complete inventory:
${vocabulary.canonicalWords} canonical Words in one flat Core dictionary, of which
${vocabulary.semanticKernelWords} form the Semantic Kernel and ${vocabulary.standardWords} are Standard Words. Both are
ordinary Core Words called by their plain names; the split is a design
classification, not a namespace. A word absent here does not exist. There is
no module system and nothing to import.

| word | category | summary |
|---|---|---|
${wordRows.join('\n')}
`;
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

const content = buildSkillMd();

if (process.argv.includes('--check')) {
  const committed = existsSync(outputPath) ? readFileSync(outputPath, 'utf8') : null;
  if (committed === null) fail('SKILL.md is missing; run `npm run generate:skill`');
  if (committed !== content) {
    fail('SKILL.md is stale relative to the sources/CLI; run `npm run generate:skill` and commit the result');
  }
  console.log('[skill-md] SKILL.md is up to date.');
} else if (process.argv.includes('--stdout')) {
  process.stdout.write(content);
} else {
  writeFileSync(outputPath, content);
  console.log(`[skill-md] wrote ${outputPath}`);
}
