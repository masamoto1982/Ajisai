# File Naming Convention

## Purpose

Ajisai applies the `Naming-as-Index` principle to **file names as well as identifiers**. File names must optimize for AI/LLM inference, `grep`/`rg`, vector search, cross-repository analysis, and long-term maintenance rather than visual brevity.

## Core Rules

- Use **lowercase kebab-case** for repository-managed files whenever toolchains allow it.
- Prefer names that expose **domain → subdomain → role → subject → type** in that order.
- Choose names that let a reader infer the primary responsibility from the filename alone.
- Treat filenames as search indexes, not decorative labels.

## AI-First Criteria

A filename is preferred only if the answer is **Yes** to these questions:

1. Can the primary responsibility be inferred from the name alone?
2. Is it easy to find with `grep`, `rg`, and LLM-assisted repository search?
3. Is it distinguishable from neighboring files?
4. Will the meaning still hold one year later?
5. Is it free from temporary context such as `old`, `new`, or `final`?
6. Is it still meaningful when listed without its parent directory?

## Prohibited or Discouraged Names

Avoid weak or context-dependent names unless a framework requires them:

- `utils`
- `helper`, `helpers`
- `common`
- `misc`
- `tmp`, `temp`
- `new`, `old`, `final`, `draft`
- `main`
- `index`
- `core`, `base` when the subject is not explicit
- `test2`, `v2`, `latest`

Conditional words such as `manager`, `service`, `adapter`, `factory`, `core`, and `base` are allowed only when the filename clearly states **what** they manage or serve.

## Test File Naming

Make the test target directly discoverable.

Recommended patterns:

- `subject-behavior.test.ts`
- `subject-behavior.integration.test.ts`
- `subject-flow.e2e.test.ts`
- `subject-regression-tests.rs`
- `subject-sample-test.ajisai`

Discouraged patterns:

- `test-auth.ts`
- `auth-test.ts`
- `store.spec2.ts`
- `test_tensor.ajisai`

## Documentation File Naming

Documentation filenames must expose purpose explicitly.

Recommended prefixes:

- `spec-`
- `design-`
- `guide-`
- `decision-`
- `migration-`
- `runbook-`

Examples:

- `design-module-boundaries.md`
- `migration-file-renaming-inventory.md`
- `guide-file-naming-convention.md`

## Required Follow-Up When Renaming

Any file rename must also update all dependent references, including:

- imports / exports
- dynamic imports
- worker entrypoints
- build and bundler config
- test references
- docs links
- service-worker caches
- toolchain globs
- package exports
- generated lookup tables when applicable

## Allowed Exceptions

Keep framework-constrained filenames only when renaming would reduce compatibility or add disproportionate indirection, for example:

- web entry documents such as `index.html`
- Rust directory modules such as `mod.rs` when path indirection is not justified
- generated artifacts such as packaged WASM bindings

If an exception is kept, document **why** it remains.
