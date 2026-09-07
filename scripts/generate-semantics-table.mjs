#!/usr/bin/env node
// Generate docs/semantics-table.json: the exhaustive (Word x input-domain
// tuple) -> outcome-category table (docs/dev/
// outcome-space-bijection-work-order-2026-09.md, Phase 3; domains were
// originally chosen by type in docs/dev/competitive-advantage-work-order-2026-08.md
// Phase 2).
//
// 65 Words in one flat dictionary with no imports means the language's whole
// input/outcome surface is finite. Excluding the three variable/control-arity
// Words (COLLECT, COND, OR-NIL) and the KEEP modifier leaves 61 Words with a
// fixed integer arity; every (Word, domain tuple) pair is run through the
// real `ajisai` CLI and its outcome recorded as a stable id — never the
// human-readable `message`, which can be reworded without changing meaning.
//
// Domain representatives are chosen by *outcome*, not by type (Phase 3): each
// one exists to reach a specific declared condition, recorded as its
// `motivatedBy`. A domain with no motivation is not added — see the pitfall
// notes below for the ones considered and rejected.
//
// Usage:
//   node scripts/generate-semantics-table.mjs            # write docs/semantics-table.json
//   node scripts/generate-semantics-table.mjs --check    # fail if it is stale
//   AJISAI_BIN=/path/to/ajisai ...                        # override CLI binary

