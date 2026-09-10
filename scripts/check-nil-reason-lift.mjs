#!/usr/bin/env node
// Reason-preservation gate for the element-wise lift (LANG.COLLECTIONS.LIFT:
// "each lane preserves the exactness, truth, NIL, and ERROR distinctions of
// the scalar law").
//
// The scalar law is passthrough with the reason intact — `1 0 / 1 +` is still
// `NIL(divisionByZero)`. This checks the lifted law says the same thing, lane
// for lane: a program that puts a reasoned NIL into a collection and then runs
// element-wise Words over it must still report that reason, and must still
// report an absence at all.
//
// Both halves are needed, and neither sees the other's failure:
//
//   - The *reason* check catches a lane whose absence survived but whose
//     reason did not. Every lane law that takes `Fraction` operands loses it:
//     a `Fraction` records absence as a zero denominator and carries nothing
//     about why, so the lane comes back reasonless — which reads as
//     `nil:literal`, "a NIL the program wrote rather than computed"
//     (spec/outcomes.json), for a NIL the program computed and never wrote.
//   - The *count* check catches a lane that stopped being an absence. The
//     exact-real lift read a NIL lane as `ExactReal::from_fraction(Fraction::
//     nil())` — a number whose denominator happens to be zero — computed with
//     it, and answered an observable `0/0` scalar. There is no reason to
//     compare there because there is no NIL left to carry one, so the reason
//     check passes such a lane in silence.
//
// Every program here is executed; nothing is string-matched. A program that
// raises is not a failure of this gate (an ERROR is a legitimate answer under
// LANG.FAILURE.TRICHOTOMY) — but a program that does not answer at all is.
//
// Usage:
//   node scripts/check-nil-reason-lift.mjs
//   AJISAI_BIN=/path/to/ajisai ...   # override CLI binary

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');

const errors = [];
const fail = (message) => errors.push(message);

// Mirrors scripts/check-outcome-prediction.mjs's resolveAjisaiBin exactly.
function resolveAjisaiBin() {
  if (process.env.AJISAI_BIN) {
    if (!existsSync(process.env.AJISAI_BIN)) {
      console.error(`[nil-reason-lift] AJISAI_BIN not found: ${process.env.AJISAI_BIN}`);
      process.exit(1);
    }
    return process.env.AJISAI_BIN;
  }
  const debugBin = resolve(repoRoot, 'rust/target/debug/ajisai');
  if (!existsSync(debugBin)) {
    console.error('[nil-reason-lift] building ajisai CLI (cargo build --bin ajisai)...');
    execFileSync('cargo', ['build', '--bin', 'ajisai'], {
      cwd: resolve(repoRoot, 'rust'),
      stdio: ['ignore', 'inherit', 'inherit'],
    });
  }
  if (!existsSync(debugBin)) {
    console.error('[nil-reason-lift] ajisai CLI binary not found after build');
    process.exit(1);
  }
  return debugBin;
}

// A program that leaves a collection with at least one reasoned NIL lane, and
// the reason every one of those lanes carries. Chosen to cover each way a lane
// can become absent and each representation a lane can live in: a dense
// rational vector, a nested one, a ragged one, and a vector of irrational
// exact reals (which takes an entirely separate lift).
//
// The written NIL (`[ 1 NIL 3 ]`) is not a spare case. A vector holding one
// used to be barred from dense storage, because dense storage could record
// only *that* a lane was absent; now that it records why, that vector is
// stored densely like any other and takes the same path as a computed one.
const PRODUCERS = [
  ['[ 1 2 ] [ 1 0 ] /', 'divisionByZero'],
  ['[ 6 6 6 ] [ 1 2 0 ] /', 'divisionByZero'],
  ['[ 6 ] [ 1 2 0 ] /', 'divisionByZero'],
  ['[ 1 2 ] [ 1 0 ] %', 'divisionByZero'],
  ['[ 4 -1 ] SQRT', 'domainMiss'],
  ['[ -1 -4 ] SQRT', 'domainMiss'],
  ["[ '1' 'a' ] [ NUM ] MAP", 'invalidEncoding'],
  ['[ 1 2 3 ] [ 0 / ] MAP', 'divisionByZero'],
  ['[ 2 3 ] [ SQRT ] MAP [ 1 0 ] /', 'divisionByZero'],
  ['[ [ 1 2 ] [ 3 4 ] ] [ [ 1 0 ] [ 1 1 ] ] /', 'divisionByZero'],
  ['[ 1 [ 2 3 ] ] 0 /', 'divisionByZero'],
  ['[ 1 NIL 3 ] [ 2 ] *', 'literal'],
];

