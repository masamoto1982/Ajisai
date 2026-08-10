#!/usr/bin/env node
import { execFile } from "node:child_process";
import { mkdirSync, mkdtempSync, renameSync, rmSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { promisify } from "node:util";
import { pathToFileURL, fileURLToPath } from "node:url";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";

const execFileAsync = promisify(execFile);
const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..", "..");
const scratch = mkdtempSync(join(tmpdir(), "ajisai-mcp-pack-"));
let tarball;
try {
  const { stdout } = await execFileAsync("npm", ["pack", "--json"], {
    cwd: here,
    encoding: "utf8",
  });
  const packed = JSON.parse(stdout)[0];
  tarball = join(here, packed.filename);
  const paths = new Set(packed.files.map(({ path }) => path));
  for (const required of [
    "index.js", "result.schema.json", "assets/metadata.json", "assets/words.json",
    "assets/word-manifest.json", "assets/quickstart.md", "eval/cases.json", "score-repairs.js",
    "eval/repair-cases.json", "backend/native-cli.js", "backend/wasm-worker.js",
    "backend/wasm-worker-entry.js", "wasm/generated/ajisai_core.js", "wasm/generated/ajisai_core_bg.wasm",
    "wasm/generated/package.json",
  ]) {
    if (!paths.has(required)) throw new Error(`tarball is missing ${required}`);
  }

  try {
    await execFileAsync(
      "npm",
      ["install", "--ignore-scripts", "--no-audit", "--no-fund", "--prefix", scratch, tarball],
      { encoding: "utf8" },
    );
  } catch (error) {
    if (!/E403|403 Forbidden|ENOTCACHED/.test(error.stderr ?? "")) throw error;
    // Restricted development sandboxes may forbid registry reads. Still test
    // the exact tarball contents, borrowing only the already lockfile-installed
    // dependencies. CI has registry access and takes the clean-install path.
    const unpack = join(scratch, "unpack");
    mkdirSync(unpack, { recursive: true });
    await execFileAsync("tar", ["-xzf", tarball, "-C", unpack]);
    const modules = join(scratch, "node_modules");
    mkdirSync(modules, { recursive: true });
    const installed = join(modules, "ajisai-mcp-server");
    renameSync(join(unpack, "package"), installed);
    symlinkSync(join(here, "node_modules"), join(installed, "node_modules"), "dir");
    console.warn("WARN registry unavailable; used lockfile-installed dependencies");
  }
  const installedEntry = join(scratch, "node_modules", "ajisai-mcp-server", "index.js");
  const installed = await import(pathToFileURL(installedEntry));

  async function withServer(run) {
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    const scenarioServer = installed.createServer();
    const scenarioClient = new Client({ name: "pack-smoke", version: "1" });
    await Promise.all([
      scenarioServer.connect(serverTransport),
      scenarioClient.connect(clientTransport),
    ]);
    try {
      await run(scenarioClient);
    } finally {
      await scenarioClient.close();
      await scenarioServer.close();
    }
  }

  // Scenario 1: neither AJISAI_REPO nor AJISAI_BIN is set — the true
  // zero-configuration default. The installed tarball must still expose its
  // packaged static resources (never read from this checkout: `here` inside
  // the installed copy resolves under `scratch`, nowhere near this repo) and
  // compute through its own packaged, self-contained WASM worker backend,
  // with no native binary and no checkout in reach.
  delete process.env.AJISAI_REPO;
  delete process.env.AJISAI_BIN;
  await withServer(async (client) => {
    const tools = await client.listTools();
    if (tools.tools.length !== 4) throw new Error("installed package did not expose four tools");
    const guide = await client.readResource({ uri: "ajisai://guide/quickstart" });
    if (!guide.contents[0]?.text?.includes("Ajisai")) {
      throw new Error("installed package did not expose its packaged guide");
    }
    const contract = await client.callTool({ name: "word_contract", arguments: { word: "MAP" } });
    if (contract.structuredContent?.matches?.[0]?.name !== "MAP") {
      throw new Error("installed package did not expose its packaged Word registry");
    }
    const computed = await client.callTool({ name: "compute", arguments: { source: "1 3 /" } });
    if (computed.structuredContent?.stackDisplay?.[0] !== "1/3") {
      throw new Error("installed package could not compute with neither AJISAI_REPO nor AJISAI_BIN set (WASM backend)");
    }
  });
  console.log(`PASS clean-installed ${packed.filename} computes with no repository and no native binary (WASM backend)`);

  // Scenario 2: AJISAI_BIN is an explicit override (the native/Docker
  // deployment story). The installed tarball must still prefer and use the
  // native backend when pointed at one.
  process.env.AJISAI_BIN = join(repoRoot, "rust", "target", "debug", "ajisai");
  await withServer(async (client) => {
    const computed = await client.callTool({ name: "compute", arguments: { source: "1 3 /" } });
    if (computed.structuredContent?.stackDisplay?.[0] !== "1/3") {
      throw new Error("installed package could not compute through the native AJISAI_BIN backend");
    }
  });
  console.log(`PASS clean-installed ${packed.filename} computes through an explicit AJISAI_BIN (native backend, ${packed.files.length} files)`);
} finally {
  if (tarball) rmSync(tarball, { force: true });
  rmSync(scratch, { recursive: true, force: true });
}
