# Current Test Status (Non-Canonical Operational Note)

This file is an operational note only.
It does not define language semantics.

Canonical semantics are defined only in `SPECIFICATION.md`.

## CI build matrix

Current CI (`.github/workflows/test.yml`) runs on `ubuntu-latest` only:

- `cargo check` (native Core build, no wasm deps)
- `cargo check --features wasm --target wasm32-unknown-unknown`
- `cargo test` / `cargo test --all-targets`

TODO(portability): add a native OS matrix (ubuntu / macos / windows) so the
host-clock (`SystemTime`) and OS-CSPRNG (`getrandom`) boundaries are exercised
on every supported native host, not just Linux. This is a future goal; the
single-OS gate above is the current baseline.

