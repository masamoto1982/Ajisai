#!/usr/bin/env node
// Predicted-vs-actual gate for `ajisai agent outcomes` (Phase 5,
// docs/dev/auditable-kernel-work-order-2026-09.md §5), pitfall D: "the
// predictor doesn't run the program, but verification does." For every
// witness in spec/outcome-witnesses.json (Phase 2's file — a witness is
// already "a source and its actual, executed outcome," exactly the input
// this gate needs), this predicts the source's outcome set *without*
// running it, then checks the witness's own already-verified `expect`
// against that prediction. A prediction that ever fails to contain a real,
// executed outcome is a predictor that lies — the one failure this gate
// exists to catch (pitfall A: over-approximation is allowed, omission is
// not).
//
// This does not re-execute anything itself (spec/outcome-witnesses.json's
// own `expect` field, checked by scripts/check-outcome-bijection.mjs, is
// already that proof) — it only calls the predictor and checks containment.
//
// Usage:
//   node scripts/check-outcome-prediction.mjs
//   AJISAI_BIN=/path/to/ajisai ...   # override CLI binary

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const read = (path) => readFileSync(resolve(repoRoot, path), 'utf8');

const errors = [];
const fail = (message) => errors.push(message);

// Mirrors scripts/check-outcome-bijection.mjs's resolveAjisaiBin exactly.
function resolveAjisaiBin() {
  if (process.env.AJISAI_BIN) {
    if (!existsSync(process.env.AJISAI_BIN)) {
      console.error(`[outcome-prediction] AJISAI_BIN not found: ${process.env.AJISAI_BIN}`);
      process.exit(1);
    }
    return process.env.AJISAI_BIN;
  }
  const debugBin = resolve(repoRoot, 'rust/target/debug/ajisai');
  if (!existsSync(debugBin)) {
    console.error('[outcome-prediction] building ajisai CLI (cargo build --bin ajisai)...');
    execFileSync('cargo', ['build', '--bin', 'ajisai'], {
      cwd: resolve(repoRoot, 'rust'),
      stdio: ['ignore', 'inherit', 'inherit'],
    });
  }
  if (!existsSync(debugBin)) {
    console.error('[outcome-prediction] ajisai CLI binary not found after build');
    process.exit(1);
  }
  return debugBin;
}

function predict(ajisaiBin, scratchDir, counter, source) {
  const file = join(scratchDir, `prediction-${counter}.ajisai`);
  writeFileSync(file, `${source}\n`);
  // `agent outcomes` always exits 0 (predicting always succeeds, even for a
  // program that cannot itself run) — execFileSync is safe here, unlike the
  // bijection gate's `run`.
  const stdout = execFileSync(ajisaiBin, ['agent', 'outcomes', file, '--json'], { encoding: 'utf8' });
  return JSON.parse(stdout);
}

// Classify a real run's JSON the way scripts/check-outcome-bijection.mjs
// does — the same duplicated-on-purpose copy, for the same reason its own
// comment gives (importing the table generator would rebuild the table).
function classifyOutcome(json) {
  if (json.status === 'error') {
    const kind = json.aiDiagnostic?.kind ?? json.diagnosis?.why;
    if (typeof kind !== 'string' || kind === '') {
      throw new Error(`error report names no category: ${JSON.stringify(json)}`);
    }
    return `error:${kind}`;
  }
  const stack = Array.isArray(json.stack) ? json.stack : [];
  const top = stack.length > 0 ? stack[stack.length - 1] : null;
  if (top && top.type === 'nil') {
    return `nil:${top.semantics?.absence?.reason}`;
  }
  return 'value';
}

function run(ajisaiBin, scratchDir, counter, source) {
  const file = join(scratchDir, `run-${counter}.ajisai`);
  writeFileSync(file, `${source}\n`);
  // A language ERROR exits 1 with the JSON diagnosis on stdout, so this
  // cannot use execFileSync (which would throw on it).
  const result = spawnSync(ajisaiBin, ['run', file, '--json'], { encoding: 'utf8' });
  if (result.error) throw result.error;
  if (result.status !== 0 && result.status !== 1) {
    throw new Error(`exit ${result.status}: ${result.stderr}`);
  }
  return classifyOutcome(JSON.parse(result.stdout));
}

