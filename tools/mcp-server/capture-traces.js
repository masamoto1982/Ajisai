#!/usr/bin/env node
// Capture what a real model does with the Ajisai tools.
//
// Everything else in `eval/` grades a trace. Nothing produced one: the two
// committed traces are hand-written fixtures that prove the scorers run, and
// the readiness tracker has said for three rounds that the harness is not the
// bottleneck — real traces are. This is the missing half.
//
// It drives the actual model over the actual MCP tool definitions, one call per
// corpus case, and writes a trace the scorers accept. What it does *not* do is
// produce a number when it cannot measure one: with no resolvable credentials
// it exits non-zero having written nothing, because a file that looks like a
// trace and isn't is worse than no file.
//
// Usage:
//   node capture-traces.js [--model <id>] [--out <path>] [--limit <n>]
//
// Credentials resolve the way the Anthropic SDK resolves them (ANTHROPIC_API_KEY,
// ANTHROPIC_AUTH_TOKEN, or an `ant auth login` profile) — this file reads none
// of them itself.

import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { engineVersion, registryDigest, serverVersion } from "./index.js";
import { validateCorpus } from "./evaluation-contract.js";

const here = dirname(fileURLToPath(import.meta.url));

/**
 * What the model is told, verbatim.
 *
 * Deliberately thin. The point of a baseline is to measure what the published
 * tool descriptions and quickstart achieve on their own — a system prompt that
 * coaches tool selection would measure the prompt instead, and every later
 * comparison would be against a moving target. Its digest is recorded with the
 * trace so a changed prompt cannot be mistaken for a changed model.
 */
export const SYSTEM_PROMPT =
  "You have access to Ajisai, a bounded exact-arithmetic engine, through the " +
  "tools below. Use a tool when it is the right instrument for the request, " +
  "and answer directly when it is not.";

export const DEFAULT_MODEL = "claude-opus-5";

function digest(text) {
  return createHash("sha256").update(text).digest("hex").slice(0, 16);
}

function argValue(argv, flag, fallback) {
  const index = argv.indexOf(flag);
  return index === -1 ? fallback : argv[index + 1];
}

/**
 * The four tools, exactly as a connected client would see them.
 *
 * Read from the server's own declaration rather than restated here: a baseline
 * measured against a hand-copied description would drift from the product the
 * moment either changed, and would then be comparing two different things
 * across the PRs it exists to compare.
 */
export async function toolDefinitions() {
  const { Client } = await import("@modelcontextprotocol/sdk/client/index.js");
  const { InMemoryTransport } = await import("@modelcontextprotocol/sdk/inMemory.js");
  const { createServer } = await import("./index.js");
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  const server = createServer();
  const client = new Client({ name: "ajisai-trace-capture", version: "1" });
  await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);
  try {
    const { tools } = await client.listTools();
    return tools.map(({ name, description, inputSchema }) => ({
      name,
      description,
      input_schema: inputSchema,
    }));
  } finally {
    await client.close();
    await server.close();
  }
}

/**
 * One case: ask once, record what the model reached for.
 *
 * `tool_choice` is `auto` and must stay that way — a third of the corpus is
 * irrelevant-intent cases whose correct answer is *no* tool call, and forcing a
 * call would score the one metric that measures restraint as a guaranteed
 * failure.
 */
export async function captureCase(client, { model, tools, testCase }) {
  const response = await client.messages.create({
    model,
    // Thinking is on by default on current models and shares this budget with
    // the response, so the ceiling is generous: a truncated turn would be
    // recorded as a model that declined to call a tool.
    max_tokens: 16000,
    system: SYSTEM_PROMPT,
    tools,
    tool_choice: { type: "auto" },
    messages: [{ role: "user", content: testCase.prompt }],
  });
  const toolUses = (response.content ?? []).filter((block) => block.type === "tool_use");
  const [first] = toolUses;
  return {
    caseId: testCase.id,
    // `null` is a real answer, not a missing one: it is what a correct response
    // to an irrelevant prompt looks like.
    selectedTool: first?.name ?? null,
    arguments: first?.input ?? {},
    // Observations the scorers do not consume yet, recorded because they cannot
    // be recovered later: a second call in one turn, and a turn that stopped
    // for a reason other than finishing.
    observed: { toolCalls: toolUses.length, stopReason: response.stop_reason ?? null },
  };
}

/**
 * Resolve the SDK client, or explain why there is nothing to run.
 *
 * Constructing a client does not check credentials — the SDK resolves them
 * when it builds the first request's headers, so a missing key surfaces as a
 * stack trace partway through a run rather than as a refusal to start. This
 * turns that into one actionable line, which `captureFailure` completes for
 * the failure that arrives late.
 */
async function connect() {
  let Anthropic;
  try {
    ({ default: Anthropic } = await import("@anthropic-ai/sdk"));
  } catch {
    throw new Error(
      "the Anthropic SDK is not installed; run `npm install` in tools/mcp-server",
    );
  }
  return new Anthropic();
}

const MISSING_CREDENTIALS = /Could not resolve authentication|authentication_error|invalid x-api-key/i;

/** The message for a failed run. Nothing is ever written on this path. */
function captureFailure(error) {
  if (MISSING_CREDENTIALS.test(error?.message ?? "")) {
    return (
      "no Anthropic credentials could be resolved. Set ANTHROPIC_API_KEY, or run `ant auth login`.\n" +
      "Nothing was written — a trace file that was not produced by a model is worse than no file."
    );
  }
  return `capture failed: ${error?.message ?? error}\nNothing was written.`;
}

export async function captureAll({ client, model, tools, cases, onCase = () => {} }) {
  const traces = [];
  for (const testCase of cases) {
    const trace = await captureCase(client, { model, tools, testCase });
    traces.push(trace);
    onCase(trace);
  }
  return {
    schemaVersion: 1,
    provenance: {
      source: "model",
      modelId: model,
      promptTemplateDigest: digest(SYSTEM_PROMPT),
      toolChoice: "auto",
      capturedAt: new Date().toISOString(),
      serverVersion: serverVersion(),
      engineVersion: engineVersion(),
      registryDigest: registryDigest(),
    },
    traces,
  };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const model = argValue(process.argv, "--model", DEFAULT_MODEL);
  const limit = Number(argValue(process.argv, "--limit", "0")) || Infinity;
  const corpus = JSON.parse(readFileSync(new URL("./eval/cases.json", import.meta.url), "utf8"));
  validateCorpus(corpus);
  const cases = corpus.cases.slice(0, limit);
  // Real traces live apart from the committed fixtures, so no directory listing
  // ever presents the two as the same kind of artifact.
  const out = argValue(
    process.argv,
    "--out",
    `${here}/eval/traces/${model}-${new Date().toISOString().replaceAll(":", "-")}.json`,
  );

  let document;
  try {
    const client = await connect();
    document = await captureAll({
      client,
      model,
      tools: await toolDefinitions(),
      cases,
      onCase: ({ caseId, selectedTool }) => console.error(`  ${caseId} -> ${selectedTool ?? "(no tool)"}`),
    });
  } catch (error) {
    console.error(captureFailure(error));
    process.exit(2);
  }
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, `${JSON.stringify(document, null, 2)}\n`);
  console.log(out);
  console.error(
    `captured ${document.traces.length} traces for ${model}; score with:\n` +
      `  node score-traces.js ${out}`,
  );
}
