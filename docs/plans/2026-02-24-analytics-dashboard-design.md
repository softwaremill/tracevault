# Analytics Dashboard Design

## Overview

Add extensive statistics and analytics to TraceVault. Users can browse stats by organization, repository, and author with interactive charts and time-range filtering. Accessible via a top-level "Analytics" sidebar item with an overview page and four drill-down subpages.

## Decisions

- **Navigation:** Top-level sidebar item "Analytics"
- **Charting library:** Chart.js via svelte-chartjs
- **Time ranges:** Preset buttons (7d, 30d, 90d, all) + custom date picker (from/to)
- **Page structure:** Overview + drill-down subpages
- **Filter scope:** Global filter bar (org, repo, author, time range) shared across all analytics pages
- **Backend aggregation:** On-the-fly SQL queries (no materialized views)
- **API design:** Dedicated endpoint per drill-down page

## Pages & Routes

| Route | Purpose |
|-------|---------|
| `/analytics` | Overview — KPI cards + summary charts |
| `/analytics/tokens` | Token usage deep dive |
| `/analytics/models` | Model distribution and trends |
| `/analytics/authors` | Author activity and leaderboard |
| `/analytics/attribution` | AI vs Human attribution breakdown |

## Global Filter Bar

Rendered in `/analytics/+layout.svelte`, shared across all analytics pages.

- **Org selector** — dropdown (for multi-org users)
- **Repo selector** — dropdown, "All repos" default
- **Author selector** — dropdown, "All authors" default
- **Time range** — preset buttons (7d, 30d, 90d, all) + custom from/to date picker

Filter state stored in URL query params (`?org_id=...&repo=...&author=...&from=...&to=...`) for shareability. Managed via `$page.url.searchParams` with pushState updates.

## Overview Page (`/analytics`)

### KPI Cards (top row)

| Card | Value | Subtext |
|------|-------|---------|
| Total Commits | count | vs previous period (% change) |
| Total Sessions | count | vs previous period |
| Total Tokens | formatted (e.g. 1.2M) | input / output breakdown |
| Active Authors | unique count | vs previous period |
| AI Attribution | overall % | human % complement |
| Estimated Cost | $USD | vs previous period |

### Summary Charts

1. **Token usage over time** — line chart, daily granularity, input vs output as stacked areas
2. **Top 5 repos by tokens** — horizontal bar chart
3. **Model distribution** — doughnut chart (% sessions per model)
4. **Recent activity** — table of last 10 commits with session count and tokens

Each chart card links to its drill-down page.

## Drill-Down Pages

### `/analytics/tokens`

- **Time series** — line chart of daily token usage (input, output, cache as separate lines)
- **By repo table** — sortable: repo name, total tokens, input, output, session count, avg tokens/session
- **By author** — bar chart of token consumption per author
- **Token efficiency** — tokens per commit, tokens per file changed

### `/analytics/models`

- **Distribution** — doughnut chart of sessions by model
- **Trends over time** — stacked area chart showing model usage shift
- **Author x Model matrix** — table: which authors use which models, how much
- **Model comparison** — bar chart: avg tokens/session and avg cost by model

### `/analytics/authors`

- **Leaderboard** — table: author, commits, sessions, tokens, cost, AI%, last active
- **Activity timeline** — line chart per selected author(s)
- **Model preferences** — per-author pie chart of model usage

### `/analytics/attribution`

- **AI vs Human trend** — stacked area chart over time
- **By repo** — bar chart of AI% per repo
- **By author** — bar chart of AI% per author
- **Lines breakdown** — total AI lines vs human lines with trend

## Backend API

All endpoints under `/api/v1/analytics/`. Common query parameters:

