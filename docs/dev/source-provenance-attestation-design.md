# Source Provenance Attestation (design note, non-canonical)

## Motivation

A programming language is a high-value supply-chain target. The 2021 attempt to
slip a backdoor into PHP's source, and the later `xz`/`liblzma` incident, both
show the same shape of threat: an attacker changes the *source of the language
implementation itself*, and the change rides downstream into every build.

A natural-but-flawed instinct is to place "something inside the language that
constantly tries to phone home", so that the moment a backdoor opens a channel
the attempt succeeds and the breach is detected. In Ajisai this is the wrong
shape twice over:

1. Ajisai's pure core has **no ambient authority**. Every outward effect is a
   `HostEffect` mediated by a closed `HostCapability` allowlist
   (`rust/src/interpreter/host.rs`); there is deliberately no `Network`
   capability. Embedding a constant communicator would mean *adding the very
   capability whose abuse we want to detect*.
2. The PHP threat is **source-level**. An attacker who can edit the source can
   edit (or delete) any in-source canary in the same commit. A runtime canary
   cannot defend against a compromise of the artifact it ships inside.

The defensible version of "detect the moment of injection" is **content
addressing of the source itself** — the same idea Ajisai already uses for word
identities (SPECIFICATION §8.6), lifted from individual words to the whole
trust-critical source surface. A backdoor changes file content, the content
digest changes, the aggregated root identity changes, and a drift guard fires.

## What this mechanism is

A deterministic attestation over the trust-critical source files:

- `scripts/generate-source-attestation.mjs` enumerates candidate files from
  `git ls-files` (committed files only), narrows them to a curated set of
  tracked roots, computes a per-file SHA-256 digest, sorts by path, and derives
  a Merkle-style **root identity** over the `(path, digest)` list. Enumerating
  from git rather than walking the working tree makes the root deterministic
  across environments: build steps that drop gitignored files (cargo creating
  `rust/Cargo.lock`, `node_modules`, `target/`) cannot perturb it. A backdoor
  that adds a new source file must `git add` it for the build to use it, which
  also enrolls it in the attested set.
- `docs/provenance/source-attestation.json` is the committed manifest: schema
  version, algorithm, the tracked globs, every file with its digest and size,
  and the root identity.
- `docs/provenance/source-root.txt` is a one-line **root pin**: just
  `sha256:<root>`. It is intentionally tiny so it can be mirrored, signed, or
  published to an external transparency log — see "Threat model" below.
- `npm run provenance:check` recomputes everything from the working tree and
  fails if (a) the manifest is stale, or (b) the recomputed root disagrees with
  the pin. This drift guard is the tripwire; it runs in CI next to the existing
  `word:manifest:check` and `check:skill` guards.

### Why SHA-256 and not the §8.6 polynomial digest

The §8.6 word-identity digest (`rust/src/interpreter/word_identity.rs`) is a
double-modulus polynomial hash chosen for cheap, deterministic *identity*. With
public primes and base it is **not collision-resistant against an adversary**,
who could craft two source bodies with the same digest. Provenance is a security
property facing a deliberate attacker, so it uses SHA-256 via Node's built-in
`node:crypto` (no new dependency). `word_identity.rs` already notes its digest
"may be replaced by a standard cryptographic hash without changing identity
semantics" — this mechanism is the security-grade sibling, not a replacement.

## Tracked surface

The tracked roots are declared explicitly at the top of the generator so the
set is auditable and reviewable. The default set is the language-defining,
trust-critical surface:

- `rust/src` — interpreter core, value model, built-ins, host boundary
- `src` — TypeScript runtime/GUI shell and platform adapters
- `src-tauri/src` — desktop host capabilities
- `scripts` — build/glue scripts (themselves part of the trusted base)
- `SPECIFICATION.html` — the canonical language definition
- build/config pins: `rust/Cargo.toml`, `rust/Cargo.lock`,
  `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `package.json`,
  `vite.config.ts`, `tsconfig.json`, `eslint.config.js`, and the CI workflows.

Generated or vendored content is excluded because its integrity is already
covered by its source: `src/wasm/generated` (built from `rust/src`),
`node_modules`, `dist`, `target`, `public/vendor`.

## Threat model — what it does and does not catch

**Catches:**

- Any change to a tracked file that is **not** accompanied by a regenerated
  attestation: CI fails immediately ("manifest is stale"). A backdoor slipped
  in without touching the attestation cannot pass CI silently.
- Drift between the committed manifest and the committed pin.
- During review, the root-identity change is a single, conspicuous line in the
  PR diff that a reviewer must consciously approve alongside the code.

**Does not catch, on its own:**

- An attacker with commit access who edits the source **and** regenerates both
  the manifest and the pin in the same change. The in-repo pin moves with the
  attacker.

This is the irreducible limit of any in-repo artifact, and it is why the pin is
factored out as a tiny, stable value. The real security boundary is **anchoring
the pin where the attacker cannot rewrite it**:

- protect `docs/provenance/source-root.txt` with branch protection / required
  review from a separate maintainer;
- sign the root in an annotated, signed git tag at each release;
- mirror the pin to an append-only external location (a transparency log, a
  second repo, a published release note) and compare during release
  verification.

With the pin externally anchored, the attestation becomes a true injection
tripwire: the root computed from the shipped source must equal the externally
held value, and any backdoor breaks that equality.

## Usage

```sh
# Regenerate the attestation after an intentional source change, then commit it.
npm run provenance:attest

# Drift guard (CI): fail if the committed attestation is stale or the pin drifts.
npm run provenance:check

# Print the manifest without writing (for inspection / external anchoring).
node scripts/generate-source-attestation.mjs --stdout
```

### Auto-sync at commit time (recommended)

Forgetting the regenerate-and-commit round-trip is the common reason
`provenance:check` fails in CI. To remove that surprise, the repository ships a
`pre-commit` hook (`.githooks/pre-commit` → `scripts/sync-source-attestation.sh`)
that, when the attested surface changed, regenerates the attestation and
**stages it into the same commit**. Because the regenerated files land in the
commit (and the PR diff), they are still reviewed — the hook automates the
chore, not the trust decision. CI keeps verifying with `provenance:check`, so a
missing or stale attestation is still caught even if the hook never ran.

Enable the repository hooks once per clone:

```sh
npm run hooks:install   # sets core.hooksPath=.githooks
```

This is intentionally opt-in (not forced via an `npm install` `prepare` step)
because `.githooks/pre-commit` also auto-creates dated branches, which would
otherwise interfere with workflows that commit on a fixed, non-dated branch.

## Relationship to existing guards

This sits alongside the established drift-guard family and reuses its `--check`
convention:

| Guard | Protects |
| --- | --- |
| `word:manifest:check` | the built-in word inventory matches the Rust sources |
| `check:skill` | `SKILL.md` matches the running CLI |
| `check:semantic-firewall` | no disallowed residue crosses the external protocol |
| **`provenance:check`** | **the trust-critical source matches its recorded content identity** |
