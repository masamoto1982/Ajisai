#!/usr/bin/env python3
"""Run snippets through the production Rust CLI and extract a compact result.

The CLI path is resolved from the AJISAI_BIN environment variable, defaulting to
the repo-relative `rust/target/release/ajisai` (same resolution as compare.py)."""
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.abspath(os.path.join(HERE, "..", ".."))
BIN = os.environ.get("AJISAI_BIN") or os.path.join(
    REPO_ROOT, "rust", "target", "release", "ajisai"
)


def run(src):
    with tempfile.NamedTemporaryFile("w", suffix=".ajisai", delete=False) as f:
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
        return {"status": "crash", "raw": r.stdout[:200] + r.stderr[:200]}
    out = {"status": d["status"]}
    if d["status"] == "ok":
        out["stack"] = d.get("stackDisplay")
        out["output"] = d.get("output")
    else:
        ai = d.get("aiDiagnostic") or {}
        out["kind"] = ai.get("kind")
    return out


if __name__ == "__main__":
    for s in sys.argv[1:]:
        print(repr(s), "->", run(s))
