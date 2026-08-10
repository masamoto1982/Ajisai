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
let client;
let server;
try {
  const { stdout } = await execFileAsync("npm", ["pack", "--json"], {
    cwd: here,
    encoding: "utf8",
  });
  const packed = JSON.parse(stdout)[0];
  tarball = join(here, packed.filename);
  const paths = new Set(packed.files.map(({ path }) => path));
  for (const required of ["index.js", "result.schema.json", "eval/cases.json", "score-repairs.js", "eval/repair-cases.json"]) {
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
  process.env.AJISAI_REPO = repoRoot;
  process.env.AJISAI_BIN = join(repoRoot, "rust", "target", "debug", "ajisai");
  const installedEntry = join(scratch, "node_modules", "ajisai-mcp-server", "index.js");
  const installed = await import(pathToFileURL(installedEntry));
  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  server = installed.createServer();
  client = new Client({ name: "pack-smoke", version: "1" });
  await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);

  const tools = await client.listTools();
  if (tools.tools.length !== 4) throw new Error("installed package did not expose four tools");
  const computed = await client.callTool({
    name: "compute",
    arguments: { source: "1 3 /" },
  });
  if (computed.structuredContent?.stackDisplay?.[0] !== "1/3") {
    throw new Error("installed package could not compute through the real backend");
  }
  console.log(`PASS clean-installed ${packed.filename} (${packed.files.length} files)`);
} finally {
  if (client) await client.close();
  if (server) await server.close();
  if (tarball) rmSync(tarball, { force: true });
  rmSync(scratch, { recursive: true, force: true });
}
