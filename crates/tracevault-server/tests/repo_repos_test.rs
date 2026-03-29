mod common;

use tracevault_server::repo::repos::GitRepoRepo;

#[sqlx::test(migrations = "./migrations")]
async fn create_idempotent(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;

    let id1 = GitRepoRepo::create(&pool, org_id, "my-repo", None)
        .await
        .unwrap();
    let id2 = GitRepoRepo::create(
        &pool,
        org_id,
        "my-repo",
        Some("https://github.com/org/repo"),
    )
    .await
    .unwrap();

    assert_eq!(id1, id2);
}

#[sqlx::test(migrations = "./migrations")]
async fn create_updates_url_on_conflict(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    GitRepoRepo::create(&pool, org_id, "repo-url", None)
        .await
        .unwrap();
    GitRepoRepo::create(&pool, org_id, "repo-url", Some("https://new-url.com"))
        .await
        .unwrap();

    let repos = GitRepoRepo::list(&pool, org_id).await.unwrap();
    let repo = repos.iter().find(|r| r.name == "repo-url").unwrap();
    assert_eq!(repo.github_url.as_deref(), Some("https://new-url.com"));
}

#[sqlx::test(migrations = "./migrations")]
async fn list_ordered_by_name(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    GitRepoRepo::create(&pool, org_id, "z-repo", None)
        .await
        .unwrap();
    GitRepoRepo::create(&pool, org_id, "a-repo", None)
        .await
        .unwrap();

    let repos = GitRepoRepo::list(&pool, org_id).await.unwrap();
    assert!(repos[0].name < repos[1].name);
}

#[sqlx::test(migrations = "./migrations")]
async fn set_clone_status_and_list_ready(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = GitRepoRepo::create(&pool, org_id, "sync-repo", None)
        .await
        .unwrap();

    let ready = GitRepoRepo::list_ready_for_sync(&pool).await.unwrap();
    assert!(!ready.iter().any(|r| r.id == repo_id));

    GitRepoRepo::set_clone_status(&pool, repo_id, "ready", Some("/data/repos/123"))
        .await
        .unwrap();

    let ready = GitRepoRepo::list_ready_for_sync(&pool).await.unwrap();
    assert!(ready.iter().any(|r| r.id == repo_id));
}

#[sqlx::test(migrations = "./migrations")]
async fn mark_fetched_updates_timestamp(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let repo_id = GitRepoRepo::create(&pool, org_id, "fetch-repo", None)
        .await
        .unwrap();

    GitRepoRepo::mark_fetched(&pool, repo_id).await.unwrap();

    let (ts,): (Option<chrono::DateTime<chrono::Utc>>,) =
        sqlx::query_as("SELECT last_fetched_at FROM repos WHERE id = $1")
            .bind(repo_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(ts.is_some());
}
