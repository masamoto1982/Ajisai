#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "./index.js";
import {
  LANGUAGES,
  indexTraces,
  mayAssertPerfect,
  traceKey,
  traceProvenance,
  validateCorpus,
} from "./evaluation-contract.js";

function atPointer(document, pointer) {
  return pointer.split("/").slice(1)
    .map((part) => part.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((value, part) => value?.[part], document);
}

const tracePath = process.argv[2];
if (!tracePath) {
  console.error("usage: node score-traces.js <trace-file.json> [--require-perfect]");
  process.exit(2);
}
const corpus = JSON.parse(readFileSync(new URL("./eval/cases.json", import.meta.url), "utf8"));
const traceDoc = JSON.parse(readFileSync(tracePath, "utf8"));
const traces = indexTraces(traceDoc, validateCorpus(corpus));
const provenance = traceProvenance(traceDoc);
const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
const server = createServer();
const client = new Client({ name: "ajisai-trace-eval", version: "1" });
await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);

/** One language's tally. Kept per language so the pair can be compared. */
function emptyTally() {
  return { asked: 0, missing: 0, selected: 0, generated: 0, semantic: 0, activations: 0 };
}

const tallies = new Map(LANGUAGES.map((language) => [language, emptyTally()]));

for (const testCase of corpus.cases) {
  for (const language of LANGUAGES) {
    const tally = tallies.get(language);
    tally.asked += 1;
    const label = `${testCase.id} [${language}]`;
    const trace = traces.get(traceKey(testCase.id, language));
    if (!trace) {
      tally.missing += 1;
      console.log(`MISS ${label}`);
      continue;
    }
    const selectionOk = trace.selectedTool === testCase.expectedTool;
    if (selectionOk) tally.selected += 1;
    if (testCase.expectedTool === null) {
      if (trace.selectedTool !== null) tally.activations += 1;
      if (selectionOk) tally.semantic += 1;
      console.log(`${selectionOk ? "PASS" : "FAIL"} ${label} restraint`);
      continue;
    }
    if (trace.selectedTool === null) {
      console.log(`FAIL ${label} no tool selected`);
      continue;
    }
    let result;
    try {
      result = await client.callTool({
        name: trace.selectedTool,
        arguments: trace.arguments ?? {},
      });
    } catch {
      result = null;
    }
    // Two different failures, separated because they have different repairs.
    // `generated` asks only whether the arguments the model wrote produce the
    // expected result — running them through the tool it actually chose. A
    // model that reaches for `check` and writes correct source has a tool
    // problem; one that reaches for `compute` and writes source that computes
    // the wrong thing has a language problem. Folding both into one rate, as
    // this scorer used to, reported the sum and named neither.
    const producedExpected = result?.isError !== true && result !== null &&
      Object.entries(testCase.expect).every(([pointer, expected]) =>
        JSON.stringify(atPointer(result.structuredContent, pointer)) === JSON.stringify(expected));
    if (producedExpected) tally.generated += 1;
    const semanticsOk = selectionOk && producedExpected;
    if (semanticsOk) tally.semantic += 1;
    console.log(
      `${semanticsOk ? "PASS" : "FAIL"} ${label} trace` +
        (semanticsOk ? "" : ` (selection=${selectionOk} generation=${producedExpected})`),
    );
  }
}

await client.close();
await server.close();

const positives = corpus.cases.filter((testCase) => testCase.expectedTool !== null).length;
const negatives = corpus.cases.length - positives;

function rates(tally) {
  return {
    asked: tally.asked,
    missingTraces: tally.missing,
    toolSelectionAccuracy: tally.selected / tally.asked,
    // Over the positive cases only: a case whose correct answer is *no* call
    // has nothing to generate, and counting it as a generation success would
    // reward restraint twice while diluting the rate this metric is for.
    firstAttemptGenerationRate: positives === 0 ? 0 : tally.generated / positives,
    semanticSuccessRate: tally.semantic / tally.asked,
    irrelevantToolRate: negatives === 0 ? 0 : tally.activations / negatives,
  };
}

const combined = emptyTally();
for (const tally of tallies.values()) {
  for (const key of Object.keys(combined)) combined[key] += tally[key];
}
const overall = {
  asked: combined.asked,
  missingTraces: combined.missing,
  toolSelectionAccuracy: combined.selected / combined.asked,
  firstAttemptGenerationRate: positives === 0
    ? 0
    : combined.generated / (positives * LANGUAGES.length),
  semanticSuccessRate: combined.semantic / combined.asked,
  irrelevantToolRate: negatives === 0
    ? 0
    : combined.activations / (negatives * LANGUAGES.length),
};

const byLanguage = Object.fromEntries(
  [...tallies].map(([language, tally]) => [language, rates(tally)]),
);
const metrics = {
  schemaVersion: 1,
  // What produced these traces travels with the numbers, so a score copied out
  // of a log still says whether it describes a model or the scorer.
  provenance,
  measures: provenance.source === "referenceFixture"
    ? "scorer conformance only — not model performance"
    : `model performance for ${provenance.modelId}`,
  cases: corpus.cases.length,
  languages: LANGUAGES,
  ...overall,
  byLanguage,
  // The number the pairing exists to produce. Both halves of a pair name the
  // same task and expect the same answer, so a non-zero gap is the cost of
  // asking in Japanese — of an English tool surface, not of the engine.
  languageGap: {
    toolSelectionAccuracy:
      byLanguage.en.toolSelectionAccuracy - byLanguage.ja.toolSelectionAccuracy,
    firstAttemptGenerationRate:
      byLanguage.en.firstAttemptGenerationRate - byLanguage.ja.firstAttemptGenerationRate,
    semanticSuccessRate: byLanguage.en.semanticSuccessRate - byLanguage.ja.semanticSuccessRate,
  },
};
console.log(JSON.stringify(metrics, null, 2));
const missing = combined.missing;
const semanticSuccess = combined.semantic;
const irrelevantActivations = combined.activations;
const total = combined.asked;
if (process.argv.includes("--require-perfect")) {
  // Asserting perfection is a statement about the *harness*. Allowing it on a
  // model trace would turn the first clean run into a committed claim that the
  // model is perfect — the exact overstatement this corpus exists to avoid.
  if (!mayAssertPerfect(traceDoc)) {
    console.error(
      "--require-perfect asserts scorer conformance and is only valid for a referenceFixture trace; " +
        "a model trace is reported, never asserted.",
    );
    process.exit(2);
  }
  if (missing !== 0 || semanticSuccess !== total || irrelevantActivations !== 0) {
    process.exit(1);
  }
}
