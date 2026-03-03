use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, AeadCore,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

/// Encrypt plaintext using AES-256-GCM.
/// Returns `(ciphertext_base64, nonce_base64)`.
pub fn encrypt(plaintext: &str, key_b64: &str) -> Result<(String, String), String> {
    let key_bytes = B64
        .decode(key_b64)
        .map_err(|e| format!("invalid encryption key base64: {e}"))?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "encryption key must be 32 bytes, got {}",
            key_bytes.len()
        ));
    }

    let cipher =
        Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| format!("cipher init error: {e}"))?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| format!("encryption error: {e}"))?;

    Ok((B64.encode(ciphertext), B64.encode(nonce)))
}

/// Decrypt ciphertext using AES-256-GCM.
pub fn decrypt(ciphertext_b64: &str, nonce_b64: &str, key_b64: &str) -> Result<String, String> {
    let key_bytes = B64
        .decode(key_b64)
        .map_err(|e| format!("invalid encryption key base64: {e}"))?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "encryption key must be 32 bytes, got {}",
            key_bytes.len()
        ));
    }

    let cipher =
        Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| format!("cipher init error: {e}"))?;

    let nonce_bytes = B64
        .decode(nonce_b64)
        .map_err(|e| format!("invalid nonce base64: {e}"))?;
    let nonce = aes_gcm::Nonce::from_slice(&nonce_bytes);

    let ciphertext = B64
        .decode(ciphertext_b64)
        .map_err(|e| format!("invalid ciphertext base64: {e}"))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("decryption error: {e}"))?;

    String::from_utf8(plaintext).map_err(|e| format!("invalid UTF-8 in decrypted data: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        // 32-byte key encoded as base64
        let key = B64.encode([0xABu8; 32]);
        let plaintext = "-----BEGIN OPENSSH PRIVATE KEY-----\ntest\n-----END OPENSSH PRIVATE KEY-----";

        let (ct, nonce) = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&ct, &nonce, &key).unwrap();
        assert_eq!(decrypted, plaintext);
    }
}
