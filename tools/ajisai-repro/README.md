# ajisai-repro — independent Ajisai reproduction & spec-vs-impl comparison

An independent re-implementation of Ajisai's host-independent core, written in
Python **from `SPECIFICATION.html` prose alone** (without consulting the Rust
sources), used as an oracle to diff against the original `ajisai` CLI.

See **FINDINGS.md** for the analysis and the discovered spec/implementation
divergences.

## Files
- `ajisai.py`  — the Python reproduction (run a program: `python3 ajisai.py "1 2 ADD"`).
- `probe.py`   — runs the original Rust CLI and extracts a compact result.
- `compare.py` — runs a battery through both and prints every divergence.
- `compare-output.txt` — last comparison run.

## Run
```
# build the original headless CLI first
( cd ../../rust && cargo build --bin ajisai --release )
python3 compare.py
```
