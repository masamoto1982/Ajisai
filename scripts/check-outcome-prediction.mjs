#!/usr/bin/env node
// Predicted-vs-actual gate for `ajisai agent outcomes` (Phase 5,
// docs/dev/auditable-kernel-work-order-2026-09.md §5), pitfall D: "the
// predictor doesn't run the program, but verification does." Given a source
// and its actual, executed outcome, this predicts the source's outcome set
// *without* running it and checks that the prediction contains the real
// outcome. A prediction that ever fails to is a predictor that lies — the
// one failure this gate exists to catch (pitfall A: over-approximation is
// allowed, omission is not).
//
// It draws those (source, observed outcome) pairs from three places:
//
//   1. Every cell of docs/semantics-table.json — the exhaustive
//      (Word x domain-tuple) table, 6,593 already-executed programs.
//   2. Every witness in spec/outcome-witnesses.json — the registry ids the
//      exhaustive table cannot reach, one hand-written witness each.
//   3. Compositions — a hand-written adversarial list plus a generated
//      operand x operand x Word sweep, below — executed here.
//
// (1) is not redundant with (2): they are complements by construction, and
// that is exactly how this gate went blind once. A witness exists in
// spec/outcome-witnesses.json *because* no table cell produces its id, so a
// gate sampling only witnesses can never see an outcome the table does
// witness — however common. `nil:literal` is 676 of the table's 6,593 cells,
// the second most frequent outcome in the language, and the predictor omitted
// it entirely while this gate stayed green. Sampling the table closes that
// class of blind spot at its root rather than by adding one more case.
//
// Neither (1) nor (2) re-executes anything: both files carry an outcome that
// was produced by running the program (scripts/generate-semantics-table.mjs
// executes every cell; spec/outcome-witnesses.json's `expect` is checked by
// scripts/check-outcome-bijection.mjs). Only (3) runs here — it has no such
// file behind it, and running is also what lets it catch an engine that
// answers with no outcome at all (a panic), which no recorded table can show.
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
  // A String can name a Word, so a Word can run with no `Token::Symbol` for
  // it anywhere in the source. These two really raise `builtinProtection` and
  // `wordNotFound` through `'DEL'` written as a string, and a prediction that
  // reasons from Symbols alone omits both. Added when narrowing the
  // structural ceiling by reachability made that omission possible: the
  // twelve cases above did not exercise it, so the gate was green against a
  // predictor that had just gone unsound.
  "[ 'ADD' ] 'DEL' MAP",
  "[ 'NOPE' ] 'DEL' MAP",
  // Ordinary compositions, as controls: a fix that widened everything to the
  // whole universe would still pass the two classes above, and should not.
  '1 2 ADD',
  '[ 1 2 3 ] [ 2 MUL ] MAP',
  "[ 1 ADD ] 'INC' DEF 5 INC",
  '1 0 DIV OR-NIL 9',
  // Reason loss: a lane holds an absence but not the reason for it, so a
  // computed NIL that crosses one twice comes back reasonless and reads as
  // `nil:literal` — with no NIL written anywhere in the source. Prediction
  // must admit that (word_outcome_vocabulary::close_over_nil_reason_loss).
  '[ 1 2 ] [ 1 0 ] DIV [ 1 1 ] DIV [ 1 ] GET',
  'NIL 1 ADD',
];

// Operand shapes and the Words applied to them, crossed exhaustively below.
//
// Unlike the table pass, this one *executes* every program, so it also
// catches an outcome that is not an outcome at all: a panic leaves no value,
// no NIL and no ERROR, so LANG.FAILURE.TRICHOTOMY does not classify it and
// every prediction for that program is vacuously wrong. That is what this
// cross product found on `main` — `[ ] 1 ADD` broadcast shapes `[0]` and `[]`
// to a one-lane result and then indexed lane 0 of a zero-lane tensor
// (tensor_ops::broadcast_shape). The empty vector, the ragged vector and the
// NIL-carrying vector are the shapes the exhaustive table's own domain list
// does not represent, which is why they are the point of this list.
const SWEEP_OPERANDS = [
  '1',
  '0',
  "'a'",
  'TRUE',
  'NIL',
  '[ ]',
  '[ 1 2 ]',
  '[ 1 ]',
  '[ NIL 1 ]',
  '[ 1 [ 2 3 ] ]',
];
const SWEEP_BINARY = ['ADD', 'DIV', 'MOD', 'EQ', 'AND', 'CONCAT', 'GET', 'MAP'];
const SWEEP_UNARY = ['NEG', 'SQRT', 'NOT', 'LENGTH', 'SORT', 'JOIN', 'NIL-REASON', 'EXEC'];

