//! Zerok Vault - Zero-knowledge encrypted storage library
//! 
//! Implements the Lockbox Protocol for AES-256-GCM encryption
//! with PBKDF2 key derivation (600,000 iterations).

pub mod crypto;
pub mod storage;
pub mod cloud;
pub mod version;

pub use crypto::{encrypt, decrypt, derive_key, generate_salt, verify_password, create_verifier};
pub use storage::FileStore;
pub use cloud::S3Backend;
pub use version::VersionManager;