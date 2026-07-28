// The session, in a worker.
//
// The interpreter lives here so that a long-running program never freezes the
// interface, and so that Escape can abort by terminating the worker outright.
// The main thread keeps a journal of fragments that ran, and replays it into a
// fresh worker afterwards — which is why abort can be brutal and still leave
// the session where it was.

const wasmUrl = new URL('./ajisai.wasm', import.meta.url);

const encoder = new TextEncoder();
const decoder = new TextDecoder();

let core = null;

const load = async () => {
  if (core) return core;
  const response = await fetch(wasmUrl);
  const { instance } = await WebAssembly.instantiate(await response.arrayBuffer(), {});
  core = instance.exports;
  return core;
};

/** Read the reply buffer the last call left behind. */
const takeReply = (length) =>
  JSON.parse(decoder.decode(new Uint8Array(core.memory.buffer, core.ajisai_reply(), length)));

/** Call an entry point that takes source. */
const withSource = (entry, source) => {
  const bytes = encoder.encode(source ?? '');
  const pointer = core.ajisai_alloc(bytes.length);
  try {
    if (bytes.length > 0) {
      new Uint8Array(core.memory.buffer, pointer, bytes.length).set(bytes);
    }
    return takeReply(entry(pointer, bytes.length));
  } finally {
    core.ajisai_free(pointer, bytes.length);
  }
};

const handlers = {
  execute: (source) => withSource(core.ajisai_execute, source),
  lint: (source) => withSource(core.ajisai_lint, source),
  format: (source) => withSource(core.ajisai_format, source),
  steps: (source) => withSource(core.ajisai_steps, source),
  snapshot: () => takeReply(core.ajisai_snapshot()),
  reset: () => takeReply(core.ajisai_reset()),
  vocabulary: () => takeReply(core.ajisai_vocabulary()),
  // Rebuild a session by re-running what already ran. Used after an abort,
  // where the worker was terminated mid-program.
  replay: (_source, journal) => {
    core.ajisai_reset();
    let reply = takeReply(core.ajisai_snapshot());
    for (const fragment of journal ?? []) {
      reply = withSource(core.ajisai_execute, fragment);
    }
    return reply;
  },
};

self.onmessage = async (event) => {
  const { id, kind, source, journal } = event.data;
  try {
    await load();
    const handler = handlers[kind];
    if (!handler) throw new Error(`unknown request: ${kind}`);
    self.postMessage({ id, ok: true, reply: handler(source, journal) });
  } catch (error) {
    self.postMessage({ id, ok: false, error: String(error?.message ?? error) });
  }
};
