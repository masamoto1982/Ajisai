# Web Serial (`SERIAL` module) — Design Note

Status: **Non-canonical** (developer note).
Authority: this document does not define Ajisai semantics. The canonical
source is `SPECIFICATION.md`. If anything here conflicts with the
specification, the specification wins. Nothing in this note is binding
until the relevant parts are promoted into `SPECIFICATION.md`.

## Motivation

Web Serial (W3C Serial API) lets a browser talk to physical serial
devices (microcontrollers, sensors, instruments) over USB/CDC and the
like. With a second major browser engine adding support, exposing serial
I/O from Ajisai becomes worthwhile: it turns the playground into a way to
drive real hardware from an exact-real dataflow language.

The implementation must **not** be tied to one browser. It is gated on
runtime capability detection (`'serial' in navigator`) and degrades
gracefully where the API is absent.

## Two delivery targets, one core

Ajisai ships in two channels and both must be served by a *single*
canonical interpreter (per `SPECIFICATION.md` §14.1, no dual-mode drift):

- **Web playground** — Vite build, interpreter compiled to WASM, executed
  in a Web Worker, GUI in the main thread.
- **Tauri desktop** (current/near-future) — the *same* frontend and the
  *same* WASM interpreter run inside the Tauri WebView; native
  capabilities are reached through TypeScript adapters that call Tauri
  commands (`invoke`), exactly as `tauri-plugin-fs` / `-dialog` /
  `-store` are reached today.

Consequently the interpreter core is identical in both channels, and the
only place that differs is the platform adapter.

## Core principle: the core never knows about serial

The Rust core does not reference `navigator.serial` or any native
`serialport` crate. It does exactly two things:

1. **Outbox** — effectful serial words emit a single command line into
   `interp.output_buffer`, prefixed `SERIAL:` and carrying a JSON
   payload. This mirrors how the `MUSIC` module emits `AUDIO:` commands
   (`rust/src/interpreter/audio/build_audio_structure.rs`).
2. **Inbox** — received bytes are made available to a run by being
   injected into the interpreter state at *execution start*; a read word
   pulls from that injected inbox.

Because of this, the same WASM binary and the same language semantics
serve both the web and Tauri channels. Platform differences are confined
to the adapter described below.

### Why not native synchronous reads under Tauri

Even under Tauri the interpreter runs as WASM in the WebView, and adapter
calls are asynchronous (`invoke` returns a Promise). The interpreter's
execution loop is synchronous Rust; it cannot `await` mid-execution
without a major rearchitecture that would also break the step-budget
determinism model and child-runtime isolation. Adding a Tauri-only
synchronous-read path would be precisely the dual-mode drift that
§14.1 forbids. The inbox/outbox model is therefore the single canonical
mechanism for both channels.

## The effect bridge (existing precedent)

Tracing the audio path that this design reuses:

1. Rust word writes `AUDIO:{json}\n` to `interp.output_buffer`
   (`build_audio_structure.rs:287`). The core never touches the browser.
2. The WASM boundary returns `output_buffer` as a string after execution
   (`wasm_interpreter_bindings/wasm_interpreter_execution.rs`).
3. TypeScript splits the output by line prefix
   (`AUDIO:` / `CONFIG:` / `EFFECT:` / `JSONEXPORT:`) in
   `src/gui/output-display-renderer.ts:307`.
4. The main thread dispatches to the host API (`src/audio/audio-engine.ts`).

Key property: this path is **one-way and fire-and-forget**. Execution
runs in a Web Worker (`src/workers/interpreter-execution-worker.ts`);
side effects run on the main thread *after* the worker returns. There is
no channel for the host to hand a value back into a running execution.
Serial writes fit this exactly; serial reads do not, which is what the
inbox model resolves.

## Inbox / outbox model

### Outbox (OPEN / CONFIGURE / WRITE / CLOSE / FLUSH)

Fire-and-forget, identical in shape to audio:

```
SERIAL:{"op":"open","portId":"p1"}
SERIAL:{"op":"configure","portId":"p1","baudRate":115200}
SERIAL:{"op":"write","portId":"p1","bytes":[72,73]}
SERIAL:{"op":"close","portId":"p1"}
```

The main-thread `Serial` adapter consumes these and drives the real port.

### Inbox (READ)

The main-thread adapter continuously reads each open port into a per-port
RX buffer. At the **start of each execution**, the bytes accumulated since
the previous run are injected as part of the
`InterpreterStateSnapshot` (`src/platform/platform-adapter.ts`). `READ`
pops from this injected inbox.

The result is an **event-poll model**: a run sees the data that arrived
since the last run. There is no blocking read. Within a single execution
the inbox is fixed, so a run remains deterministic with respect to its
inputs — consistent with the step budget, child-runtime isolation, and
hedged/elastic re-execution (`rust/src/elastic/`).

### Connection lifetime and handles

The open port lives in the main-thread adapter, keyed by an opaque
`portId`. The runtime holds only that id (an opaque handle value). Because
the port is owned by the main thread, not the worker, the connection
survives across executions and across worker recycling in both channels.

### User-gesture and security constraints (web only)

- `navigator.serial.requestPort()` requires a user gesture and must be
  called synchronously within a click handler. The worker round-trip
  loses the gesture, so port *selection* is driven by a dedicated
  "Connect serial port" UI control, not by a `SERIAL:` command emitted
  from a program run. Once a port is granted, programs reference it by id.
