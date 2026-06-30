#!/usr/bin/env bash
# Keep docs/provenance/source-attestation.json in sync at commit time.
#
# Why: the attestation content-addresses the trust-critical source surface
# (rust/src/, src/, src-tauri/src/, scripts/, plus tracked files like
# SPECIFICATION.html / package.json / the workflow YAMLs). Editing any of those
# changes the root identity, and `npm run provenance:check` (wired into CI) then
# fails until the attestation is regenerated and committed. Forgetting that
# round-trip is the recurring CI surprise this hook removes.
#
# Security: this regenerates and *stages* the attestation into the current
# commit, so the change still lands in the PR diff and is reviewed. The
# attestation is never minted by CI; the backdoor tripwire (a source change with
# no matching, reviewed attestation) is preserved.
#
# Invoked from .githooks/pre-commit. Enable the hooks with:
#   npm run hooks:install      # sets core.hooksPath=.githooks
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

# No Node (minimal/partial environment): skip silently. CI still guards the
# invariant, so a missing local toolchain can never weaken the check.
command -v node >/dev/null 2>&1 || exit 0

# Fast path: nothing on the attested surface changed, so the committed
# attestation is already current.
if node scripts/generate-source-attestation.mjs --check >/dev/null 2>&1; then
  exit 0
fi

echo "[provenance] source attestation is stale; regenerating and staging…"
node scripts/generate-source-attestation.mjs >/dev/null
git add docs/provenance/source-attestation.json docs/provenance/source-root.txt
echo "[provenance] staged docs/provenance/ — included in this commit."
