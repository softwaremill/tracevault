# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/softwaremill/tracevault/compare/v0.4.0...v0.5.0) - 2026-03-28

### Added

- add top AI tools section to author detail page and fix DataTable clickability
- register AI tools analytics routes
- add AI tools analytics endpoints and software summary
- add AI tool usage tracking (migration + extraction)
- add top authors leaderboard to dashboard
- add author detail endpoint
- add user_id to AuthorLeaderboard, drop unused fields
- register software analytics routes
- add software user detail endpoint
- add software analytics list endpoint
- extract software usage from Bash events at ingest
- add user_software_usage migration
- add manual pricing sync endpoint and sync status endpoint
- wire startup + daily background pricing sync
- add sync_pricing function with diff, update, and recalculation
- add LiteLLM JSON parsing with model mapping and tests
- add source field to PricingEntry struct and queries
- add pricing sync migration (source column + sync log table)
- *(stream)* extract token usage and costs from transcript chunks in real-time
- *(api)* add traces UI endpoints (sessions, commits, timeline, attribution, branches) and remove old traces module
- *(branch-tracking)* track commits reaching branches and tags via webhooks
- *(attribution)* add line-level attribution engine with confidence scoring
- *(api)* add streaming event endpoint
- *(api)* add commit push endpoint with file-level attribution
- *(schema)* add streaming architecture tables
- *(pricing)* add pricing CRUD and recalculate API endpoints
- *(dashboard)* add handler and register GET /dashboard route
- *(dashboard)* add compliance query
- *(dashboard)* add KPI aggregation and sparkline queries
- *(dashboard)* add types, response struct, and period range helper
- register session detail API route
- add session detail transcript parser with per-call breakdown
- add model_pricing table with seed data

### Changed

- strip non-software data from software user detail endpoint
- slim get_software to org-wide tools only
- remove git-ai, compute attribution server-side from sessions
- clean up v2 references in comments, remove dead code and legacy script
- extract seal fields into commit_seals table in compliance and CI
- extract seal fields into session_seals in dashboard compliance query
- rename sessions_v2/commits_v2 in analytics
- rename sessions_v2/commits_v2 in remaining files
- rename sessions_v2/commits_v2 in traces_ui
- rename sessions_v2/commits_v2 in stream and commit_push endpoints
- consolidate migrations, remove v2 suffixes from schema
- use real session model names for pricing instead of canonical names
- extract shared recalculate_sessions_for_pricing function
- migrate all queries from old sessions table to sessions_v2
- remove old traces.rs and legacy endpoints entirely
- pricing module to support DB-backed rates with fallback

### Fixed

- apply repo/author filters to AI summary and filter empty sessions
- apply repo/author filters to software analytics query
- cast SUM(total_tokens) to BIGINT in software user detail query
- resolve TypeScript narrowing errors in software pages
- use git CLI for clone/fetch to support all SSH key formats
- update migration 008 to use renamed sessions table
- populate tool frequency data in session analytics
- compute duration and messages from fallback sources in analytics sessions
- resolve clippy warning in startup sync
- auto-sync repos on startup, improve attribution blame and error UX
- fix attribution confidence scoring and deduplicate file changes
- *(api)* restore legacy POST /traces endpoint for backward compatibility with old CLI
- *(api)* add committed_at to GROUP BY for linked commits query
- *(ui)* fix navigation responsiveness, transcript rendering, file change display, linked commits dedup, and branch tracking from commit-push
- *(stream)* process piggybacked transcript lines on all event types, not just Transcript
- *(dashboard)* cast SUM() results to int8 for sqlx type compatibility
- *(dashboard)* fill sparkline date gaps and parallelize queries
- never drop transcript records when message field is missing
- remove audit log from login to avoid nil org_id FK violation
- fix display github hashes for real commits
- fix warnings
- fix cargo clippy

## [0.4.0](https://github.com/softwaremill/tracevault/compare/v0.3.2...v0.4.0) - 2026-03-25

