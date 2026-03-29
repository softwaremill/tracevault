mod common;

use tracevault_server::repo::api_keys::ApiKeyRepo;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn create_returns_uuid(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let id = ApiKeyRepo::create(&pool, org_id, "hash_abc12345678", "my-key")
        .await
        .unwrap();
    assert_ne!(id, Uuid::nil());
}

#[sqlx::test(migrations = "./migrations")]
async fn list_returns_prefix_and_ordered(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    ApiKeyRepo::create(&pool, org_id, "hash_first_key00", "key-a")
        .await
        .unwrap();
    ApiKeyRepo::create(&pool, org_id, "hash_second_key0", "key-b")
        .await
        .unwrap();

    let keys = ApiKeyRepo::list(&pool, org_id).await.unwrap();
    assert_eq!(keys.len(), 2);
    assert_eq!(keys[0].key_prefix, "hash_fir");
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_with_correct_org(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let id = ApiKeyRepo::create(&pool, org_id, "hash_del_test00", "del-key")
        .await
        .unwrap();
    assert!(ApiKeyRepo::delete(&pool, id, org_id).await.unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_with_wrong_org(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let other_org = common::seed_org(&pool).await;
    let id = ApiKeyRepo::create(&pool, org_id, "hash_wrong_org0", "wrong-key")
        .await
        .unwrap();
    assert!(!ApiKeyRepo::delete(&pool, id, other_org).await.unwrap());
}
