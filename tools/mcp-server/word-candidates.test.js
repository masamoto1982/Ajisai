#!/usr/bin/env node
// Cross-checks the JS "did you mean" suggester against the real engine.
//
// word-candidates.js is a deliberate hand-copy of
// rust/src/interpreter/word_candidates.rs (same distance ceiling, same cap,
// same tie-break) because the registry-lookup tool needs an answer without
// spawning the native backend for every call. A hand-copy is exactly the kind
// of thing that can drift silently — as the ranking rule changes on one side,
// or the compiled-in vocabulary the two sides draw from stops matching (this
// repo's Corewords, packaged separately as
// tools/mcp-server/assets/words.json vs. compiled into the `ajisai` binary).
// This test closes that gap the same way backend/parity-test.js closes the
// native/WASM one: run the same inputs through both implementations and
// assert they agree, rather than trusting the doc comment's claim.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createBackend } from "./index.js";
import { suggestWords } from "./word-candidates.js";

// The engine is whichever backend the server itself would answer with — the
// packaged WASM worker on a fresh clone, a native binary when one is built.
// Requiring the native one made `npm run selftest` fail on the very checkout
// the README promises needs no `cargo`; parity-test.js is what shows the two
// backends give one answer, so asking either is asking the engine.
const engine = createBackend();
assert.ok(engine, "no execution backend available to compare word candidates against");

async function engineCandidates(word) {
  // An unknown-word program is a language ERROR — a successful call carrying
  // the diagnosis whose `candidates` the lookup tool must match.
  const result = await engine.compute(word);
  const candidates = result?.diagnosis?.candidates;
  if (!Array.isArray(candidates)) {
    throw new Error(`could not read diagnosis.candidates from: ${JSON.stringify(result)}`);
  }
  return candidates;
}

// The same asset file the registry-lookup tool reads in production
// (tools/mcp-server/index.js's `contracts()`), so this test exercises the
// exact vocabulary the tool actually suggests from.
const registry = JSON.parse(
  readFileSync(new URL("./assets/words.json", import.meta.url), "utf8"),
);

// A representative spread: a one-letter transposition/omission on a short,
// medium, and longer canonical name (each falls in a different distanceCeiling
// bucket); a name with several equally-close matches, to exercise the
// distance-then-alphabetical tie-break; and an unmatched name, which must
// come back empty on both sides.
const CASES = ["LENGHT", "MAPP", "FILTR", "PRIN", "ADDD", "SQR", "EXECC", "ZZZZZZZZZZ"];

for (const word of CASES) {
  const fromJs = suggestWords(word, registry.entries);
  const fromEngine = await engineCandidates(word);
  assert.deepEqual(
    fromJs,
    fromEngine,
    `suggestWords(${JSON.stringify(word)}) = ${JSON.stringify(fromJs)}, ` +
      `but the engine's diagnosis.candidates = ${JSON.stringify(fromEngine)}`,
  );
}

console.log(`word-candidates parity: ${CASES.length} cases agree with the engine (${engine.kind}).`);
process.exit(0);
