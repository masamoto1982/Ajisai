#!/usr/bin/env python3
"""Differential test driver for the Ajisai Core reference interpreter.

Runs a corpus of Ajisai Core programs through both
  (a) the production Rust CLI (`cargo build --bin ajisai --release`), and
  (b) the maintained Python reference interpreter (`ajisai.py`, the
      "executable shadow of the spec"),
then reports every observable divergence.

Authority (see README.md and SPECIFICATION.html SS2.4/SS2.5): both sides are
*verification artifacts* subordinate to SPECIFICATION.html. A divergence is a
signal -- a candidate spec hole (class A) or implementation bug (class B), to be
adjudicated by the suite-arbitration rule of
`docs/dev/spec-impl-drift-tactic.md` SS3.3. This driver detects and records
divergences; it never decides their direction and never edits semantics.

Exit code: 0 when no divergence is observed, 1 when at least one is, so CI can
gate on it. The human-readable summary is preserved.

Configuration:
  AJISAI_BIN   path to the production CLI. Defaults to the repo-relative
               `rust/target/release/ajisai`.
Usage:
  python3 compare.py                 # inline Core corpus only
  python3 compare.py --conformance   # also include tests/conformance core cases
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))


def resolve_bin():
    """Production CLI path: AJISAI_BIN env, else repo-relative release build."""
    return os.environ.get("AJISAI_BIN") or os.path.join(
        REPO_ROOT, "rust", "target", "release", "ajisai"
    )


BIN = resolve_bin()

sys.path.insert(0, HERE)
import ajisai as repro  # noqa: E402


def orig(src):
    """Run the production CLI and project onto (status, stack, output)."""
    with tempfile.NamedTemporaryFile(
        "w", suffix=".ajisai", delete=False
    ) as f:
        f.write(src)
        path = f.name
    try:
        r = subprocess.run(
            [BIN, "run", path, "--json"], capture_output=True, text=True
        )
    finally:
        os.unlink(path)
    try:
        d = json.loads(r.stdout)
    except Exception:
        return {"status": "crash"}
    if d["status"] == "ok":
        return {"status": "ok", "stack": d.get("stackDisplay"), "output": d.get("output")}
    ai = d.get("aiDiagnostic") or {}
    return {"status": "error", "kind": ai.get("kind")}


def rep(src):
    """Run the reference interpreter, hardened against unexpected exceptions.

    The reference interpreter may legitimately not yet cover some surface
    syntax; an uncaught Python exception there must not abort the whole run, so
    it is recorded as a `crash` (itself a divergence to surface)."""
    try:
        return repro.run_program(src)
    except Exception:
        return {"status": "crash"}


def norm(r):
    """Canonical observation: status plus, on success, the Display-rendered
    stack and the output buffer.

    Comparison is by value identity via the canonical Display (SS12). Note the
    known display-only artifact recorded in DIVERGENCE-ANALYSIS.md: the headless
    CLI renders a NIL produced by CHR as the empty string ''. Such display-only
    differences are still surfaced here (not silently masked) and are recorded,
    not fixed, in line with spec-impl-drift-tactic.md SS3.3."""
    if r["status"] != "ok":
        return ("error", r.get("kind"))
    return ("ok", tuple(r.get("stack") or []), tuple(r.get("output") or []))


# Inline Core corpus. Host-dependent (Hosted) effects are intentionally out of
# scope for the Core reference interpreter (PORTABILITY.md Core/Hosted split);
# those are covered by the conformance suite and production implementations.
INLINE_CORPUS = [
    # arithmetic
    "1 2 ADD", "5 3 SUB", "3 4 MUL", "10 3 DIV", "10 3 MOD", "7 2 DIV",
    "5 0 DIV", "5 0 MOD", "1 1 ADD",
    "3.14 FLOOR", "3.14 CEIL", "3.5 ROUND", "2.5 ROUND", "-2.5 ROUND",
    # comparison
    "1 2 LT", "2 1 LT", "1 1 EQ", "1 1 LTE", "2 2 GTE", "1 2 NEQ", "1 1 NEQ",
    "1 2 3 .. LT", "3 2 1 .. LT", "1 1 1 .. EQ", "1 2 2 .. LTE",
    # logic K3
    "TRUE FALSE AND", "TRUE TRUE AND", "FALSE FALSE OR", "TRUE FALSE OR",
    "TRUE NOT", "FALSE NOT", "TRUE 1 EQ",
    # nil / bubble / vent
    "5 0 DIV", "5 0 DIV 99 ^", "5 0 DIV 1 ADD", "NIL", "NIL 7 ^", "42 7 ^",
    # nil diagnostic accessors (SPEC §4.5.0 / §7.15)
    "1 0 / NIL-REASON", "1 0 / NIL-ORIGIN", "1 0 / NIL-RECOVERABLE?",
    "1 0 / NIL-DIAGNOSIS", "1 0 / NIL?", "5 NIL?", "5 NIL-REASON",
    "NIL NIL-ORIGIN", "NIL NIL-RECOVERABLE?", "NIL NIL-REASON",
    # vectors
    "[ 1 2 3 ] LENGTH", "[ 1 2 3 ] 1 GET", "[ 1 2 3 ] 9 GET", "[ 1 2 3 ] -1 GET",
    "[ 1 2 3 ] REVERSE", "[ 1 2 ] [ 3 4 ] CONCAT", "0 5 RANGE",
    "[ 1 2 3 4 ] 2 TAKE", "[ 1 2 3 ] 1 5 REPLACE", "[ 1 2 3 ] 1 REMOVE",
    "[ 1 2 3 ] 1 9 INSERT", "1 2 3 COLLECT",
    # tensor
    "[ [ 1 2 ] [ 3 4 ] ] SHAPE", "[ [ 1 2 ] [ 3 4 ] ] RANK", "[ 1 2 3 ] RANK",
    # strings
    "'hello'", "'ab' 'cd' CONCAT", "'hello' CHARS", "65 CHR", "'42' NUM",
    "'abc' NUM", "[ 'a' 'b' ] JOIN", "1114112 CHR",
    # modifiers
    "1 2 ,, ADD", "1 2 3 ;ADD", "5 ,, PRINT",
    # higher order
    "[ 1 2 3 ] { 1 ADD } MAP", "[ 1 2 3 4 ] { 2 MOD 0 EQ } FILTER",
    "[ 1 2 3 ] 0 { ADD } FOLD", "{ 1 2 ADD } EXEC",
    # cond
    "5",
    # def
    "{ 2 MUL } 'DOUBLE' DEF 21 DOUBLE",
    # print/output
    "'TEST' PRINT", "42 PRINT", "[ 'AB' 'CD' ] PRINT",
    # sqrt (needs MATH)
    "'MATH' IMPORT 2 SQRT 2 SQRT SUB", "'MATH' IMPORT 2 SQRT 2 SQRT EQ",
    "'MATH' IMPORT 4 SQRT", "'MATH' IMPORT 2 SQRT 2 SQRT MUL",
    # bool distinctness
    "TRUE 1 EQ", "FALSE 0 EQ",
]


def conformance_corpus():
    """Programs extracted from the Core cases of tests/conformance/.

    The manifest is produced by the canonical generator
    (scripts/generate-conformance-manifest.mjs); only `core` cases are taken,
    since the reference interpreter is Core-only (Hosted cases require host
    capabilities). Returns [] if Node or the generator is unavailable, so the
    inline corpus still runs."""
    gen = os.path.join(REPO_ROOT, "scripts", "generate-conformance-manifest.mjs")
    if not os.path.exists(gen):
        print(f"warning: conformance generator not found at {gen}", file=sys.stderr)
        return []
    try:
        r = subprocess.run(
            ["node", gen], capture_output=True, text=True, cwd=REPO_ROOT
        )
    except FileNotFoundError:
        print("warning: `node` not available; skipping conformance corpus",
              file=sys.stderr)
        return []
    if r.returncode != 0:
        print(f"warning: conformance generator failed:\n{r.stderr}", file=sys.stderr)
        return []
    manifest = json.loads(r.stdout)
    return [
        c["source"]
        for c in manifest.get("cases", [])
        if c.get("category") == "core" and c.get("source")
    ]


def run_corpus(corpus):
    diffs = []
    same = 0
    for t in corpus:
        o = orig(t)
        r = rep(t)
        if norm(o) == norm(r):
            same += 1
        else:
            diffs.append((t, o, r))
    return same, diffs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--conformance",
        action="store_true",
        help="also run programs extracted from tests/conformance core cases",
    )
    args = parser.parse_args()

    if not os.path.exists(BIN):
        print(
            f"error: production CLI not found at {BIN}\n"
            f"build it with `cargo build --bin ajisai --release` (in rust/),\n"
            f"or set AJISAI_BIN to its path.",
            file=sys.stderr,
        )
        return 2

    corpus = list(INLINE_CORPUS)
    if args.conformance:
        corpus += conformance_corpus()

    same, diffs = run_corpus(corpus)
    print(f"== {same} identical, {len(diffs)} divergent of {len(corpus)} ==\n")
    for t, o, r in diffs:
        print(f"SRC : {t}")
        print(f"orig: {o}")
        print(f"repr: {r}")
        print()

    return 1 if diffs else 0


if __name__ == "__main__":
    sys.exit(main())
