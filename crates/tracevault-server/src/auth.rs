use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use sha2::{Digest, Sha256};

/// Hash a password with Argon2id. Returns PHC-format string.
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    Ok(argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

/// Verify a password against a PHC-format hash.
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

fn random_bytes_32() -> [u8; 32] {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

/// Generate a random session token. Returns (raw_token, sha256_hash).
/// Raw token is prefixed with `tvs_` and hex-encoded.
pub fn generate_session_token() -> (String, String) {
    let bytes = random_bytes_32();
    let raw = format!("tvs_{}", hex::encode(bytes));
    let hash = sha256_hex(&raw);
    (raw, hash)
}

/// Generate a random API key. Returns (raw_key, sha256_hash).
/// Raw key is prefixed with `tvk_` and hex-encoded.
pub fn generate_api_key() -> (String, String) {
    let bytes = random_bytes_32();
    let raw = format!("tvk_{}", hex::encode(bytes));
    let hash = sha256_hex(&raw);
    (raw, hash)
}

/// Generate a random device auth token (hex-encoded, no prefix).
pub fn generate_device_token() -> String {
    let bytes = random_bytes_32();
    hex::encode(bytes)
}

/// SHA-256 hash a string, return hex-encoded.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
