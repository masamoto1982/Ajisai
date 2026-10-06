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

// The module is loaded as soon as the Worker starts, before the operation
// arrives: the backend starts the next call's Worker ahead of time, so the
// load is off the call's critical path. A load failure is reported to the
// operation, as it was when the load happened after it arrived.
//
// It also runs one fixed warm-up program first. The first operation in a
// fresh module pays for compiling the code it reaches and for the engine's
// one-time tables (~10 ms against ~0.5 ms for the same operation again); the
// warm-up pays that instead, ahead of the call. It cannot change what the
// call answers: every operation builds a fresh interpreter, and what the
// warm-up leaves behind is the process-wide immutable tables (`OnceLock`s:
// the Core Word registry, builtin specs, outcome vocabulary) and a
// diagnostic mint counter that is only ever read as a difference within one
// run (`semantic::minted_absence_count`).
const WARM_UP = "[ 1 2 3 ] [ 2 MUL 1 ADD ] MAP 'X' BIND X LENGTH 1/3 ADD";
const loaded = import(workerData.wasmModulePath).then(
  async (wasm) => {
    await wasm.agent_compute(WARM_UP, undefined);
    return { wasm };
  },
  (error) => ({ error }),
);

async function main({ op, source, stepLimit, responseBytes }) {
  const { wasm, error } = await loaded;
  if (error) throw error;
  let json;
  if (op === "compute") {
    json = await wasm.agent_compute(source, stepLimit ?? undefined);
  } else if (op === "check") {
    json = wasm.agent_check(source);
  } else if (op === "inferContracts") {
    json = wasm.agent_infer_contracts(source);
  } else if (op === "outcomes") {
    json = wasm.agent_predict_outcomes(source, stepLimit ?? undefined);
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

parentPort.once("message", (job) => main(job).catch((error) => {
  parentPort.postMessage({
    error: {
      code: "backendFailure",
      message: "The Ajisai backend failed to produce a result.",
      detail: error?.message ?? String(error),
    },
  });
}));
