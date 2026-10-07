# Contributing to zerok-rs

## Workflow (PR-flow discipline)

- Work happens on feature branches — **never push directly to `master`**.
- Open pull requests as **draft** first. Mark ready only when:
  - `cargo build` and `cargo test` are green,
  - the CHANGELOG has an entry under `## [Unreleased]`,
  - the `version` in `Cargo.toml` was bumped (patch = fix, minor = feature,
    major = breaking),
  - the PR description states what was verified vs what was not.
- The owner merges. Merge commits reference the PR number.
- Releases are tagged `vX.Y.Z` after merge.

## Build and test

```bash
cargo build --release   # optimized build (opt-level 3, LTO)
cargo test              # test suite
cargo fmt --check       # formatting
```

## License

License text pending from the owner (`Cargo.toml` declares MIT but no
`LICENSE` file exists yet). Do not add license headers until the license is
confirmed.