// Applied after a producer. Each is lane-preserving: it maps over the lanes
// without adding or removing an absence, so the result must carry exactly the
// producer's absences, with exactly the producer's reasons.
const CHAINS = [
  '',
  '[ 1 1 ] +',
  '[ 1 1 ] -',
  '[ 1 1 ] *',
  '[ 1 1 ] /',
  '[ 1 1 ] %',
  '[ 2 ] *',
  '[ 2 ] +',
  '2 *',
  '2 +',
  'NEG',
  'ABS',
  'REVERSE',
  '[ 1 1 ] + [ 1 1 ] *',
  '[ 1 + ] MAP',
  // Structural rebuilds. These add no absence and remove none, but they
  // reassemble the collection from its children — which is where dense
  // storage is chosen. A lane whose reason lives only outside the dense
  // columns is lost exactly here, and nowhere the arithmetic chains above
  // would show it.
  '[ 3 4 ] CONCAT',
  '[ 3 4 ] CONCAT REVERSE',
  '[ 1 1 ] + [ 3 4 ] CONCAT',
];

function collectAbsenceReasons(node, out) {
  if (node === null || typeof node !== 'object') return;
  if (Array.isArray(node)) {
    for (const item of node) collectAbsenceReasons(item, out);
    return;
  }
  if (node.type === 'nil') out.push(node.semantics?.absence?.reason ?? '(no reason)');
  if (node.value !== undefined) collectAbsenceReasons(node.value, out);
}

const ajisaiBin = resolveAjisaiBin();
const scratchDir = mkdtempSync(join(tmpdir(), 'ajisai-nil-reason-lift-'));
let counter = 0;

// Run `source` and return its absence reasons, or a string describing why it
// has none to compare. A raise is one such reason; failing to answer is not.
function absenceReasons(source) {
  const file = join(scratchDir, `lift-${counter++}.ajisai`);
  writeFileSync(file, `${source}\n`);
  const result = spawnSync(ajisaiBin, ['run', file, '--json'], { encoding: 'utf8' });
  if (result.error) throw result.error;
  if (result.status !== 0 && result.status !== 1) {
    throw new Error(
      `the engine did not answer (exit ${result.status}) — no value, no NIL and no ERROR is ` +
        `no outcome under LANG.FAILURE.TRICHOTOMY: ${result.stderr.split('\n').slice(0, 3).join(' ')}`,
    );
  }
  const json = JSON.parse(result.stdout);
  if (json.status === 'error') return { raised: true, reasons: [] };
  const reasons = [];
  collectAbsenceReasons(json.stack ?? [], reasons);
  return { raised: false, reasons };
}

let checked = 0;
try {
  for (const [producer, expected] of PRODUCERS) {
    let base;
    try {
      base = absenceReasons(producer);
    } catch (e) {
      fail(`producer ${JSON.stringify(producer)}: ${e.message}`);
      continue;
    }
    if (base.raised || base.reasons.length === 0) {
      fail(
        `producer ${JSON.stringify(producer)} is supposed to leave at least one reasoned NIL lane ` +
          `and left none — the case no longer tests what it was written for`,
      );
      continue;
    }

    for (const chain of CHAINS) {
      const source = chain ? `${producer} ${chain}` : producer;
      let observed;
      try {
        observed = absenceReasons(source);
      } catch (e) {
        fail(`${JSON.stringify(source)}: ${e.message}`);
        continue;
      }
      if (observed.raised) continue;

      if (observed.reasons.length !== base.reasons.length) {
        fail(
          `${JSON.stringify(source)}: ${JSON.stringify(producer)} leaves ` +
            `${base.reasons.length} absent lane(s) and this leaves ${observed.reasons.length} — ` +
            `an element-wise Word neither creates nor fills a lane, so an absence stopped being one`,
        );
        continue;
      }
      const wrong = observed.reasons.filter((reason) => reason !== expected);
      if (wrong.length > 0) {
        fail(
          `${JSON.stringify(source)}: every absent lane was created with reason ` +
            `${JSON.stringify(expected)}, but the result reports ${JSON.stringify(observed.reasons)} — ` +
            `the lift dropped the reason the scalar law preserves`,
        );
        continue;
      }
      checked += 1;
    }
  }
} finally {
  rmSync(scratchDir, { recursive: true, force: true });
}

if (errors.length > 0) {
  for (const e of errors) console.error(`[nil-reason-lift] ${e}`);
  process.exit(1);
}
console.log(
  `[nil-reason-lift] every absent lane kept its reason and stayed absent across ${checked} executed ` +
    `programs (${PRODUCERS.length} producers x ${CHAINS.length} element-wise chains).`,
);