| Param | Type | Description |
|-------|------|-------------|
| `org_id` | `Option<Uuid>` | Filter by org (defaults to user's org) |
| `repo` | `Option<String>` | Filter by repo name |
| `author` | `Option<String>` | Filter by commit author |
| `from` | `Option<DateTime>` | Start date |
| `to` | `Option<DateTime>` | End date |

### `GET /api/v1/analytics/overview`

Returns all KPI values + sparkline data:

```json
{
  "total_commits": 142,
  "total_sessions": 318,
  "total_tokens": 4521000,
  "total_input_tokens": 3200000,
  "total_output_tokens": 1321000,
  "active_authors": 5,
  "estimated_cost_usd": 45.20,
  "ai_percentage": 62.3,
  "tokens_over_time": [{ "date": "2026-02-01", "input": 100000, "output": 50000 }],
  "top_repos": [{ "repo": "tracevault", "tokens": 2000000 }],
  "model_distribution": [{ "model": "claude-sonnet-4-6", "count": 200 }],
  "recent_commits": [{ "commit_sha": "abc123", "author": "...", "session_count": 3, "total_tokens": 50000, "created_at": "..." }]
}
```

### `GET /api/v1/analytics/tokens`

```json
{
  "time_series": [{ "date": "...", "input": 0, "output": 0, "cache": 0 }],
  "by_repo": [{ "repo": "...", "total": 0, "input": 0, "output": 0, "sessions": 0 }],
  "by_author": [{ "author": "...", "total": 0 }],
  "efficiency": [{ "commit_sha": "...", "tokens": 0, "files_changed": 0 }]
}
```

### `GET /api/v1/analytics/models`

```json
{
  "distribution": [{ "model": "...", "session_count": 0, "total_tokens": 0 }],
  "trends": [{ "date": "...", "model": "...", "count": 0 }],
  "author_model_matrix": [{ "author": "...", "model": "...", "sessions": 0, "tokens": 0 }],
  "comparison": [{ "model": "...", "avg_tokens": 0, "avg_cost": 0.0 }]
}
```

### `GET /api/v1/analytics/authors`

```json
{
  "leaderboard": [{ "author": "...", "commits": 0, "sessions": 0, "tokens": 0, "cost": 0.0, "ai_pct": 0.0, "last_active": "..." }],
  "timeline": [{ "date": "...", "author": "...", "commits": 0 }],
  "model_preferences": [{ "author": "...", "models": [{ "model": "...", "sessions": 0 }] }]
}
```

### `GET /api/v1/analytics/attribution`

```json
{
  "trend": [{ "date": "...", "ai_pct": 0.0, "human_pct": 0.0 }],
  "by_repo": [{ "repo": "...", "ai_pct": 0.0, "ai_lines": 0, "human_lines": 0 }],
  "by_author": [{ "author": "...", "ai_pct": 0.0 }],
  "totals": { "ai_lines": 0, "human_lines": 0, "ai_pct": 0.0 }
}
```

### `GET /api/v1/analytics/filters`

Returns available filter options for the current user:

```json
{
  "orgs": [{ "id": "...", "name": "..." }],
  "repos": [{ "id": "...", "name": "..." }],
  "authors": ["alice", "bob"]
}
```

## Technical Details

### Chart.js Configuration

- Install `chart.js` + `svelte-chartjs`
- Consistent color palette from TailwindCSS theme
- Responsive with aspect ratio preservation
- Dark mode support via Chart.js config

### Attribution Extraction

The `attribution` JSONB column has shape:
```json
{
  "files": [...],
  "summary": {
    "ai_percentage": 62.3,
    "human_percentage": 37.7,
    "total_lines_added": 150,
    "total_lines_deleted": 30
  }
}
```

SQL uses `(attribution->'summary'->>'ai_percentage')::float` to extract. Commits without attribution are excluded from AI% calculations (not treated as 0%).

### Time Series Granularity

| Range | Bucket |
|-------|--------|
| 7 days | hourly |
| 30 days | daily |
| 90+ days | weekly |

Backend determines granularity from the date range.

### Shared Layout

`/analytics/+layout.svelte` renders the global filter bar. Child pages slot below. Filter state is reactive via URL params.
