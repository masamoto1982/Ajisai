#!/usr/bin/env node
// Cross-checks the JS "did you mean" suggester against the real engine.
//
// word-candidates.js is a deliberate hand-copy of
// rust/src/interpreter/word_candidates.rs (same distance ceiling, same cap,
// same tie-break) because the registry-lookup tool needs an answer without
// spawning the native backend for every call. A hand-copy is exactly the kind
// of thing that can drift silently — as the ranking rule changes on one side,
// or the compiled-in vocabulary the two sides draw from stops matching (this
// repo's Corewords + aliases, packaged separately as
// tools/mcp-server/assets/words.json vs. compiled into the `ajisai` binary).
// This test closes that gap the same way backend/parity-test.js closes the
// native/WASM one: run the same inputs through both implementations and
// assert they agree, rather than trusting the doc comment's claim.

import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { suggestWords } from "./word-candidates.js";

const here = dirname(fileURLToPath(import.meta.url));

function resolveNativeBin() {
  if (process.env.AJISAI_BIN) return process.env.AJISAI_BIN;
  const repoRoot = process.env.AJISAI_REPO
    ? resolve(process.env.AJISAI_REPO)
    : resolve(here, "..", "..");
  for (const profile of ["debug", "release"]) {
    const candidate = join(repoRoot, "rust", "target", profile, "ajisai");
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

const bin = resolveNativeBin();
if (!bin) {
  console.error(
    "word-candidates parity test requires a built native `ajisai` binary; run " +
      "`cargo build --manifest-path rust/Cargo.toml --bin ajisai` first, or set AJISAI_BIN.",
  );
  process.exit(1);
}

function nativeCandidates(word) {
  return new Promise((resolvePromise, reject) => {
    const child = execFile(
      bin,
      ["agent", "compute", "-", "--json"],
      { encoding: "utf8" },
      (error, stdout) => {
        // The engine reports an unknown-word program as a language ERROR,
        // which exits 1 with the diagnosis JSON on stdout — the same
        // "successful call, language-level error" shape backend/native-cli.js
        // relies on.
        if (error && !(error.code === 1 && stdout)) return reject(error);
        try {
          resolvePromise(JSON.parse(stdout).diagnosis.candidates);
        } catch (parseError) {
          reject(new Error(`could not read diagnosis.candidates from: ${stdout}\n${parseError}`));
        }
      },
    );
    child.stdin.end(word, "utf8");
  });
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
  const fromEngine = await nativeCandidates(word);
  assert.deepEqual(
    fromJs,
    fromEngine,
    `suggestWords(${JSON.stringify(word)}) = ${JSON.stringify(fromJs)}, ` +
      `but the engine's diagnosis.candidates = ${JSON.stringify(fromEngine)}`,
  );
}

console.log(`word-candidates parity: ${CASES.length} cases agree with the engine.`);
