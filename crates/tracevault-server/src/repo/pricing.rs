use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, sqlx::FromRow)]
pub struct PricingRow {
    pub id: Uuid,
    pub model: String,
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_read_per_mtok: f64,
    pub cache_write_per_mtok: f64,
    pub effective_from: DateTime<Utc>,
    pub effective_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub source: String,
}

pub struct PricingRepo;

impl PricingRepo {
    /// List all pricing entries, ordered by model then date descending.
    pub async fn list(pool: &PgPool) -> Result<Vec<PricingRow>, AppError> {
        let rows = sqlx::query_as::<_, PricingRow>(
            "SELECT id, model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok,
                    effective_from, effective_until, created_at, source
             FROM model_pricing
             ORDER BY model, effective_from DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(rows)
    }

    /// Distinct model names from sessions for a given org.
    pub async fn list_session_models(pool: &PgPool, org_id: Uuid) -> Result<Vec<String>, AppError> {
        let models = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT model FROM sessions s
             JOIN repos r ON s.repo_id = r.id
             WHERE r.org_id = $1 AND s.model IS NOT NULL
             ORDER BY model",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await?;

        Ok(models)
    }

    /// Create a new pricing entry.
    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        pool: &PgPool,
        model: &str,
        input_per_mtok: f64,
        output_per_mtok: f64,
        cache_read_per_mtok: f64,
        cache_write_per_mtok: f64,
        effective_from: DateTime<Utc>,
        effective_until: Option<DateTime<Utc>>,
    ) -> Result<(Uuid, DateTime<Utc>), AppError> {
        let row = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
            "INSERT INTO model_pricing (model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok, effective_from, effective_until)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, created_at",
        )
        .bind(model)
        .bind(input_per_mtok)
        .bind(output_per_mtok)
        .bind(cache_read_per_mtok)
        .bind(cache_write_per_mtok)
        .bind(effective_from)
        .bind(effective_until)
        .fetch_one(pool)
        .await?;

        Ok(row)
    }

    /// Fetch a single pricing entry by ID.
    pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<PricingRow>, AppError> {
        let row = sqlx::query_as::<_, PricingRow>(
            "SELECT id, model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok,
                    effective_from, effective_until, created_at, source
             FROM model_pricing WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    /// Update a pricing entry (caller resolves COALESCE defaults).
    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        model: &str,
        input_per_mtok: f64,
        output_per_mtok: f64,
        cache_read_per_mtok: f64,
        cache_write_per_mtok: f64,
        effective_from: DateTime<Utc>,
        effective_until: Option<DateTime<Utc>>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE model_pricing
             SET model = $1, input_per_mtok = $2, output_per_mtok = $3,
                 cache_read_per_mtok = $4, cache_write_per_mtok = $5,
                 effective_from = $6, effective_until = $7
             WHERE id = $8",
        )
        .bind(model)
        .bind(input_per_mtok)
        .bind(output_per_mtok)
        .bind(cache_read_per_mtok)
        .bind(cache_write_per_mtok)
        .bind(effective_from)
        .bind(effective_until)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Fetch pricing data needed for session recalculation.
    pub async fn get_for_recalculate(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<
        Option<(
            String,                // model
            f64,                   // input_per_mtok
            f64,                   // output_per_mtok
            f64,                   // cache_read_per_mtok
            f64,                   // cache_write_per_mtok
            DateTime<Utc>,         // effective_from
            Option<DateTime<Utc>>, // effective_until
        )>,
        AppError,
    > {
        let row = sqlx::query_as::<
            _,
            (
                String,
                f64,
                f64,
                f64,
                f64,
                DateTime<Utc>,
                Option<DateTime<Utc>>,
            ),
        >(
            "SELECT model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok,
                    effective_from, effective_until
             FROM model_pricing WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    /// Fetch the last sync timestamp from the sync log.
    pub async fn last_sync_time(pool: &PgPool) -> Result<Option<DateTime<Utc>>, AppError> {
        let row = sqlx::query_scalar::<_, DateTime<Utc>>(
            "SELECT synced_at FROM pricing_sync_log ORDER BY synced_at DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }
}
