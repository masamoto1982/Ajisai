#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "./index.js";
import { indexTraces, mayAssertPerfect, traceProvenance, validateCorpus } from "./evaluation-contract.js";

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

let diagnosesObserved = 0;
let repairsSucceeded = 0;
let missing = 0;
try {
  for (const testCase of corpus.cases) {
    const trace = traces.get(testCase.id);
    if (!trace) {
      missing += 1;
      console.log(`MISS ${testCase.id}`);
      continue;
    }
    const first = await callAttempt(client, trace.firstAttempt);
    const diagnosisOk = matches(first, testCase.firstAttempt.expect);
    if (diagnosisOk) diagnosesObserved += 1;
    // A lucky standalone answer is not diagnostic recovery.
    const repaired = diagnosisOk ? await callAttempt(client, trace.repairedAttempt) : null;
    const repairOk = diagnosisOk && matches(repaired, testCase.repairedAttempt.expect);
    if (repairOk) repairsSucceeded += 1;
    console.log(`${repairOk ? "PASS" : "FAIL"} ${testCase.id} diagnosis=${diagnosisOk}`);
  }
} finally {
  await client.close();
  await server.close();
}

const total = corpus.cases.length;
const metrics = {
  schemaVersion: 1,
  // Travels with the numbers: a repair rate copied out of a log still says
  // whether it describes a model or the scorer that grades one.
  provenance,
  measures: provenance.source === "referenceFixture"
    ? "scorer conformance only — not model performance"
    : `model performance for ${provenance.modelId}`,
  cases: total,
  missingTraces: missing,
  diagnosisObservedRate: diagnosesObserved / total,
  diagnosisDrivenRepairRate: repairsSucceeded / total,
};
console.log(JSON.stringify(metrics, null, 2));
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
