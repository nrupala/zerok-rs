# zerok-rs — Zerok Vault in Rust

High-performance Rust implementation of Zerok Vault: zero-knowledge encrypted
file storage. Data is encrypted on the local device (AES-256-GCM, PBKDF2 key
derivation with 600,000 iterations); the storage server never sees keys or
plaintext. Ships as both a library (`zerok`) and a `zerok` CLI binary.

## Modules

| Module | What it does |
|---|---|
| `crypto` | Lockbox-protocol encryption: `encrypt`/`decrypt`, key derivation, salt generation, password verifier |
| `storage` | File store with SHA-256 deduplication index |
| `cloud` | S3 "blind" storage backend (`rusoto_s3`) |
| `version` | Compressed file-version history with retention policy (defaults: 5 versions, 30 days, 1024 MB) |

## Build

```bash
cargo build --release
```

Release profile uses `opt-level = 3` and LTO.

## Test

```bash
cargo test
```

## Usage

```bash
# Initialize a new vault
./target/release/zerok init --path ./vault

# Import files with optional deduplication
./target/release/zerok import --source ./photos --path ./vault --dedup

# Verify vault integrity
./target/release/zerok verify --path ./vault

# Find duplicate files
./target/release/zerok hash --source ./folder
```

(Command flags per the clap derive definitions in `src/main.rs`.)

## License

The owner has not yet supplied the license text — see the PR body.
