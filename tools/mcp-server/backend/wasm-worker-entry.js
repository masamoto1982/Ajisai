// Runs inside the worker_threads Worker spawned by `WasmWorkerBackend`. Off
// the stdio server's main thread by construction — the parent never imports
// the WASM module or awaits it synchronously (see wasm-worker.js).
//
// One worker per call, mirroring the native backend's one-process-per-call
// model: `wasmModulePath` is passed fresh each time, and the module itself
// runs one `agent_*` operation before the worker is torn down.
//
// The response-size ceiling is checked here, on the JSON *text* the WASM entry
// point returns — the same bytes the native backend's `execFile` measures on
// stdout, so both backends reject at the same threshold on the same units.
// Checking it here also means an oversized envelope is never structured-cloned
// across the thread boundary.

import { parentPort, workerData } from "node:worker_threads";

async function main() {
  const { op, source, stepLimit, wasmModulePath, responseBytes } = workerData;
  const wasm = await import(wasmModulePath);
  let json;
  if (op === "compute") {
    json = await wasm.agent_compute(source, stepLimit ?? undefined);
  } else if (op === "check") {
    json = wasm.agent_check(source);
  } else if (op === "inferContracts") {
    json = wasm.agent_infer_contracts(source);
  } else {
    parentPort.postMessage({
      error: {
        code: "backendFailure",
        message: "The Ajisai backend failed to produce a result.",
        detail: `unknown wasm-worker operation: ${op}`,
      },
    });
    return;
  }
  if (typeof responseBytes === "number" && Buffer.byteLength(json, "utf8") > responseBytes) {
    parentPort.postMessage({
      error: {
        code: "responseTooLarge",
        message: `The result exceeds the ${responseBytes}-byte response limit. Reduce the size of the value left on the stack.`,
      },
    });
    return;
  }
  parentPort.postMessage({ envelope: JSON.parse(json) });
}

main().catch((error) => {
  parentPort.postMessage({
    error: {
      code: "backendFailure",
      message: "The Ajisai backend failed to produce a result.",
      detail: error?.message ?? String(error),
    },
  });
});
