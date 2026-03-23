use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::SigningKey;
use sqlx::PgPool;
use uuid::Uuid;

use crate::signing::SigningService;

/// Validate a user-provided signing key seed (base64-encoded 32 bytes).
pub fn validate_seed(seed_b64: &str) -> Result<(), String> {
    let bytes = BASE64
        .decode(seed_b64)
        .map_err(|_| "Invalid base64 encoding".to_string())?;
    if bytes.len() != 32 {
        return Err(format!(
            "Signing key seed must be 32 bytes, got {}",
            bytes.len()
        ));
    }
    // Verify it produces a valid Ed25519 key
    let seed_arr: [u8; 32] = bytes.try_into().unwrap();
    let _ = SigningKey::from_bytes(&seed_arr);
    Ok(())
}

/// Store a signing key (auto-generated or user-provided) for an org.
/// If `provided_seed_b64` is Some, validates and uses it. Otherwise generates a new one.
/// Also inserts the initial entry in org_signing_key_history.
/// Returns the raw seed (base64) so it can be shown to the user once.
pub async fn generate_and_store(
    pool: &PgPool,
    org_id: Uuid,
    encryption_key: &str,
    provided_seed_b64: Option<&str>,
) -> Result<String, String> {
    let seed_b64 = match provided_seed_b64 {
        Some(seed) => {
            validate_seed(seed)?;
            seed.to_string()
        }
        None => {
            let seed = SigningKey::generate(&mut rand::thread_rng());
            BASE64.encode(seed.to_bytes())
        }
    };

    let (encrypted, nonce) = crate::encryption::encrypt(&seed_b64, encryption_key)?;

    sqlx::query("UPDATE orgs SET signing_key_encrypted = $1, signing_key_nonce = $2 WHERE id = $3")
        .bind(&encrypted)
        .bind(&nonce)
        .bind(org_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO org_signing_key_history (org_id, signing_key_encrypted, signing_key_nonce, active_from)
         VALUES ($1, $2, $3, NOW())",
    )
    .bind(org_id)
    .bind(&encrypted)
    .bind(&nonce)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(seed_b64)
}

/// Load the current signing service for an org.
pub async fn load_current(
    pool: &PgPool,
    org_id: Uuid,
    encryption_key: &str,
) -> Result<Option<SigningService>, String> {
    let row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT signing_key_encrypted, signing_key_nonce FROM orgs WHERE id = $1",
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    match row {
        (Some(encrypted), Some(nonce)) => {
            let seed_b64 = crate::encryption::decrypt(&encrypted, &nonce, encryption_key)?;
            Ok(Some(SigningService::new(Some(&seed_b64))))
        }
        _ => Ok(None),
    }
}

/// Load the signing service for an org at a specific point in time (for verification).
pub async fn load_at_time(
    pool: &PgPool,
    org_id: Uuid,
    sealed_at: &chrono::DateTime<chrono::Utc>,
    encryption_key: &str,
) -> Result<Option<SigningService>, String> {
    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT signing_key_encrypted, signing_key_nonce FROM org_signing_key_history
         WHERE org_id = $1 AND active_from <= $2 AND (active_until IS NULL OR active_until > $2)
         ORDER BY active_from DESC LIMIT 1",
    )
    .bind(org_id)
    .bind(sealed_at)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    match row {
        Some((encrypted, nonce)) => {
            let seed_b64 = crate::encryption::decrypt(&encrypted, &nonce, encryption_key)?;
            Ok(Some(SigningService::new(Some(&seed_b64))))
        }
        None => Ok(None),
    }
}
