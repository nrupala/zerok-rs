# Test plan — zerok-rs

## Scope

Testing is intended to cover the `zerok` library crate: the cryptography
module (AES-256-GCM encrypt/decrypt round-trip, PBKDF2 key derivation,
password verifier) via its in-module unit tests, plus compilation of the
library, the `zerok` CLI binary and all doc-tests. The S3 cloud backend
(`rusoto_s3`) and end-to-end CLI vault workflows against real storage are
deliberately out of scope for the automated suite — they need external
services/credentials that CI does not have.

## Current state — read this first (2026-11-06)

**The crate does not compile, and the suite therefore cannot run.** This
is a pre-existing product defect, evidenced by the first CI runs this
plan was written for (runs 42285022155, 42285174333, 42285341110 — all
`failure` at the clippy step, because clippy compiles the crate):

- 16 compile errors, in `src/cloud.rs` (the majority: `rusoto_core` and
  `rusoto_credential` are used but never declared in `Cargo.toml`;
  `ByteStream` `From<String>`/`concat` misuse), `src/storage.rs` /
  `src/version.rs` (rusqlite `FromSql` tuple errors, unsized `[u8]`
  errors, one borrow-of-moved-value `data`).
- The `#[cfg(test)]` unit tests in `src/crypto.rs` exist and are
  well-formed, but `cargo test` compiles the whole library target, so
  they cannot execute until the errors above are fixed.
- Fixing 16 errors across the S3/SQLite modules is product development,
  not a readiness-leg fix; it is STOPPED here and recorded for the
  maintainer. No green test evidence is claimed for this repo.
- CodeQL (Rust, build-mode: none) does not need compilation and DOES
  run green (runs 42285174279, 42285341239 — `success`); its findings
  are dispositioned per the house alert SLA.

## Suites

| Suite | Covers | Run it | Runs in CI |
|---|---|---|---|
| Unit tests (`src/crypto.rs` `#[cfg(test)]`) | `test_encrypt_decrypt` (encrypt→decrypt round-trip returns the plaintext), `test_verifier` (verifier created from a derived key validates against that key) | `cargo test` | Wired in CI — currently blocked by the compile errors above |
| Doc-tests + crate compilation | Library, binary and doc examples compile and pass | `cargo test` | Wired in CI — currently blocked by the compile errors above |

## CI gates

- CI workflow (`.github/workflows/ci.yml`) on every push and pull request
  to `master`: `cargo clippy --all-targets` (warnings reviewed; not run
  with `-D warnings` — the codebase's tolerance for deny-warnings has not
  been established), then `cargo test`. Today this gate is RED for the
  compile reasons above; the red output is the honest evidence until the
  product lane fixes the crate.
- CodeQL workflow (`.github/workflows/codeql.yml`, Rust, build-mode:
  none) on push, pull request and a weekly schedule — GREEN; findings
  are dispositioned per the house alert SLA (fix, or a dismissal with
  the reason recorded).

## Cadence

- Every pull request: clippy + the full `cargo test` suite, and CodeQL.
- Weekly: scheduled CodeQL run.
- Every release candidate: the release-readiness run (checker + evidence
  reads), per the house Release-Readiness Standard.

## Coverage bar

No numeric floor; new functionality lands with tests per CONTRIBUTING.
