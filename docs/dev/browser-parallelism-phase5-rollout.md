# Browser Parallelism (Phase 5) — Rollout

Status: **groundwork landed; threaded-wasm flip pending a build-capable
environment.**

This note tracks the implicit-parallelism roadmap's Phase 5 (`docs/dev/
implicit-parallelism-roadmap.md` §7, Phase 5): make the data-parallel kernels
run across cores **in the browser** via `wasm-bindgen-rayon` +
SharedArrayBuffer, with a transparent single-threaded fallback.

## Authority

Non-canonical. `SPECIFICATION.html` is the only authority for semantics. This
note describes *how to make the browser go faster*, never *what is true*. Phase
5 must not change any observable behavior (Same Result); it only changes speed.

## What is in place (this change)

All of this is verifiable without a wasm toolchain and ships inert — it does
**not** change the deployed site's behavior:

- **Capability detection** — `src/platform/cross-origin-isolation.ts`:
  `detectParallelCapability()` is the single source of truth for "can this page
  thread?" (`crossOriginIsolated && typeof SharedArrayBuffer !== 'undefined'`),
  plus `recommendedThreads` (= `hardwareConcurrency`, 1 on fallback). Unit-
  tested in `cross-origin-isolation.test.ts`.
- **Dev/preview isolation headers** — `vite.config.ts` sets
  `Cross-Origin-Opener-Policy: same-origin` and
  `Cross-Origin-Embedder-Policy: require-corp` on the dev and preview servers
  (web target only), so `crossOriginIsolated` is true under `npm run dev`. This
  is dev-only; `vite build` emits static files and bakes in no headers.
- **GitHub Pages fallback** — `public/coi-serviceworker.js` re-emits responses
  with the isolation headers (static hosts can't set them). Registered by
  `src/platform/register-cross-origin-isolation.ts` — **opt-in**, not yet wired
  into the bootstrap, so the live site is untouched.
- **Observability** — `WorkerManager` logs the detected capability at init. The
  pool still uses snapshot-copying Web Workers; nothing dispatches wasm threads
  yet.

## What remains (the flip) — requires a build-capable environment

The agent environment used to land the groundwork has **no nightly toolchain,
no `rust-src`, no `wasm-pack`, no `wasm32` target, and no cross-origin-isolated
browser**, so it cannot build or browser-verify a threaded wasm. The following
must be done where it can be built and tested in a real browser:

1. **Toolchain**: `rustup toolchain install nightly`, `rustup component add
   rust-src --toolchain nightly`, `rustup target add wasm32-unknown-unknown`.
2. **Rust deps**: add `wasm-bindgen-rayon` (under the `wasm` feature) and
   `rayon`. Export a thread-pool initializer (`initThreadPool(n)`).
3. **Parallel kernels on wasm**: today `interpreter/parallel.rs` fans out with a
   `std::thread` pool gated `#[cfg(not(target_arch = "wasm32"))]` and falls back
   to sequential on wasm. Provide a `rayon`-backed path so `parallel_map` /
   `compute_bound_map` (and the i64/Fraction kernels) fan out on wasm too, while
   the native path stays as-is. Keep the compute-bound floor gating (Never
   Slower) unchanged.
4. **Build flags**: build the wasm with
   `RUSTFLAGS="-C target-feature=+atomics,+bulk-memory,+mutable-globals"` and
   `-Z build-std=panic_abort,std` on nightly (`scripts/rebuild-wasm.sh` and the
   `build.yml` / `test.yml` wasm jobs). Rebuild and commit
   `src/wasm/generated/` from the threaded build.
5. **Thread-pool init (JS)**: after `initWasm()`, when
   `detectParallelCapability().threadsAvailable`, call `initThreadPool(
   recommendedThreads)`; otherwise skip (single-threaded fallback).
6. **Register the fallback SW**: call `registerCrossOriginIsolation()` from the
   entry bootstrap so GitHub Pages gains isolation (one-time reload, guarded).
7. **Verify in a browser**: confirm `crossOriginIsolated === true` on
   ajisai.tech, that threads actually run (not the sequential fallback), and run
   a Never-Slower check so small inputs don't regress.

## Caveats

- **COEP `require-corp`** blocks any cross-origin subresource lacking CORP/CORS.
  The app vendors its assets locally (KaTeX in `public/vendor/katex`, images in
  `public/images`), so it is same-origin clean today — keep it that way, or the
  isolated page will fail to load that resource.
- **Non-supported browsers** (no SharedArrayBuffer, or isolation refused) must
  keep working single-threaded. `threadsAvailable` is false there and the
  sequential lane is always correct.
- **Tauri** runs in a native WebView and does not use any of this; the dev
  headers and SW are gated to the web target.
