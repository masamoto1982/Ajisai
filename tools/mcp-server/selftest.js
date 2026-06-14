#!/usr/bin/env node
// End-to-end self-test: drive the real MCP protocol (list tools + call each
// of the three) over an in-memory transport pair, asserting the wrapper
// faithfully relays the CLI and the generated artifacts. Run: `npm run selftest`.
//
// Requires the ajisai CLI to be built (or AJISAI_BIN set) for the `run`
// assertions; the manifest/skill assertions only need the committed artifacts.

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { createServer } from "./index.js";

let failures = 0;
function check(label, cond, detail = "") {
  if (cond) {
    console.log(`PASS  ${label}`);
  } else {
    failures += 1;
    console.log(`FAIL  ${label}${detail ? `\n      ${detail}` : ""}`);
  }
}

const [clientTransport, serverTransport] =
  InMemoryTransport.createLinkedPair();
const server = createServer();
const client = new Client({ name: "selftest", version: "0.0.0" });
await Promise.all([
  server.connect(serverTransport),
  client.connect(clientTransport),
]);

// 1. tools/list exposes exactly the three documented tools.
const { tools } = await client.listTools();
const names = tools.map((t) => t.name).sort();
check(
  "lists exactly run/explain_word/skill",
  JSON.stringify(names) === JSON.stringify(["explain_word", "run", "skill"]),
  `got ${JSON.stringify(names)}`,
);

// 2. skill returns SKILL.md (its generated header is a stable marker).
const skill = await client.callTool({ name: "skill", arguments: {} });
const skillText = skill.content?.[0]?.text ?? "";
check(
  "skill returns the generated SKILL.md",
  skillText.includes("Agent Writing Protocol") && skillText.includes("§9"),
  `first line: ${skillText.split("\n")[0]}`,
);

// 3. explain_word resolves a core word from the manifest.
const explain = await client.callTool({
  name: "explain_word",
  arguments: { word: "map" },
});
let explainEntries = [];
try {
  explainEntries = JSON.parse(explain.content?.[0]?.text ?? "[]");
} catch {
  /* leave empty -> fails below */
}
check(
  "explain_word(map) returns the MAP coreword entry",
  Array.isArray(explainEntries) &&
    explainEntries.some((e) => e.surface === "MAP" && e.kind === "coreword"),
  `got ${explain.content?.[0]?.text?.slice(0, 80)}`,
);

// 4. explain_word on a nonexistent word reports an error result.
const explainMiss = await client.callTool({
  name: "explain_word",
  arguments: { word: "definitely-not-a-word" },
});
check("explain_word(unknown) is an error result", explainMiss.isError === true);

// 5. run executes a program and relays the CLI's --json envelope.
const run = await client.callTool({
  name: "run",
  arguments: { source: "[ 1 ] [ 2 ] + PRINT" },
});
let runEnvelope = null;
try {
  runEnvelope = JSON.parse(run.content?.[0]?.text ?? "{}");
} catch {
  /* leave null -> fails below */
}
const cliMissing =
  run.isError && (run.content?.[0]?.text ?? "").includes("CLI not found");
if (cliMissing) {
  console.log("SKIP  run (ajisai CLI not built; set AJISAI_BIN or cargo build)");
} else {
  check(
    "run relays a valid --json envelope",
    runEnvelope && runEnvelope.schemaVersion === 1 && runEnvelope.status === "ok",
    `got ${run.content?.[0]?.text?.slice(0, 80)}`,
  );
  check(
    "run reports PRINT output and energyProxyScore",
    runEnvelope &&
      Array.isArray(runEnvelope.output) &&
      runEnvelope.output.join(" ").includes("3/1") &&
      typeof runEnvelope.runtimeMetrics?.vtu?.energyProxyScore === "number",
    `output: ${JSON.stringify(runEnvelope?.output)}`,
  );

  // 6. run on a language error still returns a JSON envelope (status:error).
  const runErr = await client.callTool({
    name: "run",
    arguments: { source: "[ 1 ] FROBNICATE" },
  });
  let errEnvelope = null;
  try {
    errEnvelope = JSON.parse(runErr.content?.[0]?.text ?? "{}");
  } catch {
    /* leave null */
  }
  check(
    "run surfaces a language error as a status:error envelope",
    errEnvelope &&
      errEnvelope.status === "error" &&
      errEnvelope.diagnosis?.why === "typoOrUnknownName",
    `got ${runErr.content?.[0]?.text?.slice(0, 80)}`,
  );
}

await client.close();
await server.close();

console.log("----");
if (failures > 0) {
  console.log(`${failures} check(s) failed`);
  process.exit(1);
}
console.log("all checks passed");
