use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct SigningService {
    verifying_key: VerifyingKey,
}

impl SigningService {
    /// Create from a base64-encoded 32-byte seed, or generate a new key.
    pub fn new(seed_b64: Option<&str>) -> Self {
        let signing_key = if let Some(seed) = seed_b64 {
            let bytes = BASE64
                .decode(seed)
                .expect("Invalid base64 signing key seed");
            let seed_bytes: [u8; 32] = bytes.try_into().expect("Signing key seed must be 32 bytes");
            SigningKey::from_bytes(&seed_bytes)
        } else {
            tracing::warn!("No TRACEVAULT_SIGNING_KEY set — using ephemeral key. Signatures will not survive restart.");
            SigningKey::generate(&mut rand::thread_rng())
        };
        let verifying_key = signing_key.verifying_key();
        Self { verifying_key }
    }

    /// Compute chain hash: SHA-256(prev_chain_hash || record_hash).
    pub fn chain_hash(&self, prev_chain_hash: Option<&str>, record_hash: &str) -> String {
        let mut hasher = Sha256::new();
        if let Some(prev) = prev_chain_hash {
            hasher.update(prev.as_bytes());
        }
        hasher.update(record_hash.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Verify a signature against a record hash.
    pub fn verify(&self, record_hash: &str, signature_b64: &str) -> bool {
        let sig_bytes = match BASE64.decode(signature_b64) {
            Ok(b) => b,
            Err(_) => return false,
        };
        let sig = match Signature::from_slice(&sig_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        self.verifying_key
            .verify(record_hash.as_bytes(), &sig)
            .is_ok()
    }

    /// Get the public key as base64 for distribution.
    pub fn public_key_b64(&self) -> String {
        BASE64.encode(self.verifying_key.as_bytes())
    }
}
