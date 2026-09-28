#!/usr/bin/env node
// The four source tools answer one question each about the same program, and
// their answers have to fit together. This runs every source the golden and
// evaluation corpora hold, plus the edge cases that once split them, through
// `compute`, `check`, `infer_contracts` and `outcomes`, and holds them to the
// relations they promise:
//
//   * a run's `outcome` is always one `outcomes` predicted;
//   * malformed source is malformed to all four, and `outcomes` says so exactly;
//   * `check` ok means the run does not fail on the program's form or its names;
//   * a name `check` cannot resolve is an `unknownWord` `outcomes` allows for;
//   * inference succeeds exactly when the source reads, and a contract whose
//     body names an unresolved Word is never `total`.
//
// These split before: `[ 1 2` was `malformedSource` to three tools and an `ok`
// with no contracts to inference, and `[ FOO ] 'W' DEF` inferred a `total` Word
// whose first call fails.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createBackend, outcomeOf } from "./index.js";

const read = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));

const EDGE_CASES = [
  "",
  "   ",
  "[ 1 2",
  "1 2 ]",
  "[ 1 [ 2 ]",
  "'unterminated",
  "FOO",
  "1 ADD",
  "1 0 DIV",
  "NIL",
  "[ FOO ] 'W' DEF",
  "[ FOO ] 'W' DEF W",
  "[ 1 ADD ] 'W' DEF 2 W",
  "[ BAR ] 'W' DEF [ 1 ] 'BAR' DEF W",
  "[ 1 2 3 ] [ 2 MUL ] MAP",
  "[ [ 1 ] ADD ] 'INC' DEF [ INC ] 'TWICE' DEF 3 TWICE",
  "'W' DEL",
  "[ 1 ] 'ADD' DEF",
];

const sources = [
  ...new Set([
    ...read("./golden/cases.json").cases.map(({ source }) => source),
    ...read("./eval/cases.json")
      .cases.map(({ arguments: args }) => args?.source)
      .filter((source) => typeof source === "string"),
    ...EDGE_CASES,
  ]),
];

const backend = createBackend();
assert.ok(backend, "no execution backend available for the consistency test");

const failures = [];
const expect = (condition, source, message) => {
  if (!condition) failures.push(`${JSON.stringify(source)}: ${message}`);
};

let compared = 0;
for (const source of sources) {
  let run;
  try {
    run = await backend.compute(source);
  } catch (error) {
    // A host refusal (a ceiling the adapter holds) answers no language
    // question, so there is no outcome to compare.
    if (error?.code) continue;
    throw error;
  }
  const [checked, inferred, predicted] = await Promise.all([
    backend.check(source),
    backend.inferContracts(source),
    backend.outcomes(source),
  ]);
  compared += 1;
  const outcome = outcomeOf(run);
  const predictedSet = predicted.outcomes ?? [];

  expect(outcome && predictedSet.includes(outcome), source,
    `compute's outcome ${outcome} is not in the predicted set ${JSON.stringify(predictedSet)}`);

  const malformed = outcome === "error:malformedSource";
  expect((checked.aiDiagnostic?.category === "malformedSource") === malformed, source,
    `check says ${checked.status}/${checked.aiDiagnostic?.category}, compute says ${outcome}`);
  expect((inferred.aiDiagnostic?.category === "malformedSource") === malformed, source,
    `infer_contracts says ${inferred.status}/${inferred.aiDiagnostic?.category}, compute says ${outcome}`);
  expect((inferred.status === "ok") === !malformed, source,
    `infer_contracts is ${inferred.status} for a source compute reads as ${outcome}`);
  if (malformed) {
    expect(predicted.exact === true && predictedSet.length === 1, source,
      `outcomes is not exactly [error:malformedSource]: ${JSON.stringify(predictedSet)}`);
  }

  if (checked.status === "ok") {
    expect(!["error:malformedSource", "error:unknownWord"].includes(outcome), source,
      `check ok, but the run failed with ${outcome}`);
  }
  if (checked.aiDiagnostic?.category === "unknownWord") {
    expect(predictedSet.includes("error:unknownWord"), source,
      "check names an unknown Word that outcomes does not allow for");
  }

  for (const contract of inferred.contracts ?? []) {
    if ((contract.gaps ?? []).includes("gap.unresolvedWord")) {
      expect(contract.partiality !== "total", source,
        `${contract.name} calls an unresolved Word but is inferred total`);
    }
  }
}

assert.deepEqual(failures, [], `the four tools disagree:\n  ${failures.join("\n  ")}`);
assert.ok(compared >= 50, `expected to compare at least 50 sources, compared ${compared}`);
console.log(`tool consistency: ${compared} sources agree across compute, check, infer_contracts and outcomes (${backend.kind}).`);
process.exit(0);