### Added

- *(stream)* extract token usage and costs from transcript chunks in real-time
- *(api)* add traces UI endpoints (sessions, commits, timeline, attribution, branches) and remove old traces module
- *(branch-tracking)* track commits reaching branches and tags via webhooks
- *(attribution)* add line-level attribution engine with confidence scoring
- *(api)* add streaming event endpoint
- *(api)* add commit push endpoint with file-level attribution
- *(schema)* add streaming architecture tables
- *(pricing)* add pricing CRUD and recalculate API endpoints
- *(dashboard)* add handler and register GET /dashboard route
- *(dashboard)* add compliance query
- *(dashboard)* add KPI aggregation and sparkline queries
- *(dashboard)* add types, response struct, and period range helper
- register session detail API route
- add session detail transcript parser with per-call breakdown
- add model_pricing table with seed data

### Changed

- migrate all queries from old sessions table to sessions_v2
- remove old traces.rs and legacy endpoints entirely
- pricing module to support DB-backed rates with fallback

### Fixed

- populate tool frequency data in session analytics
- compute duration and messages from fallback sources in analytics sessions
- resolve clippy warning in startup sync
- auto-sync repos on startup, improve attribution blame and error UX
- fix attribution confidence scoring and deduplicate file changes
- *(api)* restore legacy POST /traces endpoint for backward compatibility with old CLI
- *(api)* add committed_at to GROUP BY for linked commits query
- *(ui)* fix navigation responsiveness, transcript rendering, file change display, linked commits dedup, and branch tracking from commit-push
- *(stream)* process piggybacked transcript lines on all event types, not just Transcript
- *(dashboard)* cast SUM() results to int8 for sqlx type compatibility
- *(dashboard)* fill sparkline date gaps and parallelize queries
- never drop transcript records when message field is missing
- remove audit log from login to avoid nil org_id FK violation
- fix display github hashes for real commits
- fix warnings
- fix cargo clippy

## [0.3.2](https://github.com/softwaremill/tracevault/compare/v0.3.1...v0.3.2) - 2026-03-25

### Added

- *(stream)* extract token usage and costs from transcript chunks in real-time
- *(api)* add traces UI endpoints (sessions, commits, timeline, attribution, branches) and remove old traces module
- *(branch-tracking)* track commits reaching branches and tags via webhooks
- *(attribution)* add line-level attribution engine with confidence scoring
- *(api)* add streaming event endpoint
- *(api)* add commit push endpoint with file-level attribution
- *(schema)* add streaming architecture tables
- *(pricing)* add pricing CRUD and recalculate API endpoints
- *(dashboard)* add handler and register GET /dashboard route
- *(dashboard)* add compliance query
- *(dashboard)* add KPI aggregation and sparkline queries
- *(dashboard)* add types, response struct, and period range helper
- register session detail API route
- add session detail transcript parser with per-call breakdown
- add model_pricing table with seed data

### Changed

- migrate all queries from old sessions table to sessions_v2
- remove old traces.rs and legacy endpoints entirely
- pricing module to support DB-backed rates with fallback

### Fixed

- resolve clippy warning in startup sync
- auto-sync repos on startup, improve attribution blame and error UX
- fix attribution confidence scoring and deduplicate file changes
- *(api)* restore legacy POST /traces endpoint for backward compatibility with old CLI
- *(api)* add committed_at to GROUP BY for linked commits query
- *(ui)* fix navigation responsiveness, transcript rendering, file change display, linked commits dedup, and branch tracking from commit-push
- *(stream)* process piggybacked transcript lines on all event types, not just Transcript
- *(dashboard)* cast SUM() results to int8 for sqlx type compatibility
- *(dashboard)* fill sparkline date gaps and parallelize queries
- never drop transcript records when message field is missing
- remove audit log from login to avoid nil org_id FK violation
- fix display github hashes for real commits
- fix warnings
- fix cargo clippy

