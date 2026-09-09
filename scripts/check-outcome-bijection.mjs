#!/usr/bin/env node
// Gate the outcome registry (spec/outcomes.json) from both directions
// (docs/dev/auditable-kernel-work-order-2026-09.md Phase 2, picking up
// docs/dev/outcome-space-bijection-work-order-2026-09.md Phase 4):
//
//   soundness    — every outcome docs/semantics-table.json (or a witness in
//                  this file) actually observes resolves to a registered id.
//   non-vacuity  — every id spec/outcomes.json declares has at least one
//                  observed occurrence, in the table or in a witness.
//
// Neither half alone is satisfiable by a registry that lies in the other
// direction, so both are checked and neither has an exemption list: an id
// with no witness is a candidate for deletion, not for an exception (Phase 2
// pitfall C / Phase 4 pitfall C of the bijection work order).
//
// A witness in spec/outcome-witnesses.json is *executed*, not string-matched
// (Phase 2 pitfall B / Phase 4 pitfall B): this script spawns the real
// `ajisai` CLI for every entry and classifies its actual JSON output the same
// way scripts/generate-semantics-table.mjs classifies a table cell. A
// witness file that only asserted "this id exists" without running anything
// would go silently stale the day a raise site's condition changed.
//
// This also settles the question scripts/check-unreachable-contract.mjs's
// own doc comment declined to answer: it cannot tell a live `errorWhen`
// condition from a dead one by name, because Rust error prose and camelCase
// identifiers never match by grep. Witnessing by execution does not have
// that problem — a condition with no witness anywhere is provably dead.
//
// Usage:
//   node scripts/check-outcome-bijection.mjs
//   AJISAI_BIN=/path/to/ajisai ...   # override CLI binary

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const read = (path) => readFileSync(resolve(repoRoot, path), 'utf8');

const errors = [];
const fail = (message) => errors.push(message);

// ---------------------------------------------------------------------------
// CLI harness (mirrors scripts/generate-semantics-table.mjs's
// resolveAjisaiBin exactly, minus the worker pool this script has no use
// for — every witness runs once).
// ---------------------------------------------------------------------------

function resolveAjisaiBin() {
  if (process.env.AJISAI_BIN) {
    if (!existsSync(process.env.AJISAI_BIN)) {
      console.error(`[outcome-bijection] AJISAI_BIN not found: ${process.env.AJISAI_BIN}`);
      process.exit(1);
    }
    return process.env.AJISAI_BIN;
  }
  const debugBin = resolve(repoRoot, 'rust/target/debug/ajisai');
  if (!existsSync(debugBin)) {
    console.error('[outcome-bijection] building ajisai CLI (cargo build --bin ajisai)...');
    execFileSync('cargo', ['build', '--bin', 'ajisai'], {
      cwd: resolve(repoRoot, 'rust'),
      stdio: ['ignore', 'inherit', 'inherit'],
    });
  }
  if (!existsSync(debugBin)) {
    console.error('[outcome-bijection] ajisai CLI binary not found after build');
    process.exit(1);
  }
  return debugBin;
}

// ---------------------------------------------------------------------------
// Outcome classification — the same rule
// scripts/generate-semantics-table.mjs's classifyOutcome applies, kept as an
// intentionally separate copy: that script runs its whole (expensive) table
// build as top-level module code the moment it is imported, so importing it
// here would rebuild the committed table as a side effect of a bijection
// check. Twenty lines duplicated once is cheaper than that coupling. Any
// change to one classifier belongs in the other too.
// ---------------------------------------------------------------------------

function classifyOutcome(json) {
  if (json.status === 'error') {
    const kind = json.aiDiagnostic?.kind;
    if (typeof kind === 'string' && kind !== '') {
      return `error:${kind}`;
    }
    const why = json.diagnosis?.why;
    if (typeof why !== 'string' || why === '') {
      throw new Error(`error report has neither aiDiagnostic.kind nor diagnosis.why: ${JSON.stringify(json)}`);
    }
    return `error:${why}`;
  }
  const stack = Array.isArray(json.stack) ? json.stack : [];
  const top = stack.length > 0 ? stack[stack.length - 1] : null;
  if (top && top.type === 'nil') {
    const reason = top.semantics?.absence?.reason;
    if (typeof reason !== 'string' || reason === '') {
      throw new Error(`NIL top-of-stack has no semantics.absence.reason: ${JSON.stringify(top)}`);
    }
    return `nil:${reason}`;
  }
  return 'value';
}

