# Test plan — zerok-rs

## Scope

Testing covers the `zerok` library crate: the cryptography module
(AES-256-GCM encrypt/decrypt round-trip, PBKDF2 key derivation, password
verifier) via its in-module unit tests, plus compilation of the library,
the `zerok` CLI binary and all doc-tests. The S3 cloud backend
(`rusoto_s3`) and end-to-end CLI vault workflows against real storage are
deliberately out of scope for the automated suite — they need external
services/credentials that CI does not have; that scoping is recorded here
honestly rather than implied.

## Suites

| Suite | Covers | Run it | Runs in CI |
|---|---|---|---|
| Unit tests (`src/crypto.rs` `#[cfg(test)]`) | `test_encrypt_decrypt` (encrypt→decrypt round-trip returns the plaintext), `test_verifier` (verifier created from a derived key validates against that key) | `cargo test` | Yes — CI workflow |
| Doc-tests + crate compilation | Library, binary and doc examples compile and pass | `cargo test` | Yes — CI workflow |

## CI gates

- CI workflow (`.github/workflows/ci.yml`) on every push and pull request
  to `master`: `cargo clippy --all-targets` (warnings reviewed; not run
  with `-D warnings` — the codebase's tolerance for deny-warnings has not
  been established yet, recorded here rather than claimed), then
  `cargo test` must pass.
- CodeQL workflow (`.github/workflows/codeql.yml`, Rust, build-mode:
  none) on push, pull request and a weekly schedule; findings are
  dispositioned per the house alert SLA (fix, or a dismissal with the
  reason recorded).

## Cadence

- Every pull request: clippy + the full `cargo test` suite, and CodeQL.
- Weekly: scheduled CodeQL run.
- Every release candidate: the release-readiness run (checker + evidence
  reads), per the house Release-Readiness Standard.

## Coverage bar

No numeric floor; new functionality lands with tests per CONTRIBUTING.
