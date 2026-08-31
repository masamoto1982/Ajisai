#!/usr/bin/env node
// Generate docs/semantics-table.json: the exhaustive (Word x input-domain
// tuple) -> outcome-category table (docs/dev/
// competitive-advantage-work-order-2026-08.md, Phase 2).
//
// 65 Words in one flat dictionary with no imports means the language's whole
// input/outcome surface is finite. Excluding the three variable/control-arity
// Words (COLLECT, COND, OR-NIL) and the KEEP modifier leaves 61 Words with a
// fixed integer arity; every (Word, domain tuple) pair is run through the
// real `ajisai` CLI and its outcome recorded as a stable id — never the
// human-readable `message`, which can be reworded without changing meaning.
//
// Usage:
//   node scripts/generate-semantics-table.mjs            # write docs/semantics-table.json
//   node scripts/generate-semantics-table.mjs --check    # fail if it is stale
//   AJISAI_BIN=/path/to/ajisai ...                        # override CLI binary

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
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
// Fixed domain representatives (Step 2.1). Order and source text are fixed by
// the work order and must not change: the order fixes the lexicographic
// order of domain tuples, which the committed table's cell order depends on.
// ---------------------------------------------------------------------------

const DOMAINS = [
  { id: 'scalar', source: '1' },
  { id: 'boolean', source: 'TRUE' },
  { id: 'string', source: "'a'" },
  { id: 'vector', source: '[ 1 2 ]' },
  { id: 'nil', source: 'NIL' },
  { id: 'codeblock', source: '{ 1 }' },
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
    const why = json.diagnosis?.why;
    if (typeof why !== 'string' || why === '') {
      fail(`error report has no diagnosis.why: ${JSON.stringify(json)}`);
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

function runCell(ajisaiBin, scratchDir, counter, program) {
  const file = join(scratchDir, `cell-${counter}.ajisai`);
  writeFileSync(file, `${program}\n`);
  const proc = spawnSync(ajisaiBin, ['run', file, '--json'], { encoding: 'utf8' });
  if (proc.error) fail(`failed to spawn ajisai CLI: ${proc.error.message}`);
  let json;
  try {
    json = JSON.parse(proc.stdout);
  } catch {
    fail(`CLI stdout for ${JSON.stringify(program)} is not valid JSON:\n${proc.stdout}`);
  }
  return classifyOutcome(json);
}

// ---------------------------------------------------------------------------
// Table assembly. Cell order: Word appearance order in spec/words.json, then
// domain-tuple lexicographic order — never `Object.keys`/`Map` insertion
// order, which the `--check` mode's string comparison would silently depend
// on otherwise.
// ---------------------------------------------------------------------------

function buildTable(ajisaiBin) {
  const { excluded, domainWords } = selectWords(loadWords());
  const scratchDir = mkdtempSync(join(tmpdir(), 'ajisai-semantics-'));
  let counter = 0;
  const cells = [];
  try {
    for (const word of domainWords) {
      for (const tuple of domainTuples(word.stack.inputs)) {
        const operands = tuple.map((domain) => domain.source);
        const program = [...operands, word.name].join(' ');
        const outcome = runCell(ajisaiBin, scratchDir, counter++, program);
        cells.push({
          word: word.name,
          inputs: tuple.map((domain) => domain.id),
          outcome,
        });
      }
    }
  } finally {
    rmSync(scratchDir, { recursive: true, force: true });
  }
  return {
    schemaVersion: 1,
    generator: 'scripts/generate-semantics-table.mjs',
    domains: DOMAINS.map((domain) => domain.id),
    excluded,
    cells,
  };
}

const table = buildTable(resolveAjisaiBin());
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
