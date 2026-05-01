import { defineConfig } from 'vitest/config';

// AQ-REQ-004 / AQ-VER-004 — Vitest configuration for TypeScript-side
// MC/DC tests. Kept intentionally minimal to mirror the Rust-side
// `cargo test` ergonomics: no globals, no UI, no coverage tooling here
// (the Rust quality gate already handles branch coverage; AQ-VER-004
// targets behavioural correctness of pure helpers).
export default defineConfig({
    test: {
        // Co-locate tests with source: src/**/*.test.ts.
        include: ['src/**/*.test.ts'],
        // No DOM helpers required for the current MC/DC suite. The few
        // tests that exercise `window` detection do so via deliberate
        // global stubbing, not via a simulated DOM.
        environment: 'node',
        globals: false,
        // Quality gate: surface unhandled rejections and errors.
        dangerouslyIgnoreUnhandledErrors: false,
        // Fail fast on snapshot drift; we don't use snapshots here, but
        // future contributors should opt in explicitly.
        passWithNoTests: false,
    },
});
