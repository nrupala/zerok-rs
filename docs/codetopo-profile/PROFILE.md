# Codetopo Profile — Zerok (Rust) (nrupala/zerok-rs)

**What this is.** The agent-facing map of this repository, generated with
Codetopo 0.9.1 (local engine, `~/.cargo/bin/codetopo`) so an arriving agent
can see the machine — topology, trust boundaries, money surface — before it
submits or changes anything. Read this first; the graph snapshot beside it
(`snapshot.json`, `stats.json`) is the measured evidence.

- **Repo:** nrupala/zerok-rs
- **Commit profiled:** `6026bcfd901e64862e2a82414e134ec1b16c95e5` (master)
- **Engine:** codetopo 0.9.1 · index run 2026-11-18
- **Graph:** 12 files walked · 0 failed · 279 nodes · 550 edges · audit chain valid (3 entries, head `0f05c9c8…ca232`)
- **Nodes by kind:** external 194 · method 27 · file 12 · function 12 · class 11 · module 11 · doc 6 · variable 5 · package 1
- **Edges by kind:** calls 315 · reads 126 · contains 84 · imports 23 · defines 2

## Coverage honesty

Codetopo extracts Rust, TypeScript/JavaScript and Python, plus Markdown as Doc nodes. This repo is Rust-only code: all 6 `.rs` files are extracted at full symbol level, and the 6 Markdown files are indexed as Doc nodes. `Cargo.toml` is data/config, not parsed into the graph. There is no other code language in this repo, so symbol coverage of the codebase is complete.

## Topology

The Rust implementation of Zerok (README): a zero-knowledge encrypted file store shipping as both a library (`zerok`) and a `zerok` CLI binary. Key derivation is PBKDF2 with 600,000 iterations; the storage server never sees keys or plaintext.

- `src/crypto.rs` — Lockbox-protocol encryption: `encrypt`/`decrypt`, key derivation, salt generation, password verifier.
- `src/storage.rs` — file store with a SHA-256 deduplication index.
- `src/cloud.rs` — S3 "blind" storage backend (`rusoto_s3`): stores ciphertext only.
- `src/version.rs` — compressed file-version history with retention policy (defaults per README: 5 versions, 30 days, 1024 MB).
- `src/lib.rs` — library root; `src/main.rs` — the CLI binary over the same modules.

## Trust boundaries

- **Keys never leave the client:** key derivation and the password verifier live in `src/crypto.rs`; both storage backends (`src/storage.rs` local, `src/cloud.rs` S3) receive ciphertext only — the S3 backend is explicitly "blind".
- **Cloud credentials, when the S3 backend is used, come from the standard rusoto/AWS environment chain** — no credential values or account identifiers are stored in this repo (names only were observed; nothing is reproduced here).
- **Blast radius:** `src/crypto.rs` — every module depends on it; a derivation or format change breaks every existing store.

## Money surface

**No money surface found.** Greps for `stripe|paddle|checkout|billing` over the Rust source returned no hits; no payment or spend path exists in the code. (The optional S3 backend stores data in the user's own cloud account — any storage cost there is the user's cloud bill, not a payment surface in this repo.)

## How an agent should approach

1. Read `README.md`, then this profile, then the snapshot.
2. Use `codetopo` queries against the snapshot (descendants, blast-radius,
   symbol) before editing — especially around the hubs named above.
3. Do not add credentials, account identifiers or key material to any file
   here — names and paths only. Respect the trust boundaries above as
   design constraints, not suggestions.
4. Run the repo's own tests after any change; regenerate this profile
   (below) whenever the code changes.

## Regeneration

`docs/codetopo-profile/refresh.sh` (in this pack) re-downloads the repo
at a named ref, re-runs the index, and rewrites `snapshot.json`,
`stats.json` and the header block inputs of this profile. Regenerate at
every release / material change.

Release gate: NOT RUN — Guardian Mesh: NOT RUN (this profile pack is a
documentation artifact; it is not a release-gate or Guardian Mesh verdict).
