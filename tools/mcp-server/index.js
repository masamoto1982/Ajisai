#!/usr/bin/env node
// Ajisai MCP server — a thin wrapper over the `ajisai` CLI, the word
// manifest, and SKILL.md. It holds no language logic of its own: every
// answer is produced by running the CLI (Phase 1) or reading a generated
// artifact (SKILL.md from Phase 2, docs/word-manifest.json). See README.md.
//
// Tools:
//   run(source | file)  -> the CLI's `ajisai run --json` envelope
//   explain_word(word)  -> matching docs/word-manifest.json entries
//   skill()             -> the contents of SKILL.md

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// ── Locations (overridable by env) ───────────────────────────────────────
// Repo root defaults to three levels up from this file (tools/mcp-server/).
const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = process.env.AJISAI_REPO
  ? resolve(process.env.AJISAI_REPO)
  : resolve(here, "..", "..");
const skillPath = join(repoRoot, "SKILL.md");
const manifestPath = join(repoRoot, "docs", "word-manifest.json");

// The CLI binary: explicit AJISAI_BIN, else the repo debug/release build.
function resolveAjisaiBin() {
  if (process.env.AJISAI_BIN) return process.env.AJISAI_BIN;
  for (const profile of ["debug", "release"]) {
    const candidate = join(repoRoot, "rust", "target", profile, "ajisai");
    if (existsSync(candidate)) return candidate;
  }
  return null; // reported per-call so `explain_word`/`skill` still work
}

// ── Tool definitions (plain JSON Schema; no extra deps) ───────────────────
const TOOLS = [
  {
    name: "run",
    description:
      "Run an Ajisai program through the `ajisai` CLI and return its " +
      "--json envelope (status, stack, stackDisplay, output, diagnosis, " +
      "errorFlowTrace, aiDiagnostic, runtimeMetrics). Provide the program " +
      "as `source`, or a path with `file`.",
    inputSchema: {
      type: "object",
      properties: {
        source: { type: "string", description: "Ajisai source text to run." },
        file: { type: "string", description: "Path to a .ajisai file to run." },
      },
    },
  },
  {
    name: "explain_word",
    description:
      "Look a word up in the generated word manifest (docs/word-manifest.json) " +
      "and return its entries: surface, kind, category, module, canonical, and " +
      "semantic metadata. Matches the bare name or a MODULE@WORD form, " +
      "case-insensitively.",
    inputSchema: {
      type: "object",
      properties: {
        word: { type: "string", description: "Word name, e.g. MAP or MUSIC@PLAY." },
      },
      required: ["word"],
    },
  },
  {
    name: "skill",
    description:
      "Return SKILL.md — the generated, CLI-verified agent writing protocol " +
      "for Ajisai (run loop, syntax, control flow, NIL/UNKNOWN, examples, " +
      "common errors, forbidden patterns, and the full word quick reference).",
    inputSchema: { type: "object", properties: {} },
  },
];

// ── Tool handlers ─────────────────────────────────────────────────────────
function ok(text) {
  return { content: [{ type: "text", text }] };
}
function fail(text) {
  return { content: [{ type: "text", text }], isError: true };
}

function toolRun(args) {
  const bin = resolveAjisaiBin();
  if (!bin) {
    return fail(
      "ajisai CLI not found. Build it (`cargo build --bin ajisai` in rust/) " +
        "or set AJISAI_BIN to the binary path.",
    );
  }
  const source = args?.source;
  const file = args?.file;
  if ((source == null) === (file == null)) {
    return fail("Provide exactly one of `source` or `file`.");
  }

  let target = file ? resolve(file) : null;
  let scratch = null;
  if (source != null) {
    scratch = mkdtempSync(join(tmpdir(), "ajisai-mcp-"));
    target = join(scratch, "program.ajisai");
    writeFileSync(target, source.endsWith("\n") ? source : source + "\n");
  }
  try {
    const proc = spawnSync(bin, ["run", target, "--json"], { encoding: "utf8" });
    if (proc.error) return fail(`failed to run ajisai: ${proc.error.message}`);
    // The CLI prints exactly one JSON document to stdout (pipe-safe contract).
    // Pass it through verbatim; a non-zero exit (language error) still carries
    // a valid JSON envelope, so it is a successful tool call with status:error.
    const text = proc.stdout && proc.stdout.trim().length > 0
      ? proc.stdout
      : (proc.stderr || "(no output)");
    return ok(text);
  } finally {
    if (scratch) rmSync(scratch, { recursive: true, force: true });
  }
}

let manifestCache = null;
function loadManifest() {
  if (manifestCache) return manifestCache;
  manifestCache = JSON.parse(readFileSync(manifestPath, "utf8"));
  return manifestCache;
}

function toolExplainWord(args) {
  const word = (args?.word ?? "").trim();
  if (!word) return fail("Provide a `word` to explain.");
  let manifest;
  try {
    manifest = loadManifest();
  } catch (e) {
    return fail(`could not read ${manifestPath}: ${e.message}`);
  }
  const needle = word.toUpperCase();
  const matches = (manifest.entries || []).filter((e) => {
    const surface = (e.surface ?? "").toUpperCase();
    const short = (e.short_surface ?? "").toUpperCase();
    const canonical = (e.canonical ?? "").toUpperCase();
    return surface === needle || short === needle || canonical === needle;
  });
  if (matches.length === 0) {
    return fail(
      `No word matching '${word}' in the manifest. ` +
        "Use the `skill` tool's §9 quick reference to find the right name.",
    );
  }
  return ok(JSON.stringify(matches, null, 2));
}

function toolSkill() {
  try {
    return ok(readFileSync(skillPath, "utf8"));
  } catch (e) {
    return fail(
      `could not read ${skillPath}: ${e.message}. ` +
        "Generate it with `npm run generate:skill`.",
    );
  }
}

// ── Wire up the server ────────────────────────────────────────────────────
export function createServer() {
  const server = new Server(
    { name: "ajisai", version: "0.1.0" },
    { capabilities: { tools: {} } },
  );

  server.setRequestHandler(ListToolsRequestSchema, async () => ({ tools: TOOLS }));

  server.setRequestHandler(CallToolRequestSchema, async (request) => {
    const { name, arguments: args } = request.params;
    switch (name) {
      case "run":
        return toolRun(args);
      case "explain_word":
        return toolExplainWord(args);
      case "skill":
        return toolSkill();
      default:
        return fail(`unknown tool: ${name}`);
    }
  });

  return server;
}

// Auto-start over stdio only when invoked as the executable, so the module
// can be imported (e.g. by selftest.js) without grabbing stdin/stdout.
const invokedDirectly =
  process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (invokedDirectly) {
  const transport = new StdioServerTransport();
  await createServer().connect(transport);
}
