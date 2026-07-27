## Summary

<!-- What changed, and why. -->

## Kind of change

- [ ] Language change — `SPECIFICATION.md` is updated in the same PR
- [ ] Implementation change with no language change
- [ ] Package (`ajisai-music`, `ajisai-audit`)
- [ ] Documentation

## If this changes the language

- [ ] `SPECIFICATION.md` states the execution rule, stack effect, type rule, and error rule
- [ ] `docs/migration.md` records the observable change
- [ ] A test fixes the new behaviour

## If this adds a field, variant, or classification

- [ ] Something reads it, and behaves differently because of what it says
- [ ] `docs/semantic-ontology.md` names its producers and consumers

## Verification

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo build --workspace --release`

## Notes

<!-- Anything a reviewer should know. -->
