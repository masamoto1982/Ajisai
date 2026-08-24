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
  collectionWork: 20_000_000,
  bigintBits: 262_144,
  algebraicTerms: 512,
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
// `overrides` exists for one caller: the selftest's `wallTimeMs` case. That
// ceiling is a deadline the adapter holds around execution, not a budget the
// engine spends, so the only honest way to exercise it is to build a backend
// with a deadline short enough that a real program misses it — the server's own
// admission path, with one number moved. Every other caller passes nothing and
// gets the declared profile.
export function createBackend(overrides = {}) {
  const profile = { ...LIMITS, ...overrides };
  const bin = resolveAjisaiBin();
  if (bin) {
    return new NativeCliBackend({
      bin,
      wallTimeMs: profile.wallTimeMs,
      responseBytes: profile.responseBytes,
      executionSteps: profile.executionSteps,
    });
  }
  if (WasmWorkerBackend.isAvailable()) {
    return new WasmWorkerBackend({
      wallTimeMs: profile.wallTimeMs,
      executionSteps: profile.executionSteps,
      responseBytes: profile.responseBytes,
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
      // The four rules below are not a syntax summary — they are the four
      // mistakes a real model actually made, counted. After the entry-surface
      // round moved tool selection from 0.469 to 0.862, 31 of 130 prompts
      // reached `compute` and handed it source that did not run, and eight of
      // those were the quote character alone. Each rule was executed against
      // the engine before being written here, which corrected two guesses:
      // `[1 2 3]` without inner spaces is fine, and so is `[ 1, 2, 3 ]`.
      // Stating rules that are not real would cost the caller the same turn the
      // missing ones do.
      description:
        `Ajisai source text (file paths are not accepted). The effective limit is ${LIMITS.sourceBytes} UTF-8 bytes, so non-ASCII text reaches it at fewer characters than maxLength suggests. ` +
        "Syntax is postfix: operands first, then the Word — `1 2 ADD`, `[ 1 2 3 ] LENGTH`. " +
        "A string is single-quoted (`'hi'`, never \"hi\"). " +
        "A block passed to MAP/FILTER/FOLD/ANY/ALL is in braces (`[ 1 2 3 4 ] { 2 MOD 0 = } FILTER`), not brackets. " +
        "A Word's operand shape is part of its contract and is worth checking with word_contract when unsure — several take a vector where one number looks natural, e.g. `[ 0 4 ] RANGE` and `[ [ 1 2 ] [ 3 4 ] ] ZIP`.",
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
export const TOOLS = [
  {
    name: "compute",
    // Every word of this is load-bearing, and the length is deliberate: with
    // `tool_choice: auto` these four descriptions are the *only* text a caller
    // reads before deciding. The measured baseline (`eval/traces/`) says what
    // the previous one cost. It named the numeric domain and stopped there, so
    // of 118 prompts, 21 produced no call at all — twelve of them collection
    // work, which the description never mentioned — and another 46 were spent
    // guessing Word names in the registry (`vec-add`, `group-by`, `nil-or`,
    // `dict`: none exist) because nothing readable before the first call said
    // what the Words are called. Naming the families and the Words in them is
    // the cheapest thing that answers both.
    // A third failure mode showed up only in Japanese. Live evaluation
    // against claude-opus-5 (eval/traces/claude-opus-5-full-corpus.json)
    // found three negative cases — irrelevant-currency, irrelevant-estimate,
    // irrelevant-debug, none of them Ajisai's domain — that passed restraint
    // in English but failed in Japanese: the model invented a plausible
    // real-world number (an exchange rate, characters-per-page, a step count
    // for someone else's for-loop) and ran it through this tool, so the
    // guess came back dressed as an exact result. The description at the
    // time was identical in both languages, and irrelevantToolRate was 0.10
    // en vs. 0.25 ja on that same text, so the gap was the model's response
    // to one weak sentence differing by language, not missing Japanese
    // wording. The closing sentence below names the failure directly instead
    // of leaving "out of domain" to be inferred.
    description:
      "Execute a bounded Ajisai program and return its stack. Ajisai is postfix (RPN) and its numbers are exact rationals closed under square root — no floats, so results are reproducible and comparisons decide. " +
      "Its 65 Words cover arithmetic (ADD SUB MUL DIV MOD FLOOR ROUND ABS NEG MIN MAX SQRT SUM), comparison (EQ NEQ LT LTE GT GTE), boolean logic (AND OR NOT), " +
      "vectors — arithmetic broadcasts element-wise — collections (SORT ORDER UNIQUE TALLY GROUP ZIP RANGE FILL TAKE CONCAT REVERSE LENGTH GET PUT INDEX-OF), " +
      "higher-order blocks (MAP FILTER FOLD ANY ALL), text (CHARS JOIN TOKENIZE TRIM NUM STR), and absence (NIL NIL? VENT), plus DEF to name your own. " +
      "Word names are exact and case-sensitive; the full list is the ajisai://vocabulary resource and word_contract answers a near-miss with suggestions, so look a name up rather than guessing it. " +
      "Reach for this whenever the request is one of those operations and the answer should be exact and checkable rather than recalled. Out of domain: transcendentals, floats, I/O, and general-purpose programming. " +
      "Ajisai also has no external or real-world reference data of its own — no exchange rates, no calendars, no reading speeds, no other language's syntax semantics. Do not invent a plausible-looking number for one of those and run it through this tool to dress a guess up as an exact answer; if the question needs a real-world fact rather than a value already given or derivable from first principles inside this domain, answer directly without a call, or say you don't know.",
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
    // The baseline's second failure mode was 46 turns spent here, two guesses
    // at a time, at names no Ajisai vocabulary ever had. Answering a miss with
    // `suggestions` was already right; what was missing was any hint that the
    // whole list is one resource read away, so a caller that does not know the
    // name has something better to do than guess again.
    description:
      "Return the generated canonical registry entry for a Word or alias — its arity, purity, NIL policy, contract, " +
      "and `cost`: what the Word charges on each metered resource (`steps`/`numeric`/`collection`), as a growth class " +
      "in its input. Cost classes join pointwise under concatenation, so a phrase's bound is the widest bound among " +
      "its Words — read them here to budget a program before running it rather than discovering the ceiling by hitting it. " +
      "An unmatched name answers with the closest known Words in `suggestions`. " +
      "To read every Word's full contract at once instead of probing one name at a time, read the ajisai://contracts " +
      "resource; ajisai://vocabulary lists the inventory and its semantic classification.",
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

/**
 * Drop the envelope fields whose value is `null`.
 *
 * The backend fills every slot of the schema-1 envelope on every answer, so a
 * successful `compute` used to advertise `message: null`, `diagnosis: null`,
 * `aiDiagnostic: null` and `contractDecls: null` — four diagnostic-sounding
 * fields inviting a reader to look at nothing. Absence says the same thing in
 * no bytes, and `result.schema.json` requires only `schemaVersion` and
 * `status`, so nothing that was promised stops being there.
 *
 * Top level only, and deliberately so. Recursing would save a further ~4% on
 * the largest diagnosis and would turn one stated rule into a transformation a
 * reader has to apply mentally to every nested object; the rule is worth more
 * than the bytes. The backend's own envelope is untouched — this is the
 * adapter's presentation of it, which is why the native/WASM parity test,
 * which compares what the backends return, is unaffected.
 */
function withoutEmptyFields(value) {
  return Object.fromEntries(Object.entries(value).filter(([, field]) => field !== null));
}

/**
 * One result, in the two shapes MCP asks for.
 *
 * `structuredContent` is what a caller branches on. `content` carries the
 * *same* object serialized, because the specification asks a tool with an
 * output schema to also return the serialized JSON in a text block for
 * backwards compatibility — a text-only client has no other way to reach any
 * of this. Replacing that text with a prose summary would have been the one
 * compaction that actually loses information, so what shrank instead is
 * padding: the two-space indentation this used to be written with cost about a
 * third of the text and told a machine nothing.
 *
 * Both shapes are built from one object, so the mirror cannot drift.
 */
function envelope(value) {
  const result = withoutEmptyFields(value);
  return { content: [{ type: "text", text: JSON.stringify(result) }], structuredContent: result };
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
    content: [{ type: "text", text: JSON.stringify(payload) }],
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
  // The whole contract registry in one read. `word_contract` answers one name
  // at a time, which is the wrong shape for the question cost exists to answer:
  // bounding a phrase means joining the bounds of every Word in it, and a
  // caller cannot do that from 65 separate probes.
  { uri: "ajisai://contracts", name: "Ajisai canonical Word contracts", mimeType: "application/json" },
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
    if (uri === "ajisai://contracts") return { contents: [{ uri, mimeType: "application/json", text: JSON.stringify(contracts(), null, 2) }] };
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
