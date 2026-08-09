#!/usr/bin/env node
// A deliberately narrow, source-only MCP boundary around the Ajisai CLI.

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListResourcesRequestSchema,
  ListResourceTemplatesRequestSchema,
  ListToolsRequestSchema,
  ReadResourceRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = process.env.AJISAI_REPO
  ? resolve(process.env.AJISAI_REPO)
  : resolve(here, "..", "..");
const manifestPath = join(repoRoot, "docs", "word-manifest.json");
const contractsPath = join(repoRoot, "spec", "words.json");
const skillPath = join(repoRoot, "SKILL.md");
const rootPackagePath = join(repoRoot, "package.json");
const serverPackagePath = join(here, "package.json");
export const LIMITS = Object.freeze({
  sourceBytes: 64 * 1024,
  wallTimeMs: 5_000,
  responseBytes: 1024 * 1024,
  executionSteps: 100_000,
  concurrentExecutions: 4,
});

function resolveAjisaiBin() {
  if (process.env.AJISAI_BIN) return process.env.AJISAI_BIN;
  for (const profile of ["debug", "release"]) {
    const candidate = join(repoRoot, "rust", "target", profile, "ajisai");
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

const sourceSchema = {
  type: "object",
  additionalProperties: false,
  properties: {
    source: {
      type: "string",
      maxLength: LIMITS.sourceBytes,
      description: "Ajisai source text (UTF-8; file paths are not accepted).",
    },
  },
  required: ["source"],
};
const envelopeSchema = {
  type: "object",
  additionalProperties: true,
  required: ["schemaVersion", "status", "mcp"],
  properties: {
    schemaVersion: { type: "integer" },
    status: { type: "string", enum: ["ok", "error"] },
    mcp: {
      type: "object",
      required: ["engineVersion", "registryDigest", "limits"],
      properties: {
        engineVersion: { type: "string" },
        registryDigest: { type: "string", pattern: "^[a-f0-9]{64}$" },
        limits: { type: "object" },
      },
    },
  },
};
const TOOLS = [
  {
    name: "compute",
    description: "Execute a bounded Ajisai program. Use for supported-domain exact rational, decimal, square-root and vector calculations, including reason-carrying NIL results.",
    inputSchema: sourceSchema,
    outputSchema: envelopeSchema,
  },
  {
    name: "check",
    description: "Parse and resolve Ajisai source without executing it; also verifies declared contracts conservatively.",
    inputSchema: sourceSchema,
    outputSchema: envelopeSchema,
  },
  {
    name: "infer_contracts",
    description: "Infer machine-readable contracts for user-defined Words without executing their bodies.",
    inputSchema: sourceSchema,
    outputSchema: envelopeSchema,
  },
  {
    name: "word_contract",
    description: "Return the generated canonical registry entry for a Word or alias.",
    inputSchema: {
      type: "object",
      additionalProperties: false,
      properties: { word: { type: "string", minLength: 1 } },
      required: ["word"],
    },
    outputSchema: {
      type: "object",
      additionalProperties: false,
      properties: {
        schemaVersion: { type: "integer" },
        registryDigest: { type: "string", pattern: "^[a-f0-9]{64}$" },
        matches: { type: "array", items: { type: "object" } },
      },
      required: ["schemaVersion", "registryDigest", "matches"],
    },
  },
];

let manifestCache;
let contractsCache;
let registryDigestCache;
let engineVersionCache;
let serverVersionCache;
export class ExecutionGate {
  #active = 0;

  constructor(capacity) {
    this.capacity = capacity;
  }

  tryAcquire() {
    if (this.#active >= this.capacity) return false;
    this.#active += 1;
    return true;
  }

  release() {
    if (this.#active === 0) throw new Error("execution gate released without a holder");
    this.#active -= 1;
  }
}
const executionGate = new ExecutionGate(LIMITS.concurrentExecutions);
function manifest() { return manifestCache ??= JSON.parse(readFileSync(manifestPath, "utf8")); }
function contracts() {
  return contractsCache ??= JSON.parse(readFileSync(contractsPath, "utf8"));
}
function registryDigest() {
  return registryDigestCache ??= createHash("sha256")
    .update(readFileSync(contractsPath))
    .digest("hex");
}
function engineVersion() {
  return engineVersionCache ??= JSON.parse(readFileSync(rootPackagePath, "utf8")).version;
}
function serverVersion() {
  return serverVersionCache ??= JSON.parse(readFileSync(serverPackagePath, "utf8")).version;
}
function result(value) { return { content: [{ type: "text", text: JSON.stringify(value, null, 2) }], structuredContent: value }; }
function fail(text) { return { content: [{ type: "text", text }], isError: true }; }

async function runCli(source, command) {
  if (typeof source !== "string" || source.length === 0) return fail("Provide non-empty `source` text.");
  if (Buffer.byteLength(source, "utf8") > LIMITS.sourceBytes) return fail(`source exceeds ${LIMITS.sourceBytes} UTF-8 bytes`);
  const bin = resolveAjisaiBin();
  if (!bin) return fail("ajisai CLI not found; build rust/ or set AJISAI_BIN.");
  if (!executionGate.tryAcquire()) {
    return fail(
      `Ajisai execution capacity is full (${LIMITS.concurrentExecutions}); retry later.`,
    );
  }
  let scratch;
  try {
    scratch = mkdtempSync(join(tmpdir(), "ajisai-mcp-"));
    const target = join(scratch, "program.ajisai");
    writeFileSync(target, source.endsWith("\n") ? source : `${source}\n`, { mode: 0o600 });
    const args = [command, target, "--json"];
    if (command === "run") args.push("--step-limit", String(LIMITS.executionSteps));
    if (command === "check") args.push("--contract");
    let stdout;
    try {
      ({ stdout } = await execFileAsync(bin, args, { encoding: "utf8", timeout: LIMITS.wallTimeMs, maxBuffer: LIMITS.responseBytes }));
    } catch (error) {
      // Ajisai language errors intentionally exit 1 with a JSON diagnosis.
      // Timeouts, max-buffer failures and CLI usage failures are host errors,
      // even when the child managed to write a partial stdout document.
      if (error.killed || error.signal) {
        return fail(`Ajisai exceeded ${LIMITS.wallTimeMs} ms`);
      }
      if (error.code !== 1 || !error.stdout) {
        return fail(`failed to run Ajisai: ${error.message}`);
      }
      stdout = error.stdout;
    }
    let envelope;
    try { envelope = JSON.parse(stdout); } catch { return fail("Ajisai returned a non-JSON response."); }
    // `contract --json` predates the common CLI envelope and returns a bare
    // array. Normalize it at this adapter boundary so all execution tools
    // honor their advertised MCP output schema.
    if (command === "contract") {
      envelope = {
        schemaVersion: 1,
        status: "ok",
        contracts: envelope,
      };
    }
    envelope.mcp = {
      engineVersion: engineVersion(),
      registryDigest: registryDigest(),
      limits: LIMITS,
    };
    return result(envelope);
  } finally {
    if (scratch) rmSync(scratch, { recursive: true, force: true });
    executionGate.release();
  }
}

function wordContract(word) {
  const needle = typeof word === "string" ? word.trim().toUpperCase() : "";
  if (!needle) return fail("Provide a `word`.");
  const matches = (contracts().entries ?? []).filter((entry) =>
    entry.name.toUpperCase() === needle ||
    entry.aliases.some((alias) => alias.toUpperCase() === needle)
  );
  return result({
    schemaVersion: contracts().schemaVersion,
    registryDigest: registryDigest(),
    matches,
  });
}

const RESOURCES = [
  { uri: "ajisai://guide/quickstart", name: "Ajisai agent quickstart", mimeType: "text/markdown" },
  { uri: "ajisai://vocabulary", name: "Ajisai generated Word vocabulary", mimeType: "application/json" },
  { uri: "ajisai://schema/result", name: "Ajisai MCP result contract", mimeType: "application/json" },
];
const RESOURCE_TEMPLATES = [
  {
    uriTemplate: "ajisai://words/{name}",
    name: "Ajisai canonical Word contract",
    description: "The complete spec/words.json contract for a Word or alias.",
    mimeType: "application/json",
  },
];

export function createServer() {
  const server = new Server(
    { name: "ajisai", version: serverVersion() },
    { capabilities: { tools: {}, resources: {} } },
  );
  server.setRequestHandler(ListToolsRequestSchema, async () => ({ tools: TOOLS }));
  server.setRequestHandler(CallToolRequestSchema, async ({ params }) => {
    const args = params.arguments ?? {};
    if (params.name === "compute") return runCli(args.source, "run");
    if (params.name === "check") return runCli(args.source, "check");
    if (params.name === "infer_contracts") return runCli(args.source, "contract");
    if (params.name === "word_contract") return wordContract(args.word);
    return fail(`unknown tool: ${params.name}`);
  });
  server.setRequestHandler(ListResourcesRequestSchema, async () => ({ resources: RESOURCES }));
  server.setRequestHandler(ListResourceTemplatesRequestSchema, async () => ({
    resourceTemplates: RESOURCE_TEMPLATES,
  }));
  server.setRequestHandler(ReadResourceRequestSchema, async ({ params }) => {
    const uri = params.uri;
    if (uri === "ajisai://guide/quickstart") return { contents: [{ uri, mimeType: "text/markdown", text: readFileSync(skillPath, "utf8") }] };
    if (uri === "ajisai://vocabulary") return { contents: [{ uri, mimeType: "application/json", text: JSON.stringify(manifest(), null, 2) }] };
    if (uri === "ajisai://schema/result") return { contents: [{ uri, mimeType: "application/json", text: JSON.stringify(envelopeSchema, null, 2) }] };
    if (uri.startsWith("ajisai://words/")) {
      const name = decodeURIComponent(uri.slice("ajisai://words/".length));
      const contract = wordContract(name);
      if (contract.isError) throw new Error(contract.content[0].text);
      return {
        contents: [{
          uri,
          mimeType: "application/json",
          text: JSON.stringify(contract.structuredContent, null, 2),
        }],
      };
    }
    throw new Error(`unknown resource: ${uri}`);
  });
  return server;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await createServer().connect(new StdioServerTransport());
