#!/usr/bin/env node
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import Ajv2020 from "ajv/dist/2020.js";
import { createServer, ExecutionGate, LIMITS } from "./index.js";
import { readFileSync } from "node:fs";

let failures = 0;
function check(label, condition) {
  console.log(`${condition ? "PASS" : "FAIL"}  ${label}`);
  if (!condition) failures += 1;
}

function canonicalJson(value) {
  if (Array.isArray(value)) return value.map(canonicalJson);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, child]) => [key, canonicalJson(child)]),
    );
  }
  return value;
}

function atPointer(document, pointer) {
  return pointer
    .split("/")
    .slice(1)
    .map((part) => part.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((value, part) => value?.[part], document);
}

const gate = new ExecutionGate(2);
check("execution gate admits up to its capacity", gate.tryAcquire() && gate.tryAcquire());
check("execution gate rejects excess concurrent work", gate.tryAcquire() === false);
gate.release();
check("execution gate restores capacity on release", gate.tryAcquire());
gate.release();
gate.release();

const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
const server = createServer();
const client = new Client({ name: "selftest", version: "0.0.0" });
await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);

const { tools } = await client.listTools();
check(
  "exposes the four focused agent tools",
  JSON.stringify(tools.map(({ name }) => name).sort()) ===
    JSON.stringify(["check", "compute", "infer_contracts", "word_contract"]),
);
check(
  "every tool rejects undeclared input",
  tools.every(({ inputSchema }) => inputSchema.additionalProperties === false),
);
check(
  "execution tools advertise structured output",
  tools
    .filter(({ name }) => name !== "word_contract")
    .every(({ outputSchema }) => outputSchema?.required?.includes("mcp")),
);
check(
  "tools advertise safe selection hints",
  tools.every(({ annotations }) =>
    annotations?.readOnlyHint === true &&
    annotations?.destructiveHint === false &&
    annotations?.idempotentHint === true &&
    annotations?.openWorldHint === false
  ),
);

const contract = await client.callTool({ name: "word_contract", arguments: { word: "map" } });
check(
  "word_contract returns the complete canonical contract",
  contract.structuredContent?.matches?.some((entry) =>
    entry.name === "MAP" &&
    entry.purity === "conditional" &&
    entry.stack?.inputs === 2
  ),
);

const resources = await client.listResources();
check("publishes guide, vocabulary and result schema as resources", resources.resources.length === 3);
const guide = await client.readResource({ uri: "ajisai://guide/quickstart" });
check("quickstart resource reads generated guidance", guide.contents[0]?.text?.includes("Agent Writing Protocol"));
const schemaResource = await client.readResource({ uri: "ajisai://schema/result" });
const resultSchema = JSON.parse(schemaResource.contents[0]?.text ?? "{}");
const validateResult = new Ajv2020().compile(resultSchema);
check(
  "result resource publishes the exact algebraic wire schema",
  resultSchema.$id === "ajisai://schema/result" &&
    resultSchema.$defs?.exactTerm?.required?.includes("radicand"),
);
check(
  "tool output and result resource use the same schema",
  JSON.stringify(canonicalJson(tools.find(({ name }) => name === "compute")?.outputSchema)) ===
    JSON.stringify(canonicalJson(resultSchema)),
);
const templates = await client.listResourceTemplates();
check(
  "publishes canonical Word contracts as a resource template",
  templates.resourceTemplates.some(({ uriTemplate }) => uriTemplate === "ajisai://words/{name}"),
);
const mapResource = await client.readResource({ uri: "ajisai://words/MAP" });
const mapContract = JSON.parse(mapResource.contents[0]?.text ?? "{}");
check(
  "Word resource resolves the canonical contract",
  mapContract.matches?.[0]?.name === "MAP" && mapContract.matches[0]?.nilPolicy,
);

const golden = JSON.parse(
  readFileSync(new URL("./golden/cases.json", import.meta.url), "utf8"),
);
for (const goldenCase of golden.cases) {
  const observed = await client.callTool({
    name: "compute",
    arguments: { source: goldenCase.source },
  });
  const mismatches = Object.entries(goldenCase.expect).filter(
    ([pointer, expected]) =>
      JSON.stringify(atPointer(observed.structuredContent, pointer)) !==
      JSON.stringify(expected),
  );
  check(
    `golden: ${goldenCase.name}`,
    observed.isError !== true && mismatches.length === 0,
  );
}

const compute = await client.callTool({
  name: "compute",
  arguments: { source: "[ 2 ] SQRT" },
});
const oversized = await client.callTool({
  name: "compute",
  arguments: { source: " ".repeat(LIMITS.sourceBytes + 1) },
});
check("compute rejects source beyond its UTF-8 byte limit", oversized.isError === true);
if (compute.isError && compute.content?.[0]?.text?.includes("CLI not found")) {
  check("compute requires a real Ajisai backend", false);
} else {
  const sqrt = compute.structuredContent?.stack?.[0]?.value?.[0];
  const [exactTerm] = sqrt?.semantics?.exactTerms ?? [];
  check(
    "compute preserves the exact algebraic normal form",
    exactTerm?.numerator === "1" &&
      exactTerm?.denominator === "1" &&
      exactTerm?.radicand === "2",
  );
  check(
    "compute reports engine provenance and applied limits",
    compute.structuredContent?.mcp?.engineVersion === "0.2.0-beta.1" &&
      compute.structuredContent?.mcp?.limits?.wallTimeMs === 5000 &&
      compute.structuredContent?.mcp?.limits?.materializedElements === 100000 &&
      compute.structuredContent?.mcp?.limits?.bigintBits === 262144 &&
      compute.structuredContent?.mcp?.limits?.algebraicTerms === 4096,
  );
  check("compute satisfies the published result schema", validateResult(compute.structuredContent));

  const languageError = await client.callTool({
    name: "compute",
    arguments: { source: "FROBNICATE" },
  });
  check(
    "Ajisai language errors remain structured non-MCP errors",
    languageError.isError !== true && languageError.structuredContent?.status === "error",
  );

  const checked = await client.callTool({
    name: "check",
    arguments: { source: "{ [ 1 ] + } 'INC' DEF" },
  });
  check("check is execution-free and structured", checked.structuredContent?.status === "ok");

  const inferred = await client.callTool({
    name: "infer_contracts",
    arguments: { source: "{ [ 1 ] + } 'INC' DEF" },
  });
  check(
    "infer_contracts returns the user Word contract",
    inferred.structuredContent?.contracts?.some((entry) => entry.name === "INC"),
  );
  check(
    "infer_contracts satisfies the published result schema",
    validateResult(inferred.structuredContent),
  );
}

await client.close();
await server.close();
if (failures) process.exit(1);
console.log("all checks passed");
