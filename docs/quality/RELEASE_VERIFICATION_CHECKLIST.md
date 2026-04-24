# Release Verification Checklist

- [ ] Version/release notes are updated.
- [ ] CI quality gates are green on release commit.
- [ ] Rust formatting, clippy, and all-target tests pass.
- [ ] TypeScript checks pass (when applicable).
- [ ] WASM build verification passes.
- [ ] Traceability matrix has no unresolved high-criticality gaps.
- [ ] Known quality issues are dispositioned for the release.
- [ ] Build artifacts are reproducible from repository sources.
