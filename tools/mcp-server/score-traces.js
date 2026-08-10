#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "./index.js";
import { indexTraces, validateCorpus } from "./evaluation-contract.js";

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
const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
const server = createServer();
const client = new Client({ name: "ajisai-trace-eval", version: "1" });
await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);

let selectedCorrectly = 0;
let semanticSuccess = 0;
let irrelevantActivations = 0;
let missing = 0;
for (const testCase of corpus.cases) {
  const trace = traces.get(testCase.id);
  if (!trace) {
    missing += 1;
    console.log(`MISS ${testCase.id}`);
    continue;
  }
  const selectionOk = trace.selectedTool === testCase.expectedTool;
  if (selectionOk) selectedCorrectly += 1;
  if (testCase.expectedTool === null) {
    if (trace.selectedTool !== null) irrelevantActivations += 1;
    if (selectionOk) semanticSuccess += 1;
    console.log(`${selectionOk ? "PASS" : "FAIL"} ${testCase.id} selection`);
    continue;
  }
  if (trace.selectedTool === null) {
    console.log(`FAIL ${testCase.id} no tool selected`);
    continue;
  }
  let result;
  try {
    result = await client.callTool({ name: trace.selectedTool, arguments: trace.arguments ?? {} });
  } catch {
    result = null;
  }
  const semanticsOk = selectionOk && result?.isError !== true && result !== null &&
    Object.entries(testCase.expect).every(([pointer, expected]) =>
      JSON.stringify(atPointer(result.structuredContent, pointer)) === JSON.stringify(expected));
  if (semanticsOk) semanticSuccess += 1;
  console.log(`${semanticsOk ? "PASS" : "FAIL"} ${testCase.id} trace`);
}

await client.close();
await server.close();
const total = corpus.cases.length;
const negative = corpus.cases.filter((testCase) => testCase.expectedTool === null).length;
const metrics = {
  schemaVersion: 1,
  cases: total,
  missingTraces: missing,
  toolSelectionAccuracy: selectedCorrectly / total,
  semanticSuccessRate: semanticSuccess / total,
  irrelevantToolRate: negative === 0 ? 0 : irrelevantActivations / negative,
};
console.log(JSON.stringify(metrics, null, 2));
if (process.argv.includes("--require-perfect") &&
    (missing !== 0 || semanticSuccess !== total || irrelevantActivations !== 0)) {
  process.exit(1);
}
