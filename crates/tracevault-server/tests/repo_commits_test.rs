mod common;

use chrono::Utc;
use tracevault_server::repo::commits::{CommitRepo, InsertAttribution, UpsertCommit};
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn upsert_creates_commit(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org_id).await;
    let id = common::seed_commit(&pool, repo_id, "abc123").await;
    assert_ne!(id, Uuid::nil());
}

#[sqlx::test(migrations = "./migrations")]
async fn upsert_conflict_coalesces(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org_id).await;

    let id1 = CommitRepo::upsert(
        &pool,
        &UpsertCommit {
            repo_id,
            commit_sha: "abc123".into(),
            branch: Some("main".into()),
            author: "dev@test.com".into(),
            message: None,
            diff_data: None,
            committed_at: Some(Utc::now()),
        },
    )
    .await
    .unwrap();

    let id2 = CommitRepo::upsert(
        &pool,
        &UpsertCommit {
            repo_id,
            commit_sha: "abc123".into(),
            branch: None,
            author: "dev@test.com".into(),
            message: Some("updated msg".into()),
            diff_data: None,
            committed_at: Some(Utc::now()),
        },
    )
    .await
    .unwrap();

    assert_eq!(id1, id2);

    let (branch, message): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT branch, message FROM commits WHERE id = $1")
            .bind(id1)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(branch.as_deref(), Some("main"));
    assert_eq!(message.as_deref(), Some("updated msg"));
}

#[sqlx::test(migrations = "./migrations")]
async fn attribution_workflow(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org_id).await;
    let user_id = common::seed_user(&pool).await;
    let commit_id = common::seed_commit(&pool, repo_id, "def456").await;
    let session_id = common::seed_session(&pool, org_id, repo_id, user_id).await;
    let event_id = common::seed_event(&pool, session_id, 0).await;

    CommitRepo::clear_attributions(&pool, commit_id)
        .await
        .unwrap();

    CommitRepo::insert_attribution(
        &pool,
        &InsertAttribution {
            commit_id,
            session_id,
            event_id,
            file_path: "src/main.rs".into(),
            line_start: Some(1),
            line_end: Some(10),
            confidence: 0.8,
        },
    )
    .await
    .unwrap();

    let attrs = CommitRepo::get_attributions(&pool, commit_id)
        .await
        .unwrap();
    assert_eq!(attrs.len(), 1);
    assert_eq!(attrs[0].file_path, "src/main.rs");
}

#[sqlx::test(migrations = "./migrations")]
async fn update_attribution_summary(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = common::seed_repo(&pool, org_id).await;
    let commit_id = common::seed_commit(&pool, repo_id, "ghi789").await;

    let summary = serde_json::json!({"ai_percentage": 42.0, "human_percentage": 58.0});
    CommitRepo::update_attribution_summary(&pool, commit_id, &summary)
        .await
        .unwrap();

    let (attr,): (Option<serde_json::Value>,) =
        sqlx::query_as("SELECT attribution FROM commits WHERE id = $1")
            .bind(commit_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(attr.unwrap()["ai_percentage"], 42.0);
}
