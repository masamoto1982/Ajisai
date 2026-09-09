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
} finally {
  rmSync(scratchDir, { recursive: true, force: true });
}

if (errors.length > 0) {
  for (const e of errors) console.error(`[outcome-prediction] ${e}`);
  process.exit(1);
}
console.log(
  `[outcome-prediction] the static predictor's set contains the actually-observed outcome for all ${checked} witnesses.`,
);