import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { cpus, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const outputPath = resolve(repoRoot, 'docs/semantics-table.json');
const wordsPath = resolve(repoRoot, 'spec/words.json');

function fail(message) {
  console.error(`[semantics-table] ${message}`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// CLI harness (mirrors scripts/generate-skill-md.mjs's resolveAjisaiBin/
// runSnippet shape).
// ---------------------------------------------------------------------------

function resolveAjisaiBin() {
  if (process.env.AJISAI_BIN) {
    if (!existsSync(process.env.AJISAI_BIN)) fail(`AJISAI_BIN not found: ${process.env.AJISAI_BIN}`);
    return process.env.AJISAI_BIN;
  }
  const debugBin = resolve(repoRoot, 'rust/target/debug/ajisai');
  if (!existsSync(debugBin)) {
    console.error('[semantics-table] building ajisai CLI (cargo build --bin ajisai)...');
    execFileSync('cargo', ['build', '--bin', 'ajisai'], {
      cwd: resolve(repoRoot, 'rust'),
      stdio: ['ignore', 'inherit', 'inherit'],
    });
  }
  if (!existsSync(debugBin)) fail('ajisai CLI binary not found after build');
  return debugBin;
}

// ---------------------------------------------------------------------------
// Domain representatives (work order Phase 3, §3.4 as corrected against the
// real CLI — every claim below was checked with a live probe, not assumed).
//
// `motivatedBy` names the outcome id(s) this domain exists to reach. A domain
// with none is a *carried-over type representative*, standing in for one of
// the original six (scalar/boolean/string/vector/nil/codeblock); Phase 3
// does not add a new domain motivated by nothing (pitfall A).
//
// Order and source text are fixed by this file and must not change casually:
// the order fixes the lexicographic order of domain tuples, which the
// committed table's cell order depends on.
//
// Domains considered and rejected, with the reason (kept here so the
// rejection is not silently rediscovered):
//
//   - `scalarFraction` ('1 2 /'), meant to reach `nonInteger`: RANDOM and PUT
//     are the only two Words declaring `nonInteger`, and both raise
//     `structureError` for a fractional operand instead — confirmed live
//     (`1 2 / 1 RANDOM`, `[ 1 2 3 ] 1 2 / 9 PUT`). `nonInteger` is currently
//     unreachable by any input; this is an implementation-vs-registry gap to
//     report, not a domain to add (Phase 3 does not fix `rust/`).
//   - `vectorRagged` ('[ [ 1 ] [ 2 3 ] ]'), meant to reach `shapeMismatch`:
//     a ragged vector broadcasts element-wise against a same-length flat
//     vector instead of raising (confirmed: `[ [ 1 ] [ 2 3 ] ] [ 1 2 ] ADD`
//     answers a value). `shapeMismatch` is reached far more directly by two
//     *flat* vectors of different lengths, which is what `vectorTriple`
//     below is for.
//   - `codeBlockFails` ('{ 1 0 / }'), meant to reach the retired
//     `NilReason::ExecutionFailure`: that reason no longer exists (Phase 1
//     deleted it as unreachable), and separately `{ }` is no longer valid
//     source syntax at all (confirmed: `{ 1 0 / }` is a MalformedSource
//     parse error, not a CodeBlock value) — code and data share `[ ]` since
//     the CodeBlock/Vector unification. Both premises this domain was
//     designed around are gone.
//   - `textEmpty`, `textNumeric`, `vectorEmpty`, `vectorWithNil`: none names
//     a declared condition beyond what `textShort`/`vectorPair` already
//     reach — they would answer `value` everywhere, which is worth knowing
//     but is not a `motivatedBy` claim this file can make honestly.
//
// The one correction to an existing domain: `codeblock` used `'{ 1 }'`,
// which is the same dead syntax above — every cell that domain touched in
// the committed (pre-Phase-3) table was actually testing a parse error, not
// a CodeBlock value. Replaced with `'[ 1 ]'`.
// ---------------------------------------------------------------------------

const DOMAINS = [
  { id: 'scalarOne', source: '1', motivatedBy: [] },
  { id: 'scalarZero', source: '0', motivatedBy: ['divisionByZero'] },
  { id: 'scalarNegative', source: '1 NEG', motivatedBy: ['domainMiss'] },
  { id: 'scalarLarge', source: '999', motivatedBy: ['indexOutOfBounds'] },
  { id: 'booleanTrue', source: 'TRUE', motivatedBy: [] },
  { id: 'textShort', source: "'a'", motivatedBy: [] },
  { id: 'vectorPair', source: '[ 1 2 ]', motivatedBy: [] },
  { id: 'vectorTriple', source: '[ 1 2 3 ]', motivatedBy: ['shapeMismatch'] },
  { id: 'vectorHuge', source: '[ 0 1000001 ]', motivatedBy: ['spaceExhausted'] },
  { id: 'nilLiteral', source: 'NIL', motivatedBy: ['literal'] },
  { id: 'codeBlock', source: '[ 1 ]', motivatedBy: [] },
];

function* domainTuples(arity) {
  if (arity === 0) {
    yield [];
    return;
  }
  for (const head of DOMAINS) {
    for (const rest of domainTuples(arity - 1)) {
      yield [head, ...rest];
    }
  }
}

// ---------------------------------------------------------------------------
// Word selection (Step 2.2, pitfalls A/B). `stack.inputs` is a plain integer
// for every Word except COLLECT/COND/OR-NIL (a JSON string: "variable" or
// "control" in the current spec/words.json); KEEP has a numeric arity (0) but
// is a modifier applied to the next Word, not a Word to expand on its own.
// This is not a hardcoded list (Phase 3 pitfall E): whichever Words currently
// have non-numeric `stack.inputs` are excluded, whatever their names are.
// ---------------------------------------------------------------------------

function loadWords() {
  const parsed = JSON.parse(readFileSync(wordsPath, 'utf8'));
  if (!Array.isArray(parsed.entries) || parsed.entries.length === 0) {
    fail('spec/words.json has no entries');
  }
  return parsed.entries;
}

function arityExclusionReason(word) {
  const inputs = word.stack.inputs;
  if (inputs === 'variable') return 'variableArity';
  if (inputs === 'control') return 'controlArity';
  fail(`${word.name}: unexpected non-numeric stack.inputs ${JSON.stringify(inputs)}`);
}

function selectWords(words) {
  const excluded = [];
  const domainWords = [];
  for (const word of words) {
    if (word.name === 'KEEP') {
      excluded.push({ word: word.name, reason: 'modifierNotWord' });
      continue;
    }
    if (typeof word.stack.inputs !== 'number') {
      excluded.push({ word: word.name, reason: arityExclusionReason(word) });
      continue;
    }
    domainWords.push(word);
  }
  return { excluded, domainWords };
}

// ---------------------------------------------------------------------------
// Outcome classification (Step 2.2, pitfall D). Never `message`: it is
// human-readable prose and can be reworded without the underlying category
// changing, which would make the committed table (and its CI gate) fail on
// a wording change rather than a behavior change.
// ---------------------------------------------------------------------------

function classifyOutcome(json) {
  if (json.status === 'error') {
    // `aiDiagnostic.kind` is the fine per-condition `ErrorCategory` protocol
    // string (`"indexOutOfBounds"`, `"stackUnderflow"`, a Word's own declared
    // condition...); `diagnosis.why` is the coarse ~17-bucket `CauseClass`
    // (`"valueShape"`, `"index"`...). Classifying by `why` alone is what made
    // the pre-Phase-3 table collapse dozens of distinct declared conditions
    // into one `error:valueShape` bucket. `kind` is `null` only for a raw
    // tokenize-time failure that predates word resolution (confirmed:
    // rust/src/agent/api.rs and cli/mod.rs pass `category: None` to
    // `error_report` on that one path) — no domain-tuple program reaches it,
    // but the fallback keeps this generator from crashing if one ever does.
    const kind = json.aiDiagnostic?.kind;
    if (typeof kind === 'string' && kind !== '') {
      return `error:${kind}`;
    }
    const why = json.diagnosis?.why;
    if (typeof why !== 'string' || why === '') {
      fail(`error report has neither aiDiagnostic.kind nor diagnosis.why: ${JSON.stringify(json)}`);
    }
    return `error:${why}`;
  }
  const stack = Array.isArray(json.stack) ? json.stack : [];
  const top = stack.length > 0 ? stack[stack.length - 1] : null;
  if (top && top.type === 'nil') {
    const reason = top.semantics?.absence?.reason;
    if (typeof reason !== 'string' || reason === '') {
      fail(`NIL top-of-stack has no semantics.absence.reason: ${JSON.stringify(top)}`);
    }
    return `nil:${reason}`;
  }
  return 'value';
}

// ---------------------------------------------------------------------------
// One cell, spawned asynchronously so a worker pool (below) can run several
// at once. `spawn` (not `spawnSync`) is what makes that possible: the CLI
// process runs while Node's event loop keeps dispatching the next one.
// ---------------------------------------------------------------------------

function runCellAsync(ajisaiBin, scratchDir, counter, program) {
  return new Promise((resolveCell) => {
    const file = join(scratchDir, `cell-${counter}.ajisai`);
    writeFileSync(file, `${program}\n`);
    const proc = spawn(ajisaiBin, ['run', file, '--json']);
    let stdout = '';
    proc.stdout.on('data', (chunk) => {
      stdout += chunk;
    });
    proc.on('error', (err) => fail(`failed to spawn ajisai CLI: ${err.message}`));
    proc.on('close', () => {
      let json;
      try {
        json = JSON.parse(stdout);
      } catch {
        fail(`CLI stdout for ${JSON.stringify(program)} is not valid JSON:\n${stdout}`);
      }
      resolveCell(classifyOutcome(json));
    });
  });
}

// A fixed-size pool of workers pulling from a shared index, each awaiting its
// own cell before taking the next — never more than `concurrency` CLI
// processes alive at once. `results[i]` is written by whichever worker
// happens to process job `i`, so the array comes out in job order regardless
// of which worker finished which job when (Phase 3 pitfall B): the table's
// cell order is a property of `jobs`, not of completion timing.
async function runPool(jobs, concurrency) {
  const results = new Array(jobs.length);
  let next = 0;
  async function worker() {
    while (next < jobs.length) {
      const i = next++;
      results[i] = await jobs[i]();
    }
  }
  await Promise.all(Array.from({ length: Math.min(concurrency, jobs.length) }, worker));
  return results;
}

// ---------------------------------------------------------------------------
// Table assembly. Cell order: Word appearance order in spec/words.json, then
// domain-tuple lexicographic order — never `Object.keys`/`Map` insertion
// order, or worker completion order, either of which the `--check` mode's
// string comparison would silently depend on otherwise.
// ---------------------------------------------------------------------------

async function buildTable(ajisaiBin) {
  const { excluded, domainWords } = selectWords(loadWords());
  const scratchDir = mkdtempSync(join(tmpdir(), 'ajisai-semantics-'));
  let counter = 0;
  const specs = [];
  for (const word of domainWords) {
    for (const tuple of domainTuples(word.stack.inputs)) {
      const operands = tuple.map((domain) => domain.source);
      const program = [...operands, word.name].join(' ');
      specs.push({ word: word.name, inputs: tuple.map((domain) => domain.id), program });
    }
  }

  let cells;
  try {
    const jobs = specs.map(
      (spec, i) => () => runCellAsync(ajisaiBin, scratchDir, i, spec.program),
    );
    const outcomes = await runPool(jobs, cpus().length);
    cells = specs.map((spec, i) => ({ word: spec.word, inputs: spec.inputs, outcome: outcomes[i] }));
  } finally {
    rmSync(scratchDir, { recursive: true, force: true });
  }

  return {
    schemaVersion: 2,
    generator: 'scripts/generate-semantics-table.mjs',
    // The runtime ceilings this table assumes. A cell whose outcome depends
    // on a ceiling (`vectorHuge` reaching `spaceExhausted`) is only reliably
    // reproducible under the same profile — see MCP_README's "the playground
    // applies a different, looser profile" (Phase 3 pitfall C). This is the
    // native CLI's own built-in default (`AJISAI_BIN`/`AJISAI_REPO` unset),
    // which is what generates the committed table.
    profile: {
      source: 'nativeCliDefault',
      maxMaterializedElements: 1_000_000,
    },
    domains: DOMAINS,
    excluded,
    cells,
  };
}

const table = await buildTable(resolveAjisaiBin());
const json = `${JSON.stringify(table, null, 2)}\n`;

if (process.argv.includes('--check')) {
  const existing = existsSync(outputPath) ? readFileSync(outputPath, 'utf8') : '';
  if (existing !== json) {
    fail('docs/semantics-table.json is stale. Run `npm run semantics:table` and commit the result.');
  }
  console.log(`[semantics-table] docs/semantics-table.json is up to date (${table.cells.length} cells).`);
} else {
  writeFileSync(outputPath, json);
  console.log(`[semantics-table] wrote ${table.cells.length} cells to docs/semantics-table.json`);
}
