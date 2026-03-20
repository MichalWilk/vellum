---
title: Technical Debt
tags: [tech-debt, improvements, quality]
---

# Technical Debt

Known issues, shortcuts, and areas for improvement. Items here are not bugs - they're acknowledged compromises made for development velocity.

## Backend

### Git operations block the async runtime

The `git2` crate is synchronous. All git operations are wrapped in `tokio::task::spawn_blocking`, but this adds complexity and limits concurrency under heavy load.

**Mitigation:** Current approach is acceptable for expected load. Consider `gitoxide` (pure Rust, async-friendly) as a future replacement.

### Graph index stored in Redis as a single key

The entire graph is serialized to a single Redis key (`graph:index`). For large vaults (10k+ documents), this could become a bottleneck.

**Mitigation:** Acceptable for MVP. If scaling becomes an issue, split into per-vault keys or use a proper graph store.

### Search index is not persisted

The Tantivy index is rebuilt from scratch on every startup. For large vaults, this adds startup time.

**Mitigation:** Acceptable for MVP. Consider persisting the index to disk and only rebuilding on changes.

### No rate limiting on API endpoints

API endpoints have no rate limiting. A malicious or misbehaving client could overwhelm the backend.

**Mitigation:** Add `tower::limit` middleware before production deployment.

### Error responses lack detail in production

Error messages are generic to avoid leaking internal details. This makes debugging harder for API consumers.

**Mitigation:** Add a `debug` mode that includes detailed error info. Use structured error codes.

## Frontend

### No offline support

The frontend requires a network connection. No service worker or local caching.

**Mitigation:** Low priority - Vellum is a server-side application by design.

### No skeleton loading states

Pages show nothing while data loads, then pop in. Should use skeleton components for better perceived performance.

**Mitigation:** Add skeleton components for tree, document view, and graph.

### Search results don't highlight matches in context

The search API returns snippets with highlighted terms, but the frontend doesn't render the HTML highlights.

**Mitigation:** Render snippet HTML safely (already sanitized server-side).

## Infrastructure

### No health check endpoint

Docker Compose has no health checks configured for the backend. If the backend starts but fails to connect to Redis, it's not reported.

**Mitigation:** Add `GET /api/health` endpoint that checks Redis connectivity and vault accessibility.

### No structured logging

Backend uses `tracing` but outputs plain text. Should output JSON in production for log aggregation.

**Mitigation:** Add a `log_format` config option (`text` for dev, `json` for production).

### No backup strategy for Redis

Redis holds session data and the graph cache. Sessions are ephemeral, but losing them forces all users to re-authenticate.

**Mitigation:** Configure Redis persistence (RDB/AOF). Graph cache is rebuilt on startup, so loss is not critical.

## Code quality

### Test coverage is low

Limited unit tests for role resolution and wikilink parsing. No integration tests for the full request lifecycle.

**Mitigation:** Add tests incrementally. Priority: role resolution, wikilink parsing, auth middleware.

### No CI pipeline

No automated testing or linting on push/PR.

**Mitigation:** Set up GitHub Actions with `cargo test`, `cargo clippy`, `npm run check`, `npm run lint`.

> [!tip]
> When fixing tech debt, update this document. Remove resolved items and add new ones as they're discovered.

See also: [[todo]], [[architecture]]

#tech-debt #quality
