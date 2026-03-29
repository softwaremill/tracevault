mod common;

use serde_json::json;
use tracevault_server::repo::policies::PolicyRepo;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn repo_belongs_to_org_true(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org_id).await;
    assert!(PolicyRepo::repo_belongs_to_org(&pool, repo_id, org_id)
        .await
        .unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn repo_belongs_to_org_false(pool: sqlx::PgPool) {
    let org1 = common::seed_org(&pool).await;
    let org2 = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org1).await;
    assert!(!PolicyRepo::repo_belongs_to_org(&pool, repo_id, org2)
        .await
        .unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn create_and_list_for_repo(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org_id).await;

    PolicyRepo::create(
        &pool,
        org_id,
        repo_id,
        "repo-policy",
        "desc",
        &json!({"type": "TraceCompleteness"}),
        "warn",
        "medium",
        true,
    )
    .await
    .unwrap();

    let policies = PolicyRepo::list_for_repo(&pool, org_id, repo_id)
        .await
        .unwrap();
    assert!(!policies.is_empty());
    assert!(policies.iter().any(|p| p.name == "repo-policy"));
}

#[sqlx::test(migrations = "./migrations")]
async fn update_partial_coalesces(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org_id).await;

    let (id, _, _) = PolicyRepo::create(
        &pool,
        org_id,
        repo_id,
        "original",
        "desc",
        &json!({"type": "TraceCompleteness"}),
        "warn",
        "medium",
        true,
    )
    .await
    .unwrap();

    let updated = PolicyRepo::update(
        &pool,
        id,
        org_id,
        &Some("renamed".into()),
        &None,
        &None,
        &None,
        &None,
        None,
    )
    .await
    .unwrap();

    assert!(updated.is_some());
    let p = updated.unwrap();
    assert_eq!(p.name, "renamed");
    assert_eq!(p.description, "desc");
}

#[sqlx::test(migrations = "./migrations")]
async fn update_nonexistent_returns_none(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let result = PolicyRepo::update(
        &pool,
        Uuid::new_v4(),
        org_id,
        &None,
        &None,
        &None,
        &None,
        &None,
        None,
    )
    .await
    .unwrap();
    assert!(result.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_returns_rows_affected(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org_id).await;

    let (id, _, _) = PolicyRepo::create(
        &pool,
        org_id,
        repo_id,
        "to-delete",
        "d",
        &json!({"type": "TraceCompleteness"}),
        "warn",
        "low",
        true,
    )
    .await
    .unwrap();

    assert_eq!(PolicyRepo::delete(&pool, id, org_id).await.unwrap(), 1);
    assert_eq!(PolicyRepo::delete(&pool, id, org_id).await.unwrap(), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn list_enabled_for_check_filters_disabled(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org_id).await;

    PolicyRepo::create(
        &pool,
        org_id,
        repo_id,
        "enabled-policy",
        "d",
        &json!({"type": "TraceCompleteness"}),
        "warn",
        "medium",
        true,
    )
    .await
    .unwrap();

    PolicyRepo::create(
        &pool,
        org_id,
        repo_id,
        "disabled-policy",
        "d",
        &json!({"type": "TraceCompleteness"}),
        "warn",
        "medium",
        false,
    )
    .await
    .unwrap();

    let enabled = PolicyRepo::list_enabled_for_check(&pool, org_id, repo_id)
        .await
        .unwrap();
    assert!(enabled
        .iter()
        .all(|(name, _, _, _)| name != "disabled-policy"));
    assert!(enabled
        .iter()
        .any(|(name, _, _, _)| name == "enabled-policy"));
}