## [0.3.1](https://github.com/softwaremill/tracevault/compare/v0.3.0...v0.3.1) - 2026-03-25

### Added

- *(stream)* extract token usage and costs from transcript chunks in real-time
- *(api)* add traces UI endpoints (sessions, commits, timeline, attribution, branches) and remove old traces module
- *(branch-tracking)* track commits reaching branches and tags via webhooks
- *(attribution)* add line-level attribution engine with confidence scoring
- *(api)* add streaming event endpoint
- *(api)* add commit push endpoint with file-level attribution
- *(schema)* add streaming architecture tables
- *(pricing)* add pricing CRUD and recalculate API endpoints
- *(dashboard)* add handler and register GET /dashboard route
- *(dashboard)* add compliance query
- *(dashboard)* add KPI aggregation and sparkline queries
- *(dashboard)* add types, response struct, and period range helper
- register session detail API route
- add session detail transcript parser with per-call breakdown
- add model_pricing table with seed data

### Changed

- migrate all queries from old sessions table to sessions_v2
- remove old traces.rs and legacy endpoints entirely
- pricing module to support DB-backed rates with fallback

### Fixed

- fix attribution confidence scoring and deduplicate file changes
- *(api)* restore legacy POST /traces endpoint for backward compatibility with old CLI
- *(api)* add committed_at to GROUP BY for linked commits query
- *(ui)* fix navigation responsiveness, transcript rendering, file change display, linked commits dedup, and branch tracking from commit-push
- *(stream)* process piggybacked transcript lines on all event types, not just Transcript
- *(dashboard)* cast SUM() results to int8 for sqlx type compatibility
- *(dashboard)* fill sparkline date gaps and parallelize queries
- never drop transcript records when message field is missing
- remove audit log from login to avoid nil org_id FK violation
- fix display github hashes for real commits
- fix warnings
- fix cargo clippy

## [0.3.0](https://github.com/softwaremill/tracevault/compare/v0.2.0...v0.3.0) - 2026-03-25

### Added

- *(stream)* extract token usage and costs from transcript chunks in real-time
- *(api)* add traces UI endpoints (sessions, commits, timeline, attribution, branches) and remove old traces module
- *(branch-tracking)* track commits reaching branches and tags via webhooks
- *(attribution)* add line-level attribution engine with confidence scoring
- *(api)* add streaming event endpoint
- *(api)* add commit push endpoint with file-level attribution
- *(schema)* add streaming architecture tables
- *(pricing)* add pricing CRUD and recalculate API endpoints
- *(dashboard)* add handler and register GET /dashboard route
- *(dashboard)* add compliance query
- *(dashboard)* add KPI aggregation and sparkline queries
- *(dashboard)* add types, response struct, and period range helper
- register session detail API route
- add session detail transcript parser with per-call breakdown
- add model_pricing table with seed data

### Changed

- migrate all queries from old sessions table to sessions_v2
- remove old traces.rs and legacy endpoints entirely
- pricing module to support DB-backed rates with fallback

### Fixed

- *(api)* restore legacy POST /traces endpoint for backward compatibility with old CLI
- *(api)* add committed_at to GROUP BY for linked commits query
- *(ui)* fix navigation responsiveness, transcript rendering, file change display, linked commits dedup, and branch tracking from commit-push
- *(stream)* process piggybacked transcript lines on all event types, not just Transcript
- *(dashboard)* cast SUM() results to int8 for sqlx type compatibility
- *(dashboard)* fill sparkline date gaps and parallelize queries
- never drop transcript records when message field is missing
- remove audit log from login to avoid nil org_id FK violation
- fix display github hashes for real commits
- fix warnings
- fix cargo clippy

## [0.2.0](https://github.com/softwaremill/tracevault/compare/v0.1.0...v0.2.0) - 2026-03-23

### Fixed

- fix warnings
- fix cargo clippy