- Web Serial requires a secure context (HTTPS or localhost). GitHub Pages
  qualifies.
- Under Tauri these constraints do not apply; the abstract interface is
  designed for the stricter (web) case so the Tauri backend satisfies it
  trivially.

## Platform adapter: one interface, two backends

A new abstract `Serial` member is added to `PlatformAdapter`. It is
defined for the stricter web case so Tauri satisfies it without special
casing.

```ts
export interface SerialPortInfo {
    readonly portId: string;
    readonly label?: string;
}

export interface SerialAdapter {
    readonly available: boolean;            // capability detection
    requestAccess(): Promise<SerialPortInfo | null>;   // web: requestPort under gesture
    listPorts(): Promise<SerialPortInfo[]>;
    open(portId: string): Promise<void>;
    configure(portId: string, options: { baudRate: number }): Promise<void>;
    write(portId: string, bytes: Uint8Array): Promise<void>;
    drainInbox(portId: string): Uint8Array;  // bytes since last call (for snapshot injection)
    close(portId: string): Promise<void>;
}
```

| Concern | Web backend | Tauri backend |
|---------|-------------|---------------|
| Implementation | `WebSerialAdapter` → `navigator.serial` | `TauriSerialAdapter` → `invoke('serial_*')` |
| Native side | n/a | `serialport` crate + commands in `src-tauri` |
| Gesture | `requestPort()` in click handler | not required |
| Secure context | HTTPS/localhost | not required |
| RX read | `ReadableStream` reader loop on main thread | native read thread → Tauri event → buffer |
| Capability | `'serial' in navigator` | always true |

The interface ships with the web backend first. The Tauri backend is a
typed stub initially (so the contract is fixed) and is filled in during
Phase 3 without touching the core or the language semantics.

## Vocabulary (canonical English names)

Module `SERIAL`. Action-object names per `SPECIFICATION.md` §8.5.

| Word | Stack effect (sketch) | Direction |
|------|-----------------------|-----------|
| `LIST-PORTS` | `-- ports` | query |
| `OPEN` | `port-id -- handle` | outbox |
| `CONFIGURE` | `handle options -- handle` | outbox |
| `WRITE` | `handle bytes -- ` | outbox |
| `READ` | `handle -- bytes` | inbox |
| `FLUSH` | `handle -- ` | outbox |
| `CLOSE` | `handle -- ` | outbox |

Exact stack effects, byte/vector encoding, and option-record shape are
finalized when promoted to `SPECIFICATION.md`.

## Contract metadata (per §7.14)

Every word gets a Coreword contract entry:

- `purity = Effectful`, `deterministic = false`, `safe_preview = false`.
- `safety_level = D` (effectful; external state). Serial words are
  **excluded from speculative / hedged execution**
  (`rust/src/elastic/purity_table.rs`) and are not eligible for
  self-host preview.
- `nil_policy` / `partiality` per word.
- `canonical_home = Module("SERIAL")`, listed under the `SERIAL` module.

## Bubble Rule (per §11.2)

"Could not produce a value → bubble; misuse → error."

- `READ` with no buffered data → Bubble/NIL, `reason = noData`.
- Port dropped/disconnected during use → Bubble/NIL,
  `reason = portDisconnected`.
- `WRITE`/`READ` against a non-handle, or malformed options → ordinary
  error (`StructureError`).

`noData` and `portDisconnected` are new protocol strings to be registered
when promoted to the specification.

## Specification changes required (Phase 0 → promotion)

- §9.1: add `SERIAL` to the module table.
- §7.x: register the `SERIAL` vocabulary with English canonical names.
- §7.14: contract entries (`partiality`, `nil_policy`, `safety_level`).
- §11.2: add `noData` / `portDisconnected` bubble cases and protocol
  strings.
- Confirm hedged/elastic execution never speculatively runs serial words.

## Phased plan

- **Phase 0** — this note; then promote the agreed parts into
  `SPECIFICATION.md`; fix the abstract `Serial` interface (web + Tauri).
- **Phase 1** — Web send MVP: Rust `SERIAL` module (emit only:
  `LIST-PORTS`/`OPEN`/`CONFIGURE`/`WRITE`/`CLOSE`/`FLUSH`), module
  registration + contract metadata; `WebSerialAdapter`; `SERIAL:` line
  dispatch in the output renderer; a "Connect serial port" UI control;
  a typed `TauriSerialAdapter` stub. Tests cover command emission and
  contracts (no real hardware in CI).
- **Phase 2** — Inbox: RX buffering on the main thread, snapshot
  injection, `READ`, bubble reasons, tests; semantics shared by both
  channels.
- **Phase 3** — Tauri backend: add the `serialport` crate and
  `serial_*` commands in `src-tauri`, implement `TauriSerialAdapter`. The
  web channel is unchanged.

## Risks / constraints

- Ajisai's identity (exact-real, deterministic, mechanically verifiable)
  is in tension with serial I/O (non-deterministic, effectful, stateful).
  The inbox model preserves *within-run* determinism, and `safety_level`
  `D` plus exclusion from speculative execution keep the effect isolated.
- Hardware cannot be exercised in CI. Tests assert command emission and
  contract metadata (as `audio_effect_tests.rs` does) and exercise inbox
  injection at the state-snapshot level; the adapter is mocked.
- Web Serial requires a secure context and is browser-gated; the feature
  is capability-detected and the playground remains fully usable without
  it.
