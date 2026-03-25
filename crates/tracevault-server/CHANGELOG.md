# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
