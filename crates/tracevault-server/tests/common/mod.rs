use sqlx::PgPool;
use uuid::Uuid;

pub async fn seed_org(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO orgs (name) VALUES ('test-org-' || gen_random_uuid()::text) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

pub async fn seed_user(pool: &PgPool) -> Uuid {
    let email = format!("test-{}@example.com", Uuid::new_v4());
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name) \
         VALUES ($1, '$argon2id$v=19$m=19456,t=2,p=1$fake_salt$fake_hash', 'Test User') \
         RETURNING id",
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[allow(dead_code)]
pub async fn seed_repo(pool: &PgPool, org_id: Uuid) -> Uuid {
    let name = format!("test-repo-{}", Uuid::new_v4());
    sqlx::query_scalar::<_, Uuid>("INSERT INTO repos (org_id, name) VALUES ($1, $2) RETURNING id")
        .bind(org_id)
        .bind(&name)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[allow(dead_code)]
pub async fn seed_membership(pool: &PgPool, user_id: Uuid, org_id: Uuid, role: &str) {
    sqlx::query("INSERT INTO user_org_memberships (user_id, org_id, role) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(org_id)
        .bind(role)
        .execute(pool)
        .await
        .unwrap();
}
