use sqlx::PgPool;
use uuid::Uuid;

pub struct AuditEntry {
    pub org_id: Uuid,
    pub actor_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Insert an audit log entry. Errors are logged but not propagated
/// to avoid breaking the primary operation.
pub async fn log(pool: &PgPool, entry: AuditEntry) {
    let result = sqlx::query(
        "INSERT INTO audit_log (org_id, actor_id, api_key_id, action, resource_type, resource_id, details, ip_address, user_agent)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9)"
    )
    .bind(entry.org_id)
    .bind(entry.actor_id)
    .bind(entry.api_key_id)
    .bind(&entry.action)
    .bind(&entry.resource_type)
    .bind(entry.resource_id)
    .bind(&entry.details)
    .bind(&entry.ip_address)
    .bind(&entry.user_agent)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::error!("Failed to write audit log: {e}");
    }
}

/// Convenience: create an AuditEntry for a user action.
pub fn user_action(
    org_id: Uuid,
    user_id: Uuid,
    action: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
    details: Option<serde_json::Value>,
) -> AuditEntry {
    AuditEntry {
        org_id,
        actor_id: Some(user_id),
        api_key_id: None,
        action: action.to_string(),
        resource_type: resource_type.to_string(),
        resource_id,
        details,
        ip_address: None,
        user_agent: None,
    }
}
