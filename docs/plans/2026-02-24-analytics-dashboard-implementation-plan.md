# Analytics Dashboard Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add extensive statistics and analytics to TraceVault with 5 backend endpoints, a shared filter bar, and 5 frontend pages (overview + 4 drill-downs) using Chart.js.

**Architecture:** Dedicated API endpoints per analytics page. On-the-fly SQL aggregation from `commits` + `sessions` tables joined through `repos`. Frontend uses SvelteKit with Chart.js via `svelte-chartjs`. Global filter bar (org, repo, author, date range) in a shared layout, with state in URL query params.

**Tech Stack:** Rust/Axum/sqlx (backend), SvelteKit 5 + Chart.js + svelte-chartjs + shadcn-svelte (frontend)

---

### Task 1: Install Chart.js and svelte-chartjs

**Files:**
- Modify: `web/package.json`

**Step 1: Install dependencies**

Run from `web/` directory:
```bash
cd web && pnpm add chart.js svelte-chartjs
```

**Step 2: Verify installation**

Run: `cd web && pnpm build`
Expected: Build succeeds with no errors.

**Step 3: Commit**

```bash
git add web/package.json web/pnpm-lock.yaml
git commit -m "chore: add chart.js and svelte-chartjs dependencies"
```

---

### Task 2: Backend — filters endpoint + common query types

**Files:**
- Modify: `crates/tracevault-server/src/api/analytics.rs` (rewrite)
- Modify: `crates/tracevault-server/src/main.rs:107-111` (add routes)

**Step 1: Rewrite `analytics.rs` with shared query struct and filters endpoint**

Replace the entire content of `crates/tracevault-server/src/api/analytics.rs` with:

```rust
use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::AppState;
use crate::extractors::AuthUser;

// Shared query params for all analytics endpoints
#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub org_id: Option<Uuid>,
    pub repo: Option<String>,
    pub author: Option<String>,
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
}

impl AnalyticsQuery {
    /// Returns the effective org_id, falling back to the auth user's org
    fn effective_org_id(&self, auth: &AuthUser) -> Uuid {
        self.org_id.unwrap_or(auth.org_id)
    }
}

// --- Filters endpoint ---

#[derive(Debug, Serialize)]
pub struct FiltersResponse {
    pub orgs: Vec<OrgOption>,
    pub repos: Vec<RepoOption>,
    pub authors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OrgOption {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct RepoOption {
    pub id: Uuid,
    pub name: String,
}

pub async fn get_filters(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<FiltersResponse>, (StatusCode, String)> {
    // Get orgs the user belongs to
    let orgs = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM orgs WHERE id = $1"
    )
    .bind(auth.org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let org_id = auth.org_id;

    // Get repos for this org
    let repos = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM repos WHERE org_id = $1 ORDER BY name"
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Get distinct authors from commits in this org's repos
    let authors = sqlx::query_as::<_, (String,)>(
        "SELECT DISTINCT c.author FROM commits c
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
         ORDER BY c.author"
    )
    .bind(org_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(FiltersResponse {
        orgs: orgs.into_iter().map(|(id, name)| OrgOption { id, name }).collect(),
        repos: repos.into_iter().map(|(id, name)| RepoOption { id, name }).collect(),
        authors: authors.into_iter().map(|(a,)| a).collect(),
    }))
}
```

**Step 2: Register route in `main.rs`**

In `crates/tracevault-server/src/main.rs`, replace the Analytics section (lines ~107-111):

```rust
        // Analytics
        .route(
            "/api/v1/analytics/filters",
            get(api::analytics::get_filters),
        )
```

**Step 3: Build and verify**

Run: `cargo build -p tracevault-server`
Expected: Compiles with no errors.

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/api/analytics.rs crates/tracevault-server/src/main.rs
git commit -m "feat(api): add analytics filters endpoint with shared query types"
```

---

### Task 3: Backend — overview endpoint

**Files:**
- Modify: `crates/tracevault-server/src/api/analytics.rs` (append)
- Modify: `crates/tracevault-server/src/main.rs` (add route)

**Step 1: Add overview types and handler to `analytics.rs`**

Append to the end of `analytics.rs`:

```rust
// --- Overview endpoint ---

