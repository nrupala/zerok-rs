//! Cryptography module - Lockbox Protocol implementation
//! 
//! Uses AES-256-GCM for encryption and PBKDF2 for key derivation

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

const PBKDF2_ITERATIONS: u32 = 600_000;
const SALT_BYTES: usize = 32;
const NONCE_BYTES: usize = 12;
const KEY_BYTES: usize = 32;

/// Derive a 256-bit key from password and salt using PBKDF2
pub fn derive_key(password: &str, salt: &[u8]) -> [u8; KEY_BYTES] {
    let mut key = [0u8; KEY_BYTES];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

/// Generate a random 32-byte salt
pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; SALT_BYTES];
    OsRng.fill_bytes(&mut salt);
    salt
}

/// Generate a random 12-byte nonce
fn generate_nonce() -> [u8; NONCE_BYTES] {
    let mut nonce = [0u8; NONCE_BYTES];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

/// Encrypt data using AES-256-GCM
/// Returns: nonce (12 bytes) + ciphertext
pub fn encrypt(plaintext: &[u8], key: &[u8; KEY_BYTES]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {}", e))?;
    
    let nonce = generate_nonce();
    let nonce = Nonce::from_slice(&nonce);
    
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;
    
    // Prepend nonce to ciphertext
    let mut result = Vec::with_capacity(NONCE_BYTES + ciphertext.len());
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    
    Ok(result)
}

/// Decrypt data using AES-256-GCM
pub fn decrypt(encrypted: &[u8], key: &[u8; KEY_BYTES]) -> Result<Vec<u8>, String> {
    if encrypted.len() < NONCE_BYTES + 16 {
        return Err("Invalid encrypted data".to_string());
    }
    
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| format!("Invalid key: {}", e))?;
    
    let nonce = Nonce::from_slice(&encrypted[..NONCE_BYTES]);
    let ciphertext = &encrypted[NONCE_BYTES..];
    
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))
}

/// Create a verifier for password validation
pub fn create_verifier(key: &[u8; KEY_BYTES]) -> Result<Vec<u8>, String> {
    let plaintext = b"ZEROK_VAULT_VERIFIED";
    encrypt(plaintext, key)
}

/// Verify password against stored verifier
pub fn verify_password(key: &[u8; KEY_BYTES], verifier: &[u8]) -> bool {
    match decrypt(verifier, key) {
        Ok(decrypted) => decrypted == b"ZEROK_VAULT_VERIFIED",
        Err(_) => false,
    }
}

/// Hash data using SHA-256
pub fn hash(data: &[u8]) -> Vec<u8> {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Hash string to base64
pub fn hash_string(s: &str) -> String {
    BASE64.encode(hash(s.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let salt = generate_salt();
        let key = derive_key("test_password", &salt);
        
        let plaintext = b"Hello, Zerok!";
        let encrypted = encrypt(plaintext, &key).unwrap();
        
        let decrypted = decrypt(&encrypted, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_verifier() {
        let salt = generate_salt();
        let key = derive_key("test_password", &salt);
        
        let verifier = create_verifier(&key).unwrap();
        assert!(verify_password(&key, &verifier));
    }
}