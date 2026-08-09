#!/usr/bin/env node
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "./index.js";

let failures = 0;
function check(label, condition) {
  console.log(`${condition ? "PASS" : "FAIL"}  ${label}`);
  if (!condition) failures += 1;
}

const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
const server = createServer();
const client = new Client({ name: "selftest", version: "0.0.0" });
await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);

const { tools } = await client.listTools();
check("exposes the four focused agent tools", JSON.stringify(tools.map(({ name }) => name).sort()) === JSON.stringify(["check", "compute", "infer_contracts", "word_contract"]));
check("every tool rejects undeclared input", tools.every(({ inputSchema }) => inputSchema.additionalProperties === false));
check("execution tools advertise structured output", tools.filter(({ name }) => name !== "word_contract").every(({ outputSchema }) => outputSchema?.required?.includes("mcp")));

const contract = await client.callTool({ name: "word_contract", arguments: { word: "map" } });
check("word_contract returns structured registry data", contract.structuredContent?.matches?.some((entry) => entry.surface === "MAP"));

const resources = await client.listResources();
check("publishes guide, vocabulary and result schema as resources", resources.resources.length === 3);
const guide = await client.readResource({ uri: "ajisai://guide/quickstart" });
check("quickstart resource reads generated guidance", guide.contents[0]?.text?.includes("Agent Writing Protocol"));

const compute = await client.callTool({ name: "compute", arguments: { source: "[ 1 ] [ 2 ] + PRINT" } });
if (compute.isError && compute.content?.[0]?.text?.includes("CLI not found")) {
  console.log("SKIP  compute (build the Ajisai CLI or set AJISAI_BIN for the integration assertion)");
} else {
  check("compute returns structured engine output and applied limits", compute.structuredContent?.status === "ok" && compute.structuredContent?.mcp?.limits?.wallTimeMs === 5000);
}

await client.close();
await server.close();
if (failures) process.exit(1);
console.log("all checks passed");
