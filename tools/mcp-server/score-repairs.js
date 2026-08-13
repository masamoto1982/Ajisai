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

function matches(result, expected) {
  return result?.isError !== true && result?.structuredContent &&
    Object.entries(expected).every(([pointer, value]) =>
      JSON.stringify(atPointer(result.structuredContent, pointer)) === JSON.stringify(value));
}

async function callAttempt(client, attempt) {
  if (attempt?.selectedTool !== "compute") return null;
  try {
    return await client.callTool({ name: attempt.selectedTool, arguments: attempt.arguments ?? {} });
  } catch {
    return null;
  }
}

const tracePath = process.argv[2];
if (!tracePath) {
  console.error("usage: node score-repairs.js <repair-trace-file.json> [--require-perfect]");
  process.exit(2);
}
const corpus = JSON.parse(readFileSync(new URL("./eval/repair-cases.json", import.meta.url), "utf8"));
const traceDoc = JSON.parse(readFileSync(tracePath, "utf8"));
const traces = indexTraces(traceDoc, validateCorpus(corpus, { repair: true }), { repair: true });
const provenance = traceProvenance(traceDoc);
const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
const server = createServer();
const client = new Client({ name: "ajisai-repair-eval", version: "1" });
await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);

const tallies = new Map(
  LANGUAGES.map((language) => [language, { asked: 0, missing: 0, observed: 0, repaired: 0 }]),
);
try {
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
      const first = await callAttempt(client, trace.firstAttempt);
      const diagnosisOk = matches(first, testCase.firstAttempt.expect);
      if (diagnosisOk) tally.observed += 1;
      // A lucky standalone answer is not diagnostic recovery.
      const repaired = diagnosisOk ? await callAttempt(client, trace.repairedAttempt) : null;
      const repairOk = diagnosisOk && matches(repaired, testCase.repairedAttempt.expect);
      if (repairOk) tally.repaired += 1;
      console.log(`${repairOk ? "PASS" : "FAIL"} ${label} diagnosis=${diagnosisOk}`);
    }
  }
} finally {
  await client.close();
  await server.close();
}

const combined = { asked: 0, missing: 0, observed: 0, repaired: 0 };
for (const tally of tallies.values()) {
  for (const key of Object.keys(combined)) combined[key] += tally[key];
}
const byLanguage = Object.fromEntries(
  [...tallies].map(([language, tally]) => [language, {
    asked: tally.asked,
    missingTraces: tally.missing,
    diagnosisObservedRate: tally.observed / tally.asked,
    diagnosisDrivenRepairRate: tally.repaired / tally.asked,
  }]),
);
const metrics = {
  schemaVersion: 1,
  // Travels with the numbers: a repair rate copied out of a log still says
  // whether it describes a model or the scorer that grades one.
  provenance,
  measures: provenance.source === "referenceFixture"
    ? "scorer conformance only — not model performance"
    : `model performance for ${provenance.modelId}`,
  cases: corpus.cases.length,
  languages: LANGUAGES,
  asked: combined.asked,
  missingTraces: combined.missing,
  diagnosisObservedRate: combined.observed / combined.asked,
  diagnosisDrivenRepairRate: combined.repaired / combined.asked,
  byLanguage,
  languageGap: {
    diagnosisDrivenRepairRate:
      byLanguage.en.diagnosisDrivenRepairRate - byLanguage.ja.diagnosisDrivenRepairRate,
  },
};
console.log(JSON.stringify(metrics, null, 2));
const total = combined.asked;
const missing = combined.missing;
const diagnosesObserved = combined.observed;
const repairsSucceeded = combined.repaired;
if (process.argv.includes("--require-perfect")) {
  // See score-traces.js: perfection is asserted of the harness, never of a model.
  if (!mayAssertPerfect(traceDoc)) {
    console.error(
      "--require-perfect asserts scorer conformance and is only valid for a referenceFixture trace; " +
        "a model trace is reported, never asserted.",
    );
    process.exit(2);
  }
  if (missing !== 0 || diagnosesObserved !== total || repairsSucceeded !== total) process.exit(1);
}
