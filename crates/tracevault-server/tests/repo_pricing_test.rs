mod common;

use chrono::Utc;
use tracevault_server::repo::pricing::PricingRepo;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn create_and_list(pool: sqlx::PgPool) {
    PricingRepo::create(&pool, "sonnet", 3.0, 15.0, 0.30, 3.75, Utc::now(), None)
        .await
        .unwrap();
    PricingRepo::create(&pool, "opus", 15.0, 75.0, 1.50, 18.75, Utc::now(), None)
        .await
        .unwrap();

    let list = PricingRepo::list(&pool).await.unwrap();
    assert!(list.len() >= 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn get_by_id_found_and_missing(pool: sqlx::PgPool) {
    let (id, _) = PricingRepo::create(&pool, "haiku", 0.80, 4.0, 0.08, 1.0, Utc::now(), None)
        .await
        .unwrap();

    assert!(PricingRepo::get_by_id(&pool, id).await.unwrap().is_some());
    assert!(PricingRepo::get_by_id(&pool, Uuid::new_v4())
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn update_full_replacement(pool: sqlx::PgPool) {
    let (id, _) = PricingRepo::create(&pool, "sonnet", 3.0, 15.0, 0.30, 3.75, Utc::now(), None)
        .await
        .unwrap();

    PricingRepo::update(&pool, id, "sonnet", 4.0, 20.0, 0.40, 4.0, Utc::now(), None)
        .await
        .unwrap();

    let row = PricingRepo::get_by_id(&pool, id).await.unwrap().unwrap();
    assert!((row.input_per_mtok - 4.0).abs() < 0.01);
}

#[sqlx::test(migrations = "./migrations")]
async fn get_for_recalculate(pool: sqlx::PgPool) {
    let (id, _) = PricingRepo::create(&pool, "sonnet", 3.0, 15.0, 0.30, 3.75, Utc::now(), None)
        .await
        .unwrap();

    let result = PricingRepo::get_for_recalculate(&pool, id).await.unwrap();
    assert!(result.is_some());
    let (model, input, output, _, _, _, _) = result.unwrap();
    assert_eq!(model, "sonnet");
    assert!((input - 3.0).abs() < 0.01);
    assert!((output - 15.0).abs() < 0.01);
}

#[sqlx::test(migrations = "./migrations")]
async fn last_sync_time_none_initially(pool: sqlx::PgPool) {
    let result = PricingRepo::last_sync_time(&pool).await.unwrap();
    assert!(result.is_none());
}
