// Runs inside the worker_threads Worker spawned by `WasmWorkerBackend`. Off
// the stdio server's main thread by construction — the parent never imports
// the WASM module or awaits it synchronously (see wasm-worker.js).
//
// One worker per call, mirroring the native backend's one-process-per-call
// model: `wasmModulePath` is passed fresh each time, and the module itself
// runs one `agent_*` operation before the worker is torn down.

import { parentPort, workerData } from "node:worker_threads";

async function main() {
  const { op, source, stepLimit, wasmModulePath } = workerData;
  const wasm = await import(wasmModulePath);
  let json;
  if (op === "compute") {
    json = await wasm.agent_compute(source, stepLimit ?? undefined);
  } else if (op === "check") {
    json = wasm.agent_check(source);
  } else if (op === "inferContracts") {
    json = wasm.agent_infer_contracts(source);
  } else {
    throw new Error(`unknown wasm-worker operation: ${op}`);
  }
  parentPort.postMessage({ envelope: JSON.parse(json) });
}

main().catch((error) => {
  parentPort.postMessage({ error: error?.message ?? String(error) });
});
