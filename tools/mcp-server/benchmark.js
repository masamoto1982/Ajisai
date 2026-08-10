#!/usr/bin/env node
import { performance } from "node:perf_hooks";
import { readFileSync } from "node:fs";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "./index.js";
import { validateCorpus } from "./evaluation-contract.js";

function read(relative) {
  return JSON.parse(readFileSync(new URL(relative, import.meta.url), "utf8"));
}

function percentile(sorted, fraction) {
  return sorted[Math.max(0, Math.ceil(sorted.length * fraction) - 1)];
}

function atPointer(document, pointer) {
  return pointer.split("/").slice(1)
    .map((part) => part.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((value, part) => value?.[part], document);
}

const corpus = read("./eval/cases.json");
validateCorpus(corpus);
const config = read("./eval/performance.json");
if (config.schemaVersion !== 1 || !Number.isInteger(config.measuredRuns) || config.measuredRuns < 1 ||
    !Number.isInteger(config.warmupRuns) || config.warmupRuns < 0 || !(config.p95BudgetMs > 0) ||
    !Array.isArray(config.caseIds) || config.caseIds.length === 0) {
  throw new Error("performance configuration is invalid");
}
const cases = new Map(corpus.cases.map((testCase) => [testCase.id, testCase]));
const selected = config.caseIds.map((id) => {
  const testCase = cases.get(id);
  if (!testCase || testCase.expectedTool === null) throw new Error(`unknown performance case: ${id}`);
  return testCase;
});

const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
const server = createServer();
const client = new Client({ name: "ajisai-performance-eval", version: "1" });
await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);
const measurements = [];
try {
  for (let round = -config.warmupRuns; round < config.measuredRuns; round += 1) {
    for (const testCase of selected) {
      const start = performance.now();
      const result = await client.callTool({
        name: testCase.expectedTool,
        arguments: testCase.arguments,
      });
      const elapsedMs = performance.now() - start;
      const semanticsOk = result.isError !== true && Object.entries(testCase.expect)
        .every(([pointer, expected]) =>
          JSON.stringify(atPointer(result.structuredContent, pointer)) === JSON.stringify(expected));
      if (!semanticsOk) throw new Error(`performance case failed: ${testCase.id}`);
      if (round >= 0) measurements.push({ caseId: testCase.id, tool: testCase.expectedTool, elapsedMs });
    }
  }
} finally {
  await client.close();
  await server.close();
}

const elapsed = measurements.map(({ elapsedMs }) => elapsedMs).sort((a, b) => a - b);
const byTool = Object.fromEntries([...new Set(measurements.map(({ tool }) => tool))].map((tool) => {
  const values = measurements.filter((sample) => sample.tool === tool)
    .map(({ elapsedMs }) => elapsedMs).sort((a, b) => a - b);
  return [tool, {
    samples: values.length,
    p50Ms: percentile(values, 0.5),
    p95Ms: percentile(values, 0.95),
    maxMs: values.at(-1),
  }];
}));
const report = {
  schemaVersion: 1,
  measuredRuns: config.measuredRuns,
  cases: selected.length,
  samples: measurements.length,
  budget: { p95Ms: config.p95BudgetMs },
  overall: {
    p50Ms: percentile(elapsed, 0.5),
    p95Ms: percentile(elapsed, 0.95),
    maxMs: elapsed.at(-1),
  },
  byTool,
};
console.log(JSON.stringify(report, null, 2));
if (process.argv.includes("--require-budget") && report.overall.p95Ms > config.p95BudgetMs) {
  console.error(`p95 ${report.overall.p95Ms.toFixed(1)} ms exceeds ${config.p95BudgetMs} ms budget`);
  process.exit(1);
}