#[derive(Debug, Serialize)]
pub struct OverviewResponse {
    pub total_commits: i64,
    pub total_sessions: i64,
    pub total_tokens: i64,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub active_authors: i64,
    pub estimated_cost_usd: f64,
    pub ai_percentage: Option<f64>,
    pub tokens_over_time: Vec<TimeSeriesPoint>,
    pub top_repos: Vec<RepoTokens>,
    pub model_distribution: Vec<ModelCount>,
    pub recent_commits: Vec<RecentCommit>,
}

#[derive(Debug, Serialize)]
pub struct TimeSeriesPoint {
    pub date: String,
    pub input: i64,
    pub output: i64,
}

#[derive(Debug, Serialize)]
pub struct RepoTokens {
    pub repo: String,
    pub tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct ModelCount {
    pub model: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct RecentCommit {
    pub commit_sha: String,
    pub author: String,
    pub session_count: i64,
    pub total_tokens: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_overview(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<OverviewResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    // KPI: total commits, sessions, tokens, authors, cost
    let kpi = sqlx::query_as::<_, (i64, i64, i64, i64, i64, i64, f64)>(
        "SELECT
            COUNT(DISTINCT c.id),
            COUNT(s.id),
            COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
            COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
            COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0),
            COUNT(DISTINCT c.author),
            COALESCE(SUM(s.estimated_cost_usd), 0.0)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // AI percentage (avg across commits that have attribution)
    let ai_pct = sqlx::query_as::<_, (Option<f64>,)>(
        "SELECT AVG((c.attribution->'summary'->>'ai_percentage')::float)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND c.attribution IS NOT NULL
           AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Tokens over time (daily buckets)
    let tokens_time = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT TO_CHAR(c.created_at::date, 'YYYY-MM-DD'),
                COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.created_at::date
         ORDER BY c.created_at::date"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Top 5 repos by tokens
    let top_repos = sqlx::query_as::<_, (String, i64)>(
        "SELECT r.name, COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR c.author = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
         GROUP BY r.name
         ORDER BY 2 DESC
         LIMIT 5"
    )
    .bind(org_id)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Model distribution
    let models = sqlx::query_as::<_, (String, i64)>(
        "SELECT COALESCE(s.model, 'unknown'), COUNT(*)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY s.model
         ORDER BY 2 DESC"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Recent 10 commits
    let recent = sqlx::query_as::<_, (String, String, i64, i64, chrono::DateTime<chrono::Utc>)>(
        "SELECT c.commit_sha, c.author, COUNT(s.id), COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0), c.created_at
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.id
         ORDER BY c.created_at DESC
         LIMIT 10"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(OverviewResponse {
        total_commits: kpi.0,
        total_sessions: kpi.1,
        total_tokens: kpi.2,
        total_input_tokens: kpi.3,
        total_output_tokens: kpi.4,
        active_authors: kpi.5,
        estimated_cost_usd: kpi.6,
        ai_percentage: ai_pct.0,
        tokens_over_time: tokens_time.into_iter().map(|(d, i, o)| TimeSeriesPoint { date: d, input: i, output: o }).collect(),
        top_repos: top_repos.into_iter().map(|(r, t)| RepoTokens { repo: r, tokens: t }).collect(),
        model_distribution: models.into_iter().map(|(m, c)| ModelCount { model: m, count: c }).collect(),
        recent_commits: recent.into_iter().map(|(sha, author, sc, tt, ca)| RecentCommit {
            commit_sha: sha, author, session_count: sc, total_tokens: tt, created_at: ca
        }).collect(),
    }))
}
```

**Step 2: Add route in `main.rs`**

Add after the filters route:

```rust
        .route(
            "/api/v1/analytics/overview",
            get(api::analytics::get_overview),
        )
```

**Step 3: Build and verify**

Run: `cargo build -p tracevault-server`
Expected: Compiles with no errors.

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/api/analytics.rs crates/tracevault-server/src/main.rs
git commit -m "feat(api): add analytics overview endpoint with KPIs and charts"
```

---

### Task 4: Backend — tokens endpoint

**Files:**
- Modify: `crates/tracevault-server/src/api/analytics.rs` (append)
- Modify: `crates/tracevault-server/src/main.rs` (add route)

**Step 1: Add tokens types and handler**

Append to `analytics.rs`:

```rust
// --- Tokens endpoint ---

#[derive(Debug, Serialize)]
pub struct TokensResponse {
    pub time_series: Vec<TokenTimePoint>,
    pub by_repo: Vec<RepoTokenDetail>,
    pub by_author: Vec<AuthorTokens>,
}

#[derive(Debug, Serialize)]
pub struct TokenTimePoint {
    pub date: String,
    pub input: i64,
    pub output: i64,
}

#[derive(Debug, Serialize)]
pub struct RepoTokenDetail {
    pub repo: String,
    pub total: i64,
    pub input: i64,
    pub output: i64,
    pub sessions: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthorTokens {
    pub author: String,
    pub total: i64,
}

pub async fn get_tokens(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<TokensResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    let time_series = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT TO_CHAR(c.created_at::date, 'YYYY-MM-DD'),
                COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.created_at::date
         ORDER BY c.created_at::date"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_repo = sqlx::query_as::<_, (String, i64, i64, i64, i64)>(
        "SELECT r.name,
                COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.input_tokens) AS BIGINT), 0),
                COALESCE(CAST(SUM(s.output_tokens) AS BIGINT), 0),
                COUNT(s.id)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR c.author = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
         GROUP BY r.name
         ORDER BY 2 DESC"
    )
    .bind(org_id)
    .bind(&q.author)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_author = sqlx::query_as::<_, (String, i64)>(
        "SELECT c.author, COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
         GROUP BY c.author
         ORDER BY 2 DESC"
    )
    .bind(org_id)
    .bind(&q.repo)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(TokensResponse {
        time_series: time_series.into_iter().map(|(d, i, o)| TokenTimePoint { date: d, input: i, output: o }).collect(),
        by_repo: by_repo.into_iter().map(|(r, t, i, o, s)| RepoTokenDetail { repo: r, total: t, input: i, output: o, sessions: s }).collect(),
        by_author: by_author.into_iter().map(|(a, t)| AuthorTokens { author: a, total: t }).collect(),
    }))
}
```

**Step 2: Add route in `main.rs`**

```rust
        .route(
            "/api/v1/analytics/tokens",
            get(api::analytics::get_tokens),
        )
```

Remove the old `token_analytics` route that this replaces.

**Step 3: Build and verify**

Run: `cargo build -p tracevault-server`

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/api/analytics.rs crates/tracevault-server/src/main.rs
git commit -m "feat(api): add analytics tokens endpoint"
```

---

### Task 5: Backend — models endpoint

**Files:**
- Modify: `crates/tracevault-server/src/api/analytics.rs` (append)
- Modify: `crates/tracevault-server/src/main.rs` (add route)

**Step 1: Add models types and handler**

Append to `analytics.rs`:

```rust
// --- Models endpoint ---

#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub distribution: Vec<ModelDistribution>,
    pub trends: Vec<ModelTrend>,
    pub author_model_matrix: Vec<AuthorModel>,
    pub comparison: Vec<ModelComparison>,
}

#[derive(Debug, Serialize)]
pub struct ModelDistribution {
    pub model: String,
    pub session_count: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct ModelTrend {
    pub date: String,
    pub model: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthorModel {
    pub author: String,
    pub model: String,
    pub sessions: i64,
    pub tokens: i64,
}

#[derive(Debug, Serialize)]
pub struct ModelComparison {
    pub model: String,
    pub avg_tokens: i64,
    pub avg_cost: f64,
}

pub async fn get_models(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<ModelsResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    let distribution = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT COALESCE(s.model, 'unknown'), COUNT(*), COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY s.model
         ORDER BY 2 DESC"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let trends = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT TO_CHAR(c.created_at::date, 'YYYY-MM-DD'), COALESCE(s.model, 'unknown'), COUNT(*)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.created_at::date, s.model
         ORDER BY 1, 2"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let author_model_matrix = sqlx::query_as::<_, (String, String, i64, i64)>(
        "SELECT c.author, COALESCE(s.model, 'unknown'), COUNT(*), COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.author, s.model
         ORDER BY c.author, 3 DESC"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let comparison = sqlx::query_as::<_, (String, i64, f64)>(
        "SELECT COALESCE(s.model, 'unknown'),
                COALESCE(CAST(AVG(s.total_tokens) AS BIGINT), 0),
                COALESCE(AVG(s.estimated_cost_usd), 0.0)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY s.model
         ORDER BY 2 DESC"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ModelsResponse {
        distribution: distribution.into_iter().map(|(m, c, t)| ModelDistribution { model: m, session_count: c, total_tokens: t }).collect(),
        trends: trends.into_iter().map(|(d, m, c)| ModelTrend { date: d, model: m, count: c }).collect(),
        author_model_matrix: author_model_matrix.into_iter().map(|(a, m, s, t)| AuthorModel { author: a, model: m, sessions: s, tokens: t }).collect(),
        comparison: comparison.into_iter().map(|(m, t, c)| ModelComparison { model: m, avg_tokens: t, avg_cost: c }).collect(),
    }))
}
```

**Step 2: Add route**

```rust
        .route(
            "/api/v1/analytics/models",
            get(api::analytics::get_models),
        )
```

**Step 3: Build and verify**

Run: `cargo build -p tracevault-server`

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/api/analytics.rs crates/tracevault-server/src/main.rs
git commit -m "feat(api): add analytics models endpoint"
```

---

### Task 6: Backend — authors endpoint

**Files:**
- Modify: `crates/tracevault-server/src/api/analytics.rs` (append)
- Modify: `crates/tracevault-server/src/main.rs` (add route)

**Step 1: Add authors types and handler**

Append to `analytics.rs`:

```rust
// --- Authors endpoint ---

#[derive(Debug, Serialize)]
pub struct AuthorsResponse {
    pub leaderboard: Vec<AuthorLeaderboard>,
    pub timeline: Vec<AuthorTimeline>,
    pub model_preferences: Vec<AuthorModelPreference>,
}

#[derive(Debug, Serialize)]
pub struct AuthorLeaderboard {
    pub author: String,
    pub commits: i64,
    pub sessions: i64,
    pub tokens: i64,
    pub cost: f64,
    pub ai_pct: Option<f64>,
    pub last_active: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct AuthorTimeline {
    pub date: String,
    pub author: String,
    pub commits: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthorModelPreference {
    pub author: String,
    pub model: String,
    pub sessions: i64,
}

pub async fn get_authors(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<AuthorsResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    let leaderboard = sqlx::query_as::<_, (String, i64, i64, i64, f64, Option<f64>, chrono::DateTime<chrono::Utc>)>(
        "SELECT c.author,
                COUNT(DISTINCT c.id),
                COUNT(s.id),
                COALESCE(CAST(SUM(s.total_tokens) AS BIGINT), 0),
                COALESCE(SUM(s.estimated_cost_usd), 0.0),
                AVG(CASE WHEN c.attribution IS NOT NULL AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL
                    THEN (c.attribution->'summary'->>'ai_percentage')::float END),
                MAX(c.created_at)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         LEFT JOIN sessions s ON s.commit_id = c.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
         GROUP BY c.author
         ORDER BY 4 DESC"
    )
    .bind(org_id).bind(&q.repo).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let timeline = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT TO_CHAR(c.created_at::date, 'YYYY-MM-DD'), c.author, COUNT(DISTINCT c.id)
         FROM commits c
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.created_at::date, c.author
         ORDER BY 1, 2"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let model_preferences = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT c.author, COALESCE(s.model, 'unknown'), COUNT(*)
         FROM sessions s
         JOIN commits c ON s.commit_id = c.id
         JOIN repos r ON c.repo_id = r.id
         WHERE r.org_id = $1
           AND ($2::TEXT IS NULL OR r.name = $2)
           AND ($3::TEXT IS NULL OR c.author = $3)
           AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
           AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
         GROUP BY c.author, s.model
         ORDER BY c.author, 3 DESC"
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AuthorsResponse {
        leaderboard: leaderboard.into_iter().map(|(a, c, s, t, cost, ai, la)| AuthorLeaderboard {
            author: a, commits: c, sessions: s, tokens: t, cost, ai_pct: ai, last_active: la
        }).collect(),
        timeline: timeline.into_iter().map(|(d, a, c)| AuthorTimeline { date: d, author: a, commits: c }).collect(),
        model_preferences: model_preferences.into_iter().map(|(a, m, s)| AuthorModelPreference { author: a, model: m, sessions: s }).collect(),
    }))
}
```

**Step 2: Add route**

```rust
        .route(
            "/api/v1/analytics/authors",
            get(api::analytics::get_authors),
        )
```

**Step 3: Build and verify**

Run: `cargo build -p tracevault-server`

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/api/analytics.rs crates/tracevault-server/src/main.rs
git commit -m "feat(api): add analytics authors endpoint"
```

---

### Task 7: Backend — attribution endpoint

**Files:**
- Modify: `crates/tracevault-server/src/api/analytics.rs` (append)
- Modify: `crates/tracevault-server/src/main.rs` (add route)

**Step 1: Add attribution types and handler**

Append to `analytics.rs`:

```rust
// --- Attribution endpoint ---

#[derive(Debug, Serialize)]
pub struct AttributionResponse {
    pub trend: Vec<AttributionTrend>,
    pub by_repo: Vec<RepoAttribution>,
    pub by_author: Vec<AuthorAttribution>,
    pub totals: AttributionTotals,
}

#[derive(Debug, Serialize)]
pub struct AttributionTrend {
    pub date: String,
    pub ai_pct: f64,
    pub human_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct RepoAttribution {
    pub repo: String,
    pub ai_pct: f64,
    pub ai_lines: i64,
    pub human_lines: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthorAttribution {
    pub author: String,
    pub ai_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct AttributionTotals {
    pub ai_lines: i64,
    pub human_lines: i64,
    pub ai_pct: f64,
}

pub async fn get_attribution(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(q): Query<AnalyticsQuery>,
) -> Result<Json<AttributionResponse>, (StatusCode, String)> {
    let org_id = q.effective_org_id(&auth);

    // Only include commits that have attribution data
    let base_filter = "c.attribution IS NOT NULL AND c.attribution->'summary'->>'ai_percentage' IS NOT NULL";

    let trend = sqlx::query_as::<_, (String, f64)>(
        &format!(
            "SELECT TO_CHAR(c.created_at::date, 'YYYY-MM-DD'),
                    AVG((c.attribution->'summary'->>'ai_percentage')::float)
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1 AND {base_filter}
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR c.author = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)
             GROUP BY c.created_at::date
             ORDER BY 1"
        )
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_repo = sqlx::query_as::<_, (String, f64, i64, i64)>(
        &format!(
            "SELECT r.name,
                    AVG((c.attribution->'summary'->>'ai_percentage')::float),
                    COALESCE(CAST(SUM((c.attribution->'summary'->>'total_lines_added')::bigint) AS BIGINT), 0),
                    COALESCE(CAST(SUM((c.attribution->'summary'->>'total_lines_deleted')::bigint) AS BIGINT), 0)
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1 AND {base_filter}
               AND ($2::TEXT IS NULL OR c.author = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
             GROUP BY r.name
             ORDER BY 2 DESC"
        )
    )
    .bind(org_id).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let by_author = sqlx::query_as::<_, (String, f64)>(
        &format!(
            "SELECT c.author, AVG((c.attribution->'summary'->>'ai_percentage')::float)
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1 AND {base_filter}
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TIMESTAMPTZ IS NULL OR c.created_at >= $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at <= $4)
             GROUP BY c.author
             ORDER BY 2 DESC"
        )
    )
    .bind(org_id).bind(&q.repo).bind(q.from).bind(q.to)
    .fetch_all(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let totals = sqlx::query_as::<_, (i64, i64, f64)>(
        &format!(
            "SELECT
                COALESCE(CAST(SUM((c.attribution->'summary'->>'total_lines_added')::bigint) AS BIGINT), 0),
                COALESCE(CAST(SUM((c.attribution->'summary'->>'total_lines_deleted')::bigint) AS BIGINT), 0),
                COALESCE(AVG((c.attribution->'summary'->>'ai_percentage')::float), 0.0)
             FROM commits c
             JOIN repos r ON c.repo_id = r.id
             WHERE r.org_id = $1 AND {base_filter}
               AND ($2::TEXT IS NULL OR r.name = $2)
               AND ($3::TEXT IS NULL OR c.author = $3)
               AND ($4::TIMESTAMPTZ IS NULL OR c.created_at >= $4)
               AND ($5::TIMESTAMPTZ IS NULL OR c.created_at <= $5)"
        )
    )
    .bind(org_id).bind(&q.repo).bind(&q.author).bind(q.from).bind(q.to)
    .fetch_one(&state.pool).await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AttributionResponse {
        trend: trend.into_iter().map(|(d, ai)| AttributionTrend { date: d, ai_pct: ai, human_pct: 100.0 - ai }).collect(),
        by_repo: by_repo.into_iter().map(|(r, ai, al, hl)| RepoAttribution { repo: r, ai_pct: ai, ai_lines: al, human_lines: hl }).collect(),
        by_author: by_author.into_iter().map(|(a, ai)| AuthorAttribution { author: a, ai_pct: ai }).collect(),
        totals: AttributionTotals { ai_lines: totals.0, human_lines: totals.1, ai_pct: totals.2 },
    }))
}
```

**Step 2: Add route**

```rust
        .route(
            "/api/v1/analytics/attribution",
            get(api::analytics::get_attribution),
        )
```

**Step 3: Build and verify**

Run: `cargo build -p tracevault-server`

**Step 4: Commit**

```bash
git add crates/tracevault-server/src/api/analytics.rs crates/tracevault-server/src/main.rs
git commit -m "feat(api): add analytics attribution endpoint"
```

---

### Task 8: Frontend — sidebar nav + analytics layout with filter bar

**Files:**
- Modify: `web/src/lib/components/app-sidebar.svelte:13-17` (add nav item)
- Create: `web/src/routes/analytics/+layout.svelte`
- Create: `web/src/lib/components/analytics-filters.svelte`

**Step 1: Add "Analytics" to sidebar nav**

In `web/src/lib/components/app-sidebar.svelte`, add Analytics between Traces and Settings in the `navItems` array:

```typescript
const navItems = [
    { href: '/repos', label: 'Repos' },
    { href: '/traces', label: 'Traces' },
    { href: '/analytics', label: 'Analytics' },
    { href: '/settings', label: 'Settings' }
];
```

**Step 2: Create analytics filter bar component**

Create `web/src/lib/components/analytics-filters.svelte`:

```svelte
<script lang="ts">
    import { page } from '$app/stores';
    import { goto } from '$app/navigation';
    import { onMount } from 'svelte';
    import { api } from '$lib/api';
    import * as Select from '$lib/components/ui/select/index.js';
    import { Button } from '$lib/components/ui/button/index.js';
    import { Input } from '$lib/components/ui/input/index.js';

    interface FilterOptions {
        orgs: { id: string; name: string }[];
        repos: { id: string; name: string }[];
        authors: string[];
    }

    let filters: FilterOptions = $state({ orgs: [], repos: [], authors: [] });
    let selectedOrg = $state('');
    let selectedRepo = $state('');
    let selectedAuthor = $state('');
    let dateFrom = $state('');
    let dateTo = $state('');
    let activePreset = $state('30d');

    onMount(async () => {
        try {
            filters = await api.get<FilterOptions>('/api/v1/analytics/filters');
        } catch {
            // filters stay empty
        }
        // Read initial state from URL
        const params = $page.url.searchParams;
        selectedOrg = params.get('org_id') ?? '';
        selectedRepo = params.get('repo') ?? '';
        selectedAuthor = params.get('author') ?? '';
        dateFrom = params.get('from') ?? '';
        dateTo = params.get('to') ?? '';
        if (dateFrom || dateTo) activePreset = '';
        else if (!params.has('from')) applyPreset('30d');
    });

    function applyPreset(preset: string) {
        activePreset = preset;
        const now = new Date();
        let from = '';
        if (preset === '7d') {
            from = new Date(now.getTime() - 7 * 86400000).toISOString();
        } else if (preset === '30d') {
            from = new Date(now.getTime() - 30 * 86400000).toISOString();
        } else if (preset === '90d') {
            from = new Date(now.getTime() - 90 * 86400000).toISOString();
        }
        // 'all' means no from/to
        dateFrom = from;
        dateTo = '';
        updateUrl();
    }

    function updateUrl() {
        const params = new URLSearchParams();
        if (selectedOrg) params.set('org_id', selectedOrg);
        if (selectedRepo) params.set('repo', selectedRepo);
        if (selectedAuthor) params.set('author', selectedAuthor);
        if (dateFrom) params.set('from', dateFrom);
        if (dateTo) params.set('to', dateTo);
        const qs = params.toString();
        const path = $page.url.pathname;
        goto(`${path}${qs ? '?' + qs : ''}`, { replaceState: true, keepFocus: true });
    }

    function onOrgChange(value: string | undefined) {
        selectedOrg = value ?? '';
        updateUrl();
    }

    function onRepoChange(value: string | undefined) {
        selectedRepo = value ?? '';
        updateUrl();
    }

    function onAuthorChange(value: string | undefined) {
        selectedAuthor = value ?? '';
        updateUrl();
    }

    function onDateFromChange(e: Event) {
        dateFrom = (e.target as HTMLInputElement).value ? new Date((e.target as HTMLInputElement).value).toISOString() : '';
        activePreset = '';
        updateUrl();
    }

    function onDateToChange(e: Event) {
        dateTo = (e.target as HTMLInputElement).value ? new Date((e.target as HTMLInputElement).value).toISOString() : '';
        activePreset = '';
        updateUrl();
    }

    export function getQueryString(): string {
        const params = $page.url.searchParams;
        return params.toString() ? '?' + params.toString() : '';
    }
</script>

<div class="flex flex-wrap items-center gap-3 rounded-lg border bg-card p-3">
    {#if filters.orgs.length > 1}
        <Select.Root onValueChange={onOrgChange}>
            <Select.Trigger class="w-[160px]">
                {selectedOrg ? filters.orgs.find(o => o.id === selectedOrg)?.name ?? 'Org' : 'All orgs'}
            </Select.Trigger>
            <Select.Content>
                <Select.Item value="">All orgs</Select.Item>
                {#each filters.orgs as org}
                    <Select.Item value={org.id}>{org.name}</Select.Item>
                {/each}
            </Select.Content>
        </Select.Root>
    {/if}

    <Select.Root onValueChange={onRepoChange}>
        <Select.Trigger class="w-[160px]">
            {selectedRepo || 'All repos'}
        </Select.Trigger>
        <Select.Content>
            <Select.Item value="">All repos</Select.Item>
            {#each filters.repos as repo}
                <Select.Item value={repo.name}>{repo.name}</Select.Item>
            {/each}
        </Select.Content>
    </Select.Root>

    <Select.Root onValueChange={onAuthorChange}>
        <Select.Trigger class="w-[160px]">
            {selectedAuthor || 'All authors'}
        </Select.Trigger>
        <Select.Content>
            <Select.Item value="">All authors</Select.Item>
            {#each filters.authors as author}
                <Select.Item value={author}>{author}</Select.Item>
            {/each}
        </Select.Content>
    </Select.Root>

    <div class="flex items-center gap-1 ml-auto">
        {#each ['7d', '30d', '90d', 'all'] as preset}
            <Button
                variant={activePreset === preset ? 'default' : 'outline'}
                size="sm"
                onclick={() => applyPreset(preset)}
            >
                {preset === 'all' ? 'All' : preset}
            </Button>
        {/each}
    </div>

    <Input type="date" class="w-[140px]" onchange={onDateFromChange} placeholder="From" />
    <Input type="date" class="w-[140px]" onchange={onDateToChange} placeholder="To" />
</div>
```

**Step 3: Create analytics layout**

Create `web/src/routes/analytics/+layout.svelte`:

```svelte
<script lang="ts">
    import AppLayout from '$lib/components/app-layout.svelte';
    import AnalyticsFilters from '$lib/components/analytics-filters.svelte';
    let { children } = $props();
</script>

<AppLayout>
    <div class="space-y-4">
        <AnalyticsFilters />
        {@render children()}
    </div>
</AppLayout>
```

**Step 4: Verify**

Run: `cd web && npx svelte-check --threshold error`
Expected: 0 errors

**Step 5: Commit**

```bash
git add web/src/lib/components/app-sidebar.svelte web/src/lib/components/analytics-filters.svelte web/src/routes/analytics/+layout.svelte
git commit -m "feat(web): add analytics sidebar nav and filter bar layout"
```

---

### Task 9: Frontend — overview page

**Files:**
- Create: `web/src/routes/analytics/+page.svelte`

**Step 1: Create the overview page**

Create `web/src/routes/analytics/+page.svelte` with:
- 6 KPI cards in a grid (commits, sessions, tokens, authors, AI%, cost)
- Line chart for tokens over time (using `svelte-chartjs` `Line` component)
- Horizontal bar chart for top repos
- Doughnut chart for model distribution
- Recent commits table

The page should:
- Read filters from `$page.url.searchParams` to build the API query string
- Call `api.get<OverviewResponse>('/api/v1/analytics/overview' + queryString)`
- Re-fetch when URL params change (use `$effect` watching `$page.url.searchParams`)
- Use Card components from shadcn for each section
- Use Badge for model names
- Each chart section links to its drill-down page

Charts should use these Chart.js colors:
- Input tokens: `hsl(221, 83%, 53%)` (blue)
- Output tokens: `hsl(142, 71%, 45%)` (green)
- Consistent palette for model colors: `['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16']`

Chart.js must be registered via `import { Chart, ... } from 'chart.js'` and `Chart.register(...)` at the top level.

**Step 2: Verify**

Run: `cd web && npx svelte-check --threshold error`

**Step 3: Commit**

```bash
git add web/src/routes/analytics/+page.svelte
git commit -m "feat(web): add analytics overview page with KPIs and charts"
```

---

### Task 10: Frontend — tokens drill-down page

**Files:**
- Create: `web/src/routes/analytics/tokens/+page.svelte`

**Step 1: Create the tokens page**

Create `web/src/routes/analytics/tokens/+page.svelte` with:
- Line chart for token time series (input, output as separate lines)
- Table for by-repo breakdown (sortable by clicking column headers)
- Bar chart for by-author token usage

Fetch from: `api.get<TokensResponse>('/api/v1/analytics/tokens' + queryString)`

Use the same Chart.js registration and color scheme as the overview page.

Table should use shadcn Table components with columns: Repo, Total Tokens, Input, Output, Sessions, Avg/Session.

**Step 2: Verify**

Run: `cd web && npx svelte-check --threshold error`

**Step 3: Commit**

```bash
git add web/src/routes/analytics/tokens/+page.svelte
git commit -m "feat(web): add analytics tokens drill-down page"
```

---

### Task 11: Frontend — models drill-down page

**Files:**
- Create: `web/src/routes/analytics/models/+page.svelte`

**Step 1: Create the models page**

Create `web/src/routes/analytics/models/+page.svelte` with:
- Doughnut chart for model distribution (session count)
- Stacked area chart for model trends over time (group by model, stack by date)
- Table for author x model matrix
- Bar chart for model comparison (avg tokens, avg cost)

Fetch from: `api.get<ModelsResponse>('/api/v1/analytics/models' + queryString)`

**Step 2: Verify**

Run: `cd web && npx svelte-check --threshold error`

**Step 3: Commit**

```bash
git add web/src/routes/analytics/models/+page.svelte
git commit -m "feat(web): add analytics models drill-down page"
```

---

### Task 12: Frontend — authors drill-down page

**Files:**
- Create: `web/src/routes/analytics/authors/+page.svelte`

**Step 1: Create the authors page**

Create `web/src/routes/analytics/authors/+page.svelte` with:
- Leaderboard table (author, commits, sessions, tokens, cost, AI%, last active)
- Line chart for author activity timeline
- Per-author model preference display (badges or small doughnut per top 3 authors)

Fetch from: `api.get<AuthorsResponse>('/api/v1/analytics/authors' + queryString)`

**Step 2: Verify**

Run: `cd web && npx svelte-check --threshold error`

**Step 3: Commit**

```bash
git add web/src/routes/analytics/authors/+page.svelte
git commit -m "feat(web): add analytics authors drill-down page"
```

---

### Task 13: Frontend — attribution drill-down page

**Files:**
- Create: `web/src/routes/analytics/attribution/+page.svelte`

**Step 1: Create the attribution page**

Create `web/src/routes/analytics/attribution/+page.svelte` with:
- Stacked area chart for AI vs Human trend over time (ai_pct + human_pct = 100%)
- Horizontal bar chart for AI% per repo
- Horizontal bar chart for AI% per author
- Summary card with total AI lines, human lines, overall AI%

Fetch from: `api.get<AttributionResponse>('/api/v1/analytics/attribution' + queryString)`

Use purple for AI (`hsl(262, 83%, 58%)`) and green for human (`hsl(142, 71%, 45%)`).

**Step 2: Verify**

Run: `cd web && npx svelte-check --threshold error`

**Step 3: Commit**

```bash
git add web/src/routes/analytics/attribution/+page.svelte
git commit -m "feat(web): add analytics attribution drill-down page"
```

---

### Task 14: Final build verification

**Step 1: Build backend**

Run: `cargo build -p tracevault-server`
Expected: Compiles with no warnings.

**Step 2: Build frontend**

Run: `cd web && pnpm build`
Expected: Build succeeds.

**Step 3: Run svelte-check**

Run: `cd web && npx svelte-check --threshold error`
Expected: 0 errors.

**Step 4: Run core tests**

Run: `cargo test -p tracevault-core`
Expected: All tests pass.

**Step 5: Commit any remaining changes and push**

```bash
git push
```