function runProgram(ajisaiBin, scratchDir, counter, source, profile) {
  const file = join(scratchDir, `witness-${counter}.ajisai`);
  writeFileSync(file, `${source}\n`);
  const args = ['run', file, '--json'];
  if (profile?.stepLimit !== undefined) {
    args.push('--step-limit', String(profile.stepLimit));
  }
  // `spawnSync`, not `execFileSync`: a language ERROR exits 1 (the CLI's own
  // documented exit code), which `execFileSync` treats as a thrown failure
  // even though its stdout is exactly the JSON diagnosis this script needs.
  const result = spawnSync(ajisaiBin, args, { encoding: 'utf8' });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0 && result.status !== 1) {
    throw new Error(`exit code ${result.status} (CLI usage error?): ${result.stderr}`);
  }
  return JSON.parse(result.stdout);
}

// ---------------------------------------------------------------------------
// spec/outcomes.json — the registry both halves check against.
// ---------------------------------------------------------------------------

const outcomes = JSON.parse(read('spec/outcomes.json'));
const registryNilIds = new Set(outcomes.nilReasons.map((r) => r.id));
const registryErrorIds = new Set(outcomes.errorCategories.map((c) => c.id));

function idFromOutcome(outcome) {
  // 'value' names no id at all; 'nil:<reason>' / 'error:<category>' each
  // name one in their own namespace.
  if (outcome === 'value') return null;
  const [namespace, id] = outcome.split(/:(.*)/s);
  return { namespace, id };
}

function isRegistered(outcome) {
  const parsed = idFromOutcome(outcome);
  if (parsed === null) return true;
  if (parsed.namespace === 'nil') return registryNilIds.has(parsed.id);
  if (parsed.namespace === 'error') return registryErrorIds.has(parsed.id);
  return false;
}

// ---------------------------------------------------------------------------
// Soundness: every outcome docs/semantics-table.json actually observed
// resolves to a registered id. (The witness half of soundness — a witness's
// *observed* outcome matching a registered id — falls out of the
// non-vacuity loop below, since a witness that observes an unregistered
// outcome also fails to match its own declared `expect`.)
// ---------------------------------------------------------------------------

const table = JSON.parse(read('docs/semantics-table.json'));
const tableOutcomes = new Set(table.cells.map((cell) => cell.outcome));

const unsoundTableOutcomes = [...tableOutcomes].filter((outcome) => !isRegistered(outcome)).sort();
if (unsoundTableOutcomes.length > 0) {
  fail(
    `docs/semantics-table.json observes outcome(s) spec/outcomes.json does not declare: ` +
      JSON.stringify(unsoundTableOutcomes),
  );
}

// ---------------------------------------------------------------------------
// spec/outcome-witnesses.json — structural validation, then execution.
// ---------------------------------------------------------------------------

const witnessDoc = JSON.parse(read('spec/outcome-witnesses.json'));
if (witnessDoc.schemaVersion !== 1) {
  fail(`spec/outcome-witnesses.json: unsupported schemaVersion ${JSON.stringify(witnessDoc.schemaVersion)}`);
}
const witnesses = Array.isArray(witnessDoc.witnesses) ? witnessDoc.witnesses : [];
if (witnesses.length === 0) {
  fail('spec/outcome-witnesses.json declares zero witnesses');
}

const idPattern = /^[a-z][a-zA-Z0-9]*$/;
const outcomePattern = /^(value|nil:[a-z][a-zA-Z0-9]*|error:[a-z][a-zA-Z0-9]*)$/;