function sweepPrograms() {
  const programs = new Set();
  for (const a of SWEEP_OPERANDS) {
    for (const u of SWEEP_UNARY) programs.add(`${a} ${u}`);
    for (const b of SWEEP_OPERANDS) {
      for (const op of SWEEP_BINARY) programs.add(`${a} ${b} ${op}`);
    }
  }
  return [...programs];
}

const witnessDoc = JSON.parse(read('spec/outcome-witnesses.json'));
const witnesses = Array.isArray(witnessDoc.witnesses) ? witnessDoc.witnesses : [];
if (witnesses.length === 0) {
  fail('spec/outcome-witnesses.json declares zero witnesses');
}

// Every cell of the exhaustive table, as (source, already-observed outcome).
// The source is rebuilt from the table's own embedded `domains` by the rule
// scripts/generate-semantics-table.mjs used to build it — operands in tuple
// order, then the Word name — read from the JSON rather than imported from
// the generator, which would regenerate the table (the same reason
// classifyOutcome above is a deliberate copy).
function tableCases() {
  const table = JSON.parse(read('docs/semantics-table.json'));
  const domains = new Map((table.domains ?? []).map((d) => [d.id, d.source]));
  const cells = Array.isArray(table.cells) ? table.cells : [];
  if (cells.length === 0) {
    fail('docs/semantics-table.json declares zero cells');
  }
  return cells.map((cell) => {
    const operands = (cell.inputs ?? []).map((id) => {
      const source = domains.get(id);
      if (source === undefined) {
        throw new Error(`cell names domain "${id}", which docs/semantics-table.json does not define`);
      }
      return source;
    });
    return { source: [...operands, cell.word].join(' '), expect: cell.outcome };
  });
}

let tableCells;
try {
  tableCells = tableCases();
} catch (e) {
  fail(`docs/semantics-table.json could not be read as prediction cases: ${e.message}`);
  tableCells = [];
}

const ajisaiBin = resolveAjisaiBin();
const scratchDir = mkdtempSync(join(tmpdir(), 'ajisai-outcome-prediction-'));

let checked = 0;
let counter = 0;
try {
  tableCells.forEach((cell) => {
    let prediction;
    try {
      prediction = predict(ajisaiBin, scratchDir, counter++, cell.source);
    } catch (e) {
      fail(`table cell ${JSON.stringify(cell.source)}: prediction failed to run: ${e.message}`);
      return;
    }
    const outcomes = Array.isArray(prediction.outcomes) ? prediction.outcomes : [];
    if (!outcomes.includes(cell.expect)) {
      fail(
        `table cell ${JSON.stringify(cell.source)}: docs/semantics-table.json records the executed ` +
          `outcome ${JSON.stringify(cell.expect)}, but the static predictor's set did not include it: ` +
          `${JSON.stringify(outcomes)} — the predictor under-approximates, which pitfall A forbids`,
      );
      return;
    }
    checked += 1;
  });

  witnesses.forEach((w) => {
    let prediction;
    try {
      prediction = predict(ajisaiBin, scratchDir, counter++, w.source);
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

  [...ADVERSARIAL, ...sweepPrograms()].forEach((source) => {
    let prediction;
    let observed;
    const index = counter++;
    try {
      prediction = predict(ajisaiBin, scratchDir, index, source);
      observed = run(ajisaiBin, scratchDir, index, source);
    } catch (e) {
      fail(
        `executed case ${JSON.stringify(source)}: failed to run: ${e.message} — an engine that does not ` +
          `answer at all produces no outcome under LANG.FAILURE.TRICHOTOMY, so no prediction for it can be right`,
      );
      return;
    }
    const outcomes = Array.isArray(prediction.outcomes) ? prediction.outcomes : [];
    if (!outcomes.includes(observed)) {
      fail(
        `executed case ${JSON.stringify(source)}: running it observed ${JSON.stringify(observed)}, ` +
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
        `executed case ${JSON.stringify(source)}: claimed exact but ${JSON.stringify(outcomes)} ` +
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
    `(${tableCells.length} exhaustive-table cells + ${witnesses.length} registry witnesses + ` +
    `${ADVERSARIAL.length + sweepPrograms().length} compositions, the last group run for real).`,
);