// Programs whose *shape*, not whose outcome id, is the point: each one broke
// the predictor's soundness in a way the witness list could not see, because
// every witness is a short, direct program that fails at its first Word.
// These are run for real and checked the same way, so the containment
// property is tested against the compositions that actually threaten it.
// Added after an Opus review pass found the first two classes below live on
// `main`; keep adding here rather than to spec/outcome-witnesses.json, whose
// job is one witness per registry id (with a rationale about the exhaustive
// table), not adversarial composition.
const ADVERSARIAL = [
  // A name nothing defines only decides the outcome if execution reaches it.
  // Both of these answer something else entirely, and prediction used to
  // claim `error:unknownWord` *exactly*.
  'ADD FROBNICATE',
  "'a' 1 ADD FROBNICATE",
  '1 0 DIV FROBNICATE',
  'FROBNICATE',
  // A block can be pushed by one Word and executed by another, so a literal
  // that is inert *where it is written* still runs later. Prediction used to
  // skip anything a data literal contained.
  "[ [ 'a' ADD ] ] 'G' DEF 1 G EXEC",
  "[ [ 1 0 DIV ] ] 'G' DEF G EXEC",
  '[ 1 ADD ] EXEC',
  '[ 1 2 ADD ] 1 GET EXEC',
  // Ordinary compositions, as controls: a fix that widened everything to the
  // whole universe would still pass the two classes above, and should not.
  '1 2 ADD',
  '[ 1 2 3 ] [ 2 MUL ] MAP',
  "[ 1 ADD ] 'INC' DEF 5 INC",
  '1 0 DIV OR-NIL 9',
];

const witnessDoc = JSON.parse(read('spec/outcome-witnesses.json'));
const witnesses = Array.isArray(witnessDoc.witnesses) ? witnessDoc.witnesses : [];
if (witnesses.length === 0) {
  fail('spec/outcome-witnesses.json declares zero witnesses');
}

const ajisaiBin = resolveAjisaiBin();
const scratchDir = mkdtempSync(join(tmpdir(), 'ajisai-outcome-prediction-'));

let checked = 0;
try {
  witnesses.forEach((w, i) => {
    let prediction;
    try {
      prediction = predict(ajisaiBin, scratchDir, i, w.source);
    } catch (e) {
      fail(`witness "${w.id}": prediction failed to run: ${e.message}`);
      return;
    }
    const outcomes = Array.isArray(prediction.outcomes) ? prediction.outcomes : [];
    if (!outcomes.includes(w.expect)) {
      fail(
        `witness "${w.id}": actually observed ${JSON.stringify(w.expect)} (source: ${JSON.stringify(w.source)}), ` +
          `but the static predictor's set did not include it: ${JSON.stringify(outcomes)} — the predictor ` +
          `under-approximates, which pitfall A forbids`,
      );
      return;
    }
    checked += 1;
  });

  ADVERSARIAL.forEach((source, i) => {
    let prediction;
    let observed;
    try {
      prediction = predict(ajisaiBin, scratchDir, witnesses.length + i, source);
      observed = run(ajisaiBin, scratchDir, witnesses.length + i, source);
    } catch (e) {
      fail(`adversarial case ${JSON.stringify(source)}: failed to run: ${e.message}`);
      return;
    }
    const outcomes = Array.isArray(prediction.outcomes) ? prediction.outcomes : [];
    if (!outcomes.includes(observed)) {
      fail(
        `adversarial case ${JSON.stringify(source)}: running it observed ${JSON.stringify(observed)}, ` +
          `but the static predictor's set did not include it: ${JSON.stringify(outcomes)} — the predictor ` +
          `under-approximates, which pitfall A forbids`,
      );
      return;
    }
    // `exact` claims the set narrowed to the one outcome a deterministic,
    // total program really produces. If it says so and is wrong, the tool is
    // lying under its strongest label — check it separately from containment.
    if (prediction.exact === true && (outcomes.length !== 1 || outcomes[0] !== observed)) {
      fail(
        `adversarial case ${JSON.stringify(source)}: claimed exact but ${JSON.stringify(outcomes)} ` +
          `is not exactly the observed ${JSON.stringify(observed)}`,
      );
      return;
    }
    checked += 1;
  });
} finally {
  rmSync(scratchDir, { recursive: true, force: true });
}

if (errors.length > 0) {
  for (const e of errors) console.error(`[outcome-prediction] ${e}`);
  process.exit(1);
}
console.log(
  `[outcome-prediction] the static predictor's set contains the actually-observed outcome for all ${checked} cases ` +
    `(${witnesses.length} registry witnesses + ${ADVERSARIAL.length} adversarial compositions, the latter run for real).`,
);
