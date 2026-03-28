mod common;

use tracevault_server::repo::orgs::OrgRepo;
use tracevault_server::repo::users::UserRepo;

#[sqlx::test(migrations = "./migrations")]
async fn create_and_find_by_email(pool: sqlx::PgPool) {
    let email = format!("user-{}@example.com", uuid::Uuid::new_v4());
    let id = UserRepo::create(&pool, &email, "hash123", Some("Alice"))
        .await
        .unwrap();

    let found = UserRepo::find_by_email(&pool, &email).await.unwrap();
    assert!(found.is_some());
    let found = found.unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.email, email);
    assert_eq!(found.name.as_deref(), Some("Alice"));
}

#[sqlx::test(migrations = "./migrations")]
async fn email_exists(pool: sqlx::PgPool) {
    let email = format!("exists-{}@example.com", uuid::Uuid::new_v4());

    assert!(!UserRepo::email_exists(&pool, &email).await.unwrap());

    UserRepo::create(&pool, &email, "hash", None).await.unwrap();

    assert!(UserRepo::email_exists(&pool, &email).await.unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn find_by_id(pool: sqlx::PgPool) {
    let email = format!("byid-{}@example.com", uuid::Uuid::new_v4());
    let id = UserRepo::create(&pool, &email, "hash", Some("Bob"))
        .await
        .unwrap();

    let info = UserRepo::find_by_id(&pool, id).await.unwrap().unwrap();
    assert_eq!(info.id, id);
    assert_eq!(info.email, email);
    assert_eq!(info.name.as_deref(), Some("Bob"));

    // Non-existent
    let missing = UserRepo::find_by_id(&pool, uuid::Uuid::new_v4())
        .await
        .unwrap();
    assert!(missing.is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn add_org_membership_and_list_orgs(pool: sqlx::PgPool) {
    let user_id = common::seed_user(&pool).await;
    let org_id = common::seed_org(&pool).await;

    UserRepo::add_org_membership(&pool, user_id, org_id, "admin")
        .await
        .unwrap();

    let orgs = UserRepo::list_orgs(&pool, user_id).await.unwrap();
    assert_eq!(orgs.len(), 1);
    assert_eq!(orgs[0].org_id, org_id);
    assert_eq!(orgs[0].role, "admin");
}

#[sqlx::test(migrations = "./migrations")]
async fn get_org_role(pool: sqlx::PgPool) {
    let user_id = common::seed_user(&pool).await;
    let org_id = common::seed_org(&pool).await;

    // No membership yet
    let role = UserRepo::get_org_role(&pool, user_id, org_id)
        .await
        .unwrap();
    assert!(role.is_none());

    UserRepo::add_org_membership(&pool, user_id, org_id, "developer")
        .await
        .unwrap();

    let role = UserRepo::get_org_role(&pool, user_id, org_id)
        .await
        .unwrap();
    assert_eq!(role.as_deref(), Some("developer"));
}

#[sqlx::test(migrations = "./migrations")]
async fn org_create_and_find_by_slug(pool: sqlx::PgPool) {
    let name = format!("org-{}", uuid::Uuid::new_v4());
    let id = OrgRepo::create(&pool, &name, Some("Display Name"))
        .await
        .unwrap();

    let found = OrgRepo::find_by_slug(&pool, &name).await.unwrap();
    assert_eq!(found, Some(id));
}

#[sqlx::test(migrations = "./migrations")]
async fn org_name_exists(pool: sqlx::PgPool) {
    let name = format!("unique-org-{}", uuid::Uuid::new_v4());

    assert!(!OrgRepo::name_exists(&pool, &name).await.unwrap());

    OrgRepo::create(&pool, &name, None).await.unwrap();

    assert!(OrgRepo::name_exists(&pool, &name).await.unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn org_list_members(pool: sqlx::PgPool) {
    let org_id = common::seed_org(&pool).await;
    let u1 = common::seed_user(&pool).await;
    let u2 = common::seed_user(&pool).await;

    common::seed_membership(&pool, u1, org_id, "admin").await;
    common::seed_membership(&pool, u2, org_id, "developer").await;

    let members = OrgRepo::list_members(&pool, org_id).await.unwrap();
    assert_eq!(members.len(), 2);
}
