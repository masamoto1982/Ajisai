#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "./index.js";
import { validateCorpus } from "./evaluation-contract.js";

function atPointer(document, pointer) {
  return pointer
    .split("/")
    .slice(1)
    .map((part) => part.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((value, part) => value?.[part], document);
}

const corpus = JSON.parse(readFileSync(new URL("./eval/cases.json", import.meta.url), "utf8"));
validateCorpus(corpus);
const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
const server = createServer();
const client = new Client({ name: "ajisai-eval", version: "1" });
await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);

let passed = 0;
for (const testCase of corpus.cases) {
  if (testCase.expectedTool === null) {
    passed += 1;
    console.log(`PASS ${testCase.id} (no Ajisai tool expected)`);
    continue;
  }
  const result = await client.callTool({
    name: testCase.expectedTool,
    arguments: testCase.arguments,
  });
  const mismatches = Object.entries(testCase.expect).filter(
    ([pointer, expected]) =>
      JSON.stringify(atPointer(result.structuredContent, pointer)) !== JSON.stringify(expected),
  );
  const ok = result.isError !== true && mismatches.length === 0;
  if (ok) passed += 1;
  console.log(`${ok ? "PASS" : "FAIL"} ${testCase.id}`);
  for (const [pointer, expected] of mismatches) {
    console.log(`  ${pointer}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(atPointer(result.structuredContent, pointer))}`);
  }
}

await client.close();
await server.close();
console.log(`${passed}/${corpus.cases.length} backend-semantic cases passed`);
if (passed !== corpus.cases.length) process.exit(1);
