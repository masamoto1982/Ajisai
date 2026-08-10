// Native-process backend: shells out to a built `ajisai` binary per call, the
// same one-process-per-call boundary the CLI's own `agent` command uses.
//
// Every method resolves to the parsed schema-1 JSON envelope on success —
// including a language-level `NIL` or `ERROR` result, which is still a
// *successful* MCP tool call. A thrown `Error` means a host failure (timeout,
// spawn failure, a non-JSON response) — the caller is responsible for turning
// that into MCP `isError`, never a translated Ajisai `ERROR`.

import { execFile } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

export class NativeCliBackend {
  constructor({ bin, wallTimeMs, responseBytes, executionSteps }) {
    this.bin = bin;
    this.wallTimeMs = wallTimeMs;
    this.responseBytes = responseBytes;
    this.executionSteps = executionSteps;
  }

  async #runAgent(source, operation, extraArgs) {
    let scratch;
    try {
      scratch = mkdtempSync(join(tmpdir(), "ajisai-mcp-"));
      const target = join(scratch, "program.ajisai");
      writeFileSync(target, source.endsWith("\n") ? source : `${source}\n`, { mode: 0o600 });
      const args = ["agent", operation, target, "--json", ...extraArgs];
      let stdout;
      try {
        ({ stdout } = await execFileAsync(this.bin, args, {
          encoding: "utf8",
          timeout: this.wallTimeMs,
          maxBuffer: this.responseBytes,
        }));
      } catch (error) {
        // Ajisai language errors intentionally exit 1 with a JSON diagnosis.
        // Timeouts, max-buffer failures and CLI usage failures are host
        // errors, even when the child managed to write a partial stdout
        // document.
        if (error.killed || error.signal) {
          throw new Error(`Ajisai exceeded ${this.wallTimeMs} ms`);
        }
        if (error.code !== 1 || !error.stdout) {
          throw new Error(`failed to run Ajisai: ${error.message}`);
        }
        stdout = error.stdout;
      }
      try {
        return JSON.parse(stdout);
      } catch {
        throw new Error("Ajisai returned a non-JSON response.");
      }
    } finally {
      if (scratch) rmSync(scratch, { recursive: true, force: true });
    }
  }

  compute(source) {
    return this.#runAgent(source, "compute", ["--step-limit", String(this.executionSteps)]);
  }

  check(source) {
    return this.#runAgent(source, "check", []);
  }

  inferContracts(source) {
    return this.#runAgent(source, "infer-contracts", []);
  }
}
