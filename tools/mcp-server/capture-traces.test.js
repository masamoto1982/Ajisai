#!/usr/bin/env node
// The capture harness, driven by a scripted client.
//
// This tests the harness — prompt assembly, tool-call extraction, the shape of
// what gets written — and **not** a model: the client here returns canned
// responses. That distinction is the whole subject of this round of work, so
// the file states it rather than leaving a reader to infer it from the name.

import assert from "node:assert/strict";
import { captureAll, captureCase, DEFAULT_MODEL, SYSTEM_PROMPT, toolDefinitions } from "./capture-traces.js";
import { indexTraces, mayAssertPerfect, validateCorpus } from "./evaluation-contract.js";

const tools = await toolDefinitions();
assert.deepEqual(
  tools.map(({ name }) => name).sort(),
  ["check", "compute", "infer_contracts", "word_contract"],
  "the model is offered exactly the tools the server publishes",
);
assert.ok(
  tools.every(({ description, input_schema }) => description?.length > 0 && input_schema?.type === "object"),
  "each definition carries the server's own description and input schema",
);

/** A client that answers from a script and records what it was asked. */
function scriptedClient(replies) {
  const requests = [];
  return {
    requests,
    messages: {
      create: async (request) => {
        requests.push(request);
        return replies[requests.length - 1];
      },
    },
  };
}

const compute = {
  content: [
    { type: "text", text: "I'll compute that." },
    { type: "tool_use", id: "t1", name: "compute", input: { source: "1 3 /" } },
  ],
  stop_reason: "tool_use",
};
const noTool = { content: [{ type: "text", text: "That isn't arithmetic." }], stop_reason: "end_turn" };

const client = scriptedClient([compute, noTool]);
const positive = await captureCase(client, {
  model: DEFAULT_MODEL,
  tools,
  testCase: { id: "exact-fraction", prompt: "Compute one third exactly." },
});
assert.deepEqual(positive.selectedTool, "compute");
assert.deepEqual(positive.arguments, { source: "1 3 /" });
assert.deepEqual(positive.observed, { toolCalls: 1, stopReason: "tool_use" });

// The negative half of the corpus. A model that answers an irrelevant prompt
// without touching Ajisai is *correct*, so this has to record `null` rather
// than treat the absence as a missing trace.
const negative = await captureCase(client, {
  model: DEFAULT_MODEL,
  tools,
  testCase: { id: "irrelevant-translation", prompt: "Translate hello into French." },
});
assert.equal(negative.selectedTool, null);
assert.deepEqual(negative.arguments, {});

const [request] = client.requests;
assert.equal(request.system, SYSTEM_PROMPT);
// Forcing a tool call would make every irrelevant-intent case an automatic
// failure of the one metric that measures restraint.
assert.deepEqual(request.tool_choice, { type: "auto" });
assert.equal(request.tools.length, 4);
assert.ok(!("temperature" in request), "sampling parameters are rejected by current models");

// What gets written must be a document the scorers accept — and must be
// classified as a model capture, so no run of it can ever be asserted perfect.
const corpus = JSON.parse(
  (await import("node:fs")).readFileSync(new URL("./eval/cases.json", import.meta.url), "utf8"),
);
const ids = validateCorpus(corpus);
const document = await captureAll({
  client: scriptedClient([compute]),
  model: DEFAULT_MODEL,
  tools,
  cases: [corpus.cases[0]],
});
assert.equal(indexTraces(document, ids).size, 1, "the captured document validates as a trace");
assert.equal(document.provenance.source, "model");
assert.equal(mayAssertPerfect(document), false, "a capture can be reported, never asserted perfect");
for (const field of ["modelId", "promptTemplateDigest", "toolChoice", "capturedAt", "serverVersion", "engineVersion", "registryDigest"]) {
  assert.ok(document.provenance[field], `provenance records ${field}`);
}

console.log("capture harness checks passed (scripted client — not a model measurement)");
