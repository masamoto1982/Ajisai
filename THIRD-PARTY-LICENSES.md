# Third-party licenses

Ajisai Core has three dependencies, all of them exact integer arithmetic. The
packages add none.

| Crate | Purpose | License |
|---|---|---|
| [num-bigint](https://github.com/rust-num/num-bigint) | arbitrary-precision integers | MIT OR Apache-2.0 |
| [num-integer](https://github.com/rust-num/num-integer) | integer traits (`gcd`) | MIT OR Apache-2.0 |
| [num-traits](https://github.com/rust-num/num-traits) | numeric traits | MIT OR Apache-2.0 |

`autocfg` (MIT OR Apache-2.0) is pulled in as a build dependency of the above.

`ajisai-audit` implements SHA-256 in-crate rather than take a dependency in a
trust-sensitive position, so it adds nothing to this list.

Ajisai itself is MIT licensed; see `LICENSE`.

Regenerate this list with:

```sh
cargo tree --workspace --prefix none --format '{p} {l}'
```
