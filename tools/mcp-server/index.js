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
import { createHash } from "node:crypto";
import { existsSync, readFileSync, realpathSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { NativeCliBackend } from "./backend/native-cli.js";
import { WasmWorkerBackend } from "./backend/wasm-worker.js";
import { HostError, logHostError } from "./host-error.js";
import { suggestWords } from "./word-candidates.js";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = process.env.AJISAI_REPO
  ? resolve(process.env.AJISAI_REPO)
  : resolve(here, "..", "..");
const assetsPath = join(here, "assets");
const manifestPath = join(assetsPath, "word-manifest.json");
const contractsPath = join(assetsPath, "words.json");
const skillPath = join(assetsPath, "quickstart.md");
const metadataPath = join(assetsPath, "metadata.json");
const serverPackagePath = join(here, "package.json");
const resultSchemaPath = join(here, "result.schema.json");
export const LIMITS = Object.freeze({
  sourceBytes: 64 * 1024,
  wallTimeMs: 5_000,
  responseBytes: 1024 * 1024,
  executionSteps: 100_000,
  concurrentExecutions: 4,
  materializedElements: 100_000,
  numericLiteralDigits: 4_096,
  numericWork: 10_000_000,
  bigintBits: 262_144,
  algebraicTerms: 4_096,
});
/**
 * How long a saturated server waits for an execution slot before answering
 * `capacityExhausted`.
 *
 * A ceiling that answers "full, try again" the instant it is reached turns
 * every burst into a retry loop the caller has to write. A short queue absorbs
 * the burst instead: the ceiling on *concurrent* work is unchanged, and only
 * a caller that would have waited longer than this ever sees the error.
 */
export const CAPACITY_WAIT_MS = 1_000;

function resolveAjisaiBin() {
  if (process.env.AJISAI_BIN) return process.env.AJISAI_BIN;
  for (const profile of ["debug", "release"]) {
    const candidate = join(repoRoot, "rust", "target", profile, "ajisai");
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

// Backend selection: the packaged WASM worker needs neither `AJISAI_REPO` nor
// `AJISAI_BIN` and is the self-contained default. A native binary — found via
// the explicit `AJISAI_BIN` override, or discovered under a checked-out
// `rust/` (local development, Docker images that build it in) — takes
// precedence when present. Both backends return the identical schema-1
// envelope (`docs/dev/agent-cli-output-contract.md`), verified case-by-case in
// `backend/parity-test.js`, so the two agree on every result.
//
// Resolved **once**, at server construction, and reported in `mcp.backend`.
// Re-resolving per request meant a `cargo build` finishing mid-session
// silently moved later calls from the WASM backend to the native one: a
// deterministic kernel whose execution path could change under it between two
// identical calls, with nothing in the response saying so. Parity keeps the
// two answers equal; provenance is what makes an unequal one diagnosable.
export function createBackend() {
  const bin = resolveAjisaiBin();
  if (bin) {
    return new NativeCliBackend({
      bin,
      wallTimeMs: LIMITS.wallTimeMs,
      responseBytes: LIMITS.responseBytes,
      executionSteps: LIMITS.executionSteps,
    });
  }
  if (WasmWorkerBackend.isAvailable()) {
    return new WasmWorkerBackend({
      wallTimeMs: LIMITS.wallTimeMs,
      executionSteps: LIMITS.executionSteps,
      responseBytes: LIMITS.responseBytes,
    });
  }
  return null;
}

let selectedBackend;
let backendResolved = false;
function backend() {
  if (!backendResolved) {
    selectedBackend = createBackend();
    backendResolved = true;
  }
  return selectedBackend;
}

const sourceSchema = {
  type: "object",
  additionalProperties: false,
  properties: {
    source: {
      type: "string",
      // JSON Schema counts characters; the server counts UTF-8 bytes, and one
      // character is never fewer than one byte — so this is the coarse
      // character bound the byte budget implies, and never rejects source the
      // byte check would have accepted. Source that passes here can still be
      // rejected as `sourceTooLarge`, which is why the effective unit is
      // stated in the description rather than left to be inferred.
      maxLength: LIMITS.sourceBytes,
      description: `Ajisai source text (file paths are not accepted). The effective limit is ${LIMITS.sourceBytes} UTF-8 bytes, so non-ASCII text reaches it at fewer characters than maxLength suggests.`,
    },
  },
  required: ["source"],
};
const envelopeSchema = JSON.parse(readFileSync(resultSchemaPath, "utf8"));
const READ_ONLY_ANNOTATIONS = Object.freeze({
  readOnlyHint: true,
  destructiveHint: false,
  idempotentHint: true,
  openWorldHint: false,
});
const TOOLS = [
  {
    name: "compute",
    description: "Execute a bounded Ajisai program. Use for supported-domain exact rational, decimal, square-root and vector calculations, including reason-carrying NIL results.",
    inputSchema: sourceSchema,
    outputSchema: envelopeSchema,
    annotations: READ_ONLY_ANNOTATIONS,
  },
  {
    name: "check",
    description: "Parse and resolve Ajisai source without executing it; also verifies declared contracts conservatively.",
    inputSchema: sourceSchema,
    outputSchema: envelopeSchema,
    annotations: READ_ONLY_ANNOTATIONS,
  },
  {
    name: "infer_contracts",
    description: "Infer machine-readable contracts for user-defined Words without executing their bodies.",
    inputSchema: sourceSchema,
    outputSchema: envelopeSchema,
    annotations: READ_ONLY_ANNOTATIONS,
  },
  {
    name: "word_contract",
    description: "Return the generated canonical registry entry for a Word or alias. An unmatched name answers with the closest known Words in `suggestions`.",
    inputSchema: {
      type: "object",
      additionalProperties: false,
      properties: { word: { type: "string", minLength: 1 } },
      required: ["word"],
    },
    // The same envelope every other tool answers with. `matches` used to be
    // declared as an array of untyped objects on a tool whose entire purpose
    // is returning contracts, so a caller could not know the shape of the
    // answer without making the call first; the registry entry is now
    // described in `result.schema.json` beside everything else.
    outputSchema: envelopeSchema,
    annotations: READ_ONLY_ANNOTATIONS,
  },
];

let manifestCache;
let contractsCache;
let registryDigestCache;
let metadataCache;
let serverVersionCache;

/**
 * Admission control for concurrent executions, with a short waiting queue.
 *
 * `tryAcquire` keeps the non-blocking form the ceiling is defined in terms of;
 * `acquire` adds the queue, so a burst that clears within `waitMs` becomes
 * back-pressure rather than an error the caller has to retry around.
 */
export class ExecutionGate {
  #active = 0;
  #waiting = [];

  constructor(capacity) {
    this.capacity = capacity;
  }

  tryAcquire() {
    if (this.#active >= this.capacity) return false;
    this.#active += 1;
    return true;
  }

  /** Resolves `true` once a slot is held, or `false` if `waitMs` elapses. */
  acquire(waitMs) {
    if (this.tryAcquire()) return Promise.resolve(true);
    if (!(waitMs > 0)) return Promise.resolve(false);
    return new Promise((settle) => {
      const waiter = {
        settle: (acquired) => {
          clearTimeout(timer);
          settle(acquired);
        },
      };
      const timer = setTimeout(() => {
        const index = this.#waiting.indexOf(waiter);
        if (index !== -1) this.#waiting.splice(index, 1);
        settle(false);
      }, waitMs);
      this.#waiting.push(waiter);
    });
  }

  release() {
    if (this.#active === 0) throw new Error("execution gate released without a holder");
    const waiter = this.#waiting.shift();
    if (waiter) {
      // Hand the slot straight over: decrementing first would let a new
      // request overtake the caller that has been queued longest.
      waiter.settle(true);
      return;
    }
    this.#active -= 1;
  }
}
const executionGate = new ExecutionGate(LIMITS.concurrentExecutions);
function manifest() { return manifestCache ??= JSON.parse(readFileSync(manifestPath, "utf8")); }
function contracts() {
  return contractsCache ??= JSON.parse(readFileSync(contractsPath, "utf8"));
}
export function registryDigest() {
  const calculated = registryDigestCache ??= createHash("sha256")
    .update(readFileSync(contractsPath))
    .digest("hex");
  if (calculated !== metadata().registryDigest) {
    throw new HostError(
      "registryUnavailable",
      "The packaged Ajisai Word registry does not match its recorded digest.",
      { detail: `calculated=${calculated} expected=${metadata().registryDigest}` },
    );
  }
  return calculated;
}
function metadata() { return metadataCache ??= JSON.parse(readFileSync(metadataPath, "utf8")); }
export function engineVersion() { return metadata().engineVersion; }
export function serverVersion() {
  return serverVersionCache ??= JSON.parse(readFileSync(serverPackagePath, "utf8")).version;
}
export function packageEngines() {
  return JSON.parse(readFileSync(serverPackagePath, "utf8")).engines ?? {};
}

/**
 * Provenance every answer carries: which adapter, which engine, which
 * registry, which of the two interchangeable backends actually ran, and the
 * limits that were applied.
 *
 * `serverVersion` and `engineVersion` are two independently versioned
 * components — this adapter and the Ajisai language it speaks for — and a
 * saved result used to name only the second. Which adapter produced a stored
 * envelope decided whether a field was absent because the engine cannot
 * answer it or because this version of the server never sent it, and that
 * question was unanswerable from the result alone.
 *
 * The backend identifier does not weaken the abstraction the two backends
 * share — parity testing is what guarantees they agree. It is what makes a
 * disagreement investigable if one ever occurs.
 */
function provenance() {
  const selected = backend();
  return {
    serverVersion: serverVersion(),
    engineVersion: engineVersion(),
    registryDigest: registryDigest(),
    backend: { kind: selected?.kind ?? null },
    limits: LIMITS,
  };
}

function envelope(value) {
  return { content: [{ type: "text", text: JSON.stringify(value, null, 2) }], structuredContent: value };
}

/**
 * A host failure as a structured, schema-valid tool result.
 *
 * The text content stays — a client that only renders text still shows
 * something useful — but the machine-readable form is what a caller should
 * branch on. `mcp` provenance is attached when it can be: a registry failure
 * is precisely the case where it cannot.
 */
function fail(error, context = "tool call") {
  logHostError(error, context);
  const payload = {
    schemaVersion: 1,
    status: "hostError",
    error: {
      code: error.code,
      message: error.message,
      retryable: error.retryable,
      ...(error.limit ? { limit: error.limit } : {}),
      ...(error.retryAfterMs ? { retryAfterMs: error.retryAfterMs } : {}),
    },
  };
  try {
    payload.mcp = provenance();
  } catch {
    // Provenance itself is what failed; the error block above already says so.
  }
  return {
    content: [{ type: "text", text: JSON.stringify(payload, null, 2) }],
    structuredContent: payload,
    isError: true,
  };
}

async function runAgent(source, command) {
  if (typeof source !== "string" || source.length === 0) {
    return fail(new HostError("invalidRequest", "Provide non-empty `source` text."), command);
  }
  if (Buffer.byteLength(source, "utf8") > LIMITS.sourceBytes) {
    return fail(
      new HostError(
        "sourceTooLarge",
        `Source exceeds the ${LIMITS.sourceBytes}-byte limit (measured in UTF-8 bytes, not characters).`,
      ),
      command,
    );
  }
  const selected = backend();
  if (!selected) {
    return fail(
      new HostError(
        "backendUnavailable",
        "No Ajisai execution backend is available on this server.",
        { detail: "no native binary found (build rust/ or set AJISAI_BIN) and the packaged WASM module is missing" },
      ),
      command,
    );
  }
  if (!(await executionGate.acquire(CAPACITY_WAIT_MS))) {
    return fail(
      new HostError(
        "capacityExhausted",
        `All ${LIMITS.concurrentExecutions} execution slots are busy. Retry shortly.`,
        { retryAfterMs: CAPACITY_WAIT_MS },
      ),
      command,
    );
  }
  try {
    const operation = { run: "compute", check: "check", contract: "inferContracts" }[command];
    const result = await selected[operation](source);
    result.mcp = provenance();
    return envelope(result);
  } catch (error) {
    // A backend throw is always a host failure (timeout, spawn/worker
    // failure, an oversized or non-JSON response) — never a translated Ajisai
    // `ERROR`, which each backend already returns as a normal envelope above.
    return fail(HostError.from(error), command);
  } finally {
    executionGate.release();
  }
}

function wordContract(word) {
  const needle = typeof word === "string" ? word.trim().toUpperCase() : "";
  if (!needle) {
    return fail(new HostError("invalidRequest", "Provide a `word`."), "word_contract");
  }
  const entries = contracts().entries ?? [];
  const matches = entries.filter((entry) =>
    entry.name.toUpperCase() === needle ||
    entry.aliases.some((alias) => alias.toUpperCase() === needle)
  );
  let mcp;
  try {
    mcp = provenance();
  } catch (error) {
    return fail(HostError.from(error), "word_contract");
  }
  return envelope({
    schemaVersion: 1,
    status: "ok",
    registrySchemaVersion: contracts().schemaVersion,
    matches,
    // An empty result used to be a bare success — technically correct and
    // practically a dead end, since the caller most likely mistyped a name the
    // server holds the complete list of. The same edit-distance answer the
    // engine gives an unknown Word at runtime is available here for free.
    suggestions: matches.length === 0 ? suggestWords(needle, entries) : [],
    mcp,
  });
}

const RESOURCES = [
  { uri: "ajisai://guide/quickstart", name: "Ajisai agent quickstart", mimeType: "text/markdown" },
  { uri: "ajisai://vocabulary", name: "Ajisai generated Word vocabulary", mimeType: "application/json" },
  { uri: "ajisai://schema/result", name: "Ajisai MCP result contract", mimeType: "application/json" },
  { uri: "ajisai://limits", name: "Ajisai MCP host profile limits", mimeType: "application/json" },
];
const RESOURCE_TEMPLATES = [
  {
    uriTemplate: "ajisai://words/{name}",
    name: "Ajisai canonical Word contract",
    description: "The complete spec/words.json contract for a Word or alias.",
    mimeType: "application/json",
  },
];

/**
 * Validate everything the server promises to be able to answer, before it
 * accepts a single request.
 *
 * A corrupt packaged registry used to surface as a generic host error on
 * whichever request happened to touch it first. It is a startup fault: the
 * server cannot honour its own provenance claims, and the operator should
 * learn that when they start it.
 */
export function validateAssets() {
  manifest();
  contracts();
  metadata();
  serverVersion();
  registryDigest();
  readFileSync(skillPath, "utf8");
}

export function createServer() {
  validateAssets();
  // Fix the execution path for the lifetime of the process.
  backend();
  const server = new Server(
    { name: "ajisai", version: serverVersion() },
    { capabilities: { tools: {}, resources: {} } },
  );
  server.setRequestHandler(ListToolsRequestSchema, async () => ({ tools: TOOLS }));
  server.setRequestHandler(CallToolRequestSchema, async ({ params }) => {
    const args = params.arguments ?? {};
    if (params.name === "compute") return runAgent(args.source, "run");
    if (params.name === "check") return runAgent(args.source, "check");
    if (params.name === "infer_contracts") return runAgent(args.source, "contract");
    if (params.name === "word_contract") return wordContract(args.word);
    return fail(new HostError("unknownTool", `No such tool: ${params.name}`), "tools/call");
  });
  server.setRequestHandler(ListResourcesRequestSchema, async () => ({ resources: RESOURCES }));
  server.setRequestHandler(ListResourceTemplatesRequestSchema, async () => ({
    resourceTemplates: RESOURCE_TEMPLATES,
  }));
  server.setRequestHandler(ReadResourceRequestSchema, async ({ params }) => {
    const uri = params.uri;
    if (uri === "ajisai://guide/quickstart") return { contents: [{ uri, mimeType: "text/markdown", text: readFileSync(skillPath, "utf8") }] };
    if (uri === "ajisai://vocabulary") return { contents: [{ uri, mimeType: "application/json", text: JSON.stringify(manifest(), null, 2) }] };
    if (uri === "ajisai://schema/result") return { contents: [{ uri, mimeType: "application/json", text: readFileSync(resultSchemaPath, "utf8") }] };
    if (uri === "ajisai://limits") {
      return {
        contents: [{
          uri,
          mimeType: "application/json",
          text: JSON.stringify(
            {
              profile: "mcp-local-stdio",
              serverVersion: serverVersion(),
              engineVersion: engineVersion(),
              backend: { kind: backend()?.kind ?? null },
              limits: LIMITS,
              capacityWaitMs: CAPACITY_WAIT_MS,
            },
            null,
            2,
          ),
        }],
      };
    }
    if (uri.startsWith("ajisai://words/")) {
      const name = decodeURIComponent(uri.slice("ajisai://words/".length));
      const contract = wordContract(name);
      if (contract.isError) throw new Error(contract.structuredContent.error.message);
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

/**
 * Whether this module was the process entry point.
 *
 * `resolve()` normalizes a path; it does not follow symlinks. Every documented
 * way of launching this package by name — `npx ajisai-mcp-server`, a client
 * spawning the installed `ajisai-mcp-server` command — runs it through
 * `node_modules/.bin/ajisai-mcp-server`, a *symlink*, so `argv[1]` was the
 * link and `import.meta.url` was its target. The comparison was false, the
 * guard did not fire, and the process exited 0 having served nothing: a client
 * saw a server that started, offered no tools and reported no error. Only
 * naming `index.js` directly ever worked, which is why the self-test and the
 * pack smoke test — both of which import `createServer` rather than launch the
 * binary — never saw it. Comparing real paths is what makes the bin entry the
 * same entry point as the file.
 */
function isEntryPoint() {
  const entry = process.argv[1];
  if (!entry) return false;
  const self = fileURLToPath(import.meta.url);
  if (resolve(entry) === self) return true;
  try {
    return realpathSync(entry) === self;
  } catch {
    return false;
  }
}

if (isEntryPoint()) {
  // Any argument means a human at a terminal, not a client opening a session:
  // `--version`, `--doctor`, `--help`, or a mistake worth naming rather than
  // ignoring. The terminal commands live in `doctor.js` so the server path
  // never loads them, and they are never reached once a transport is open —
  // the stdout MCP frames travel over stays free of diagnostics.
  //
  // Deliberately not awaited: `doctor.js` imports back from this module, so
  // awaiting the import here would suspend this module's evaluation waiting
  // for a module that is waiting for this one. Continuing lets evaluation
  // finish first, which is what resolves the cycle.
  if (process.argv.length > 2) {
    import("./doctor.js")
      .then(async ({ runCli }) => process.exit(await runCli(process.argv.slice(2))))
      .catch((error) => {
        console.error(`[ajisai-mcp] ${error.detail ?? error.message}`);
        process.exit(1);
      });
  } else {
    try {
      await createServer().connect(new StdioServerTransport());
    } catch (error) {
      // Startup faults are the operator's to fix, and a stdio server that keeps
      // running while unable to answer is worse than one that does not start.
      console.error(`[ajisai-mcp] failed to start: ${error.detail ?? error.message}`);
      process.exit(1);
    }
  }
}
