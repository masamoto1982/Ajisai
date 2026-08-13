#!/usr/bin/env node
// Regenerate the selection reference fixture from the corpus it grades.
//
// The fixture is the corpus answering itself: every case, asked once per
// language, answered with the case's own reference arguments. That is what a
// perfect trace *is*, so writing it by hand was busywork that drifted — a case
// added to `cases.json` without a matching pair here scores as two missing
// traces, and `--require-perfect` then fails for a reason that has nothing to
// do with the scorer it is asserting.
//
// Generated rather than removed because the fixture still has a job: it proves
// `score-traces.js` runs end to end, including the per-language split, on a
// document built to pass it. What it must never be mistaken for is a model
// result, which is what `provenance.source` is for.
//
//   node generate-reference-traces.mjs           # rewrite
//   node generate-reference-traces.mjs --check   # fail on drift (CI)

import { readFileSync, writeFileSync } from "node:fs";
import { LANGUAGES, validateCorpus } from "./evaluation-contract.js";

const corpusUrl = new URL("./eval/cases.json", import.meta.url);
const traceUrl = new URL("./eval/reference-traces.json", import.meta.url);

const NOTE =
  "Generated tool-selection trace built to pass score-traces.js: every case answered " +
  "with its own reference arguments, in both languages. It proves the scorer runs end " +
  "to end; it is not a model result and its perfect score describes nothing about any " +
  "model. Regenerate with `npm run eval:reference-traces` in tools/mcp-server.";

const corpus = JSON.parse(readFileSync(corpusUrl, "utf8"));
validateCorpus(corpus);

const traces = corpus.cases.flatMap((testCase) =>
  LANGUAGES.map((language) => ({
    caseId: testCase.id,
    language,
    selectedTool: testCase.expectedTool,
    // A negative case selects no tool and carries no arguments; the scorer
    // never calls anything for it, and `{}` keeps the trace shape uniform.
    arguments: testCase.arguments ?? {},
  })),
);

const document = {
  schemaVersion: 1,
  provenance: { source: "referenceFixture", note: NOTE },
  traces,
};
const rendered = `${JSON.stringify(document, null, 1)}\n`;

if (process.argv.includes("--check")) {
  const current = readFileSync(traceUrl, "utf8");
  if (current !== rendered) {
    console.error(
      "reference-traces.json is out of date with cases.json; run " +
        "`npm run eval:reference-traces` in tools/mcp-server",
    );
    process.exit(1);
  }
  console.log(`reference selection trace is current (${traces.length} traces)`);
} else {
  writeFileSync(traceUrl, rendered);
  console.log(`wrote ${traces.length} reference traces`);
}