for (const [i, w] of witnesses.entries()) {
  const where = `spec/outcome-witnesses.json[${i}]`;
  if (typeof w.id !== 'string' || !idPattern.test(w.id)) {
    fail(`${where}: "id" must be a lowerCamelCase identifier, got ${JSON.stringify(w.id)}`);
  }
  if (typeof w.source !== 'string' || w.source.length === 0) {
    fail(`${where}: "source" must be a non-empty string`);
  }
  if (typeof w.expect !== 'string' || !outcomePattern.test(w.expect)) {
    fail(`${where}: "expect" must be "value", "nil:<reason>" or "error:<category>", got ${JSON.stringify(w.expect)}`);
  }
  if (typeof w.rationale !== 'string' || w.rationale.length === 0) {
    fail(`${where}: "rationale" must be a non-empty string`);
  }
  if (w.profile !== undefined) {
    if (typeof w.profile !== 'object' || w.profile === null) {
      fail(`${where}: "profile" must be an object when present`);
    } else if (w.profile.stepLimit !== undefined && !(Number.isInteger(w.profile.stepLimit) && w.profile.stepLimit > 0)) {
      fail(`${where}: "profile.stepLimit" must be a positive integer`);
    }
  }
}

if (errors.length > 0) {
  // A structurally broken witness file cannot be executed meaningfully —
  // report what is wrong and stop before spawning any CLI processes.
  for (const e of errors) console.error(`[outcome-bijection] ${e}`);
  process.exit(1);
}

// Every id a witness *targets*, regardless of whether it turns out to
// execute correctly — used below to detect a witness that duplicates
// coverage the table already has (allowed) versus one that is simply wrong.
const witnessedIds = new Set(); // "nil:<reason>" / "error:<category>" of what each witness actually observed and matched
const ajisaiBin = resolveAjisaiBin();
const scratchDir = mkdtempSync(join(tmpdir(), 'ajisai-outcome-witness-'));

try {
  witnesses.forEach((w, i) => {
    let observed;
    try {
      const json = runProgram(ajisaiBin, scratchDir, i, w.source, w.profile);
      observed = classifyOutcome(json);
    } catch (e) {
      fail(`witness "${w.id}" (${where(w)}): failed to run or classify: ${e.message}`);
      return;
    }
    if (observed !== w.expect) {
      fail(
        `witness "${w.id}": expected ${JSON.stringify(w.expect)}, observed ${JSON.stringify(observed)} — ` +
          `either the program no longer reaches this outcome, or "expect" is stale`,
      );
      return;
    }
    if (!isRegistered(observed)) {
      fail(`witness "${w.id}": observed ${JSON.stringify(observed)}, which spec/outcomes.json does not declare`);
      return;
    }
    witnessedIds.add(observed);
  });
} finally {
  rmSync(scratchDir, { recursive: true, force: true });
}

function where(w) {
  return `id=${w.id}`;
}

// ---------------------------------------------------------------------------
// Non-vacuity: every registry id has at least one occurrence, in the table
// or in a witness that actually executed and matched. No exemption list —
// an id with neither is reported so it can be deleted or given a witness.
// ---------------------------------------------------------------------------

const nilIdsSeen = new Set(
  [...tableOutcomes, ...witnessedIds]
    .map(idFromOutcome)
    .filter((p) => p !== null && p.namespace === 'nil')
    .map((p) => p.id),
);
const errorIdsSeen = new Set(
  [...tableOutcomes, ...witnessedIds]
    .map(idFromOutcome)
    .filter((p) => p !== null && p.namespace === 'error')
    .map((p) => p.id),
);

const unwitnessedNilIds = [...registryNilIds].filter((id) => !nilIdsSeen.has(id)).sort();
if (unwitnessedNilIds.length > 0) {
  fail(`spec/outcomes.json's nilReasons have no witness (table or spec/outcome-witnesses.json): ${JSON.stringify(unwitnessedNilIds)}`);
}
const unwitnessedErrorIds = [...registryErrorIds].filter((id) => !errorIdsSeen.has(id)).sort();
if (unwitnessedErrorIds.length > 0) {
  fail(`spec/outcomes.json's errorCategories have no witness (table or spec/outcome-witnesses.json): ${JSON.stringify(unwitnessedErrorIds)}`);
}

// ---------------------------------------------------------------------------

if (errors.length > 0) {
  for (const e of errors) console.error(`[outcome-bijection] ${e}`);
  process.exit(1);
}
console.log(
  `[outcome-bijection] soundness holds (${tableOutcomes.size} distinct table outcomes) and non-vacuity holds ` +
    `(${registryNilIds.size} NIL reasons + ${registryErrorIds.size} error categories, ${witnesses.length} executed witnesses).`,
);
