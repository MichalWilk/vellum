---
title: Architecture Overview
tags: [architecture, design, backend, frontend]
---

# Architecture Overview

Vellum follows a clean separation between backend (Rust/Axum) and frontend (SvelteKit), connected through a JSON API. Caddy sits in front as a reverse proxy handling TLS and routing.

## High-level diagram

```
                    ┌─────────────┐
                    │   Browser   │
                    └──────┬──────┘
                           │ HTTPS
                    ┌──────┴──────┐
                    │    Caddy    │
                    │  (TLS, proxy)│
                    └──┬──────┬──┘
                       │      │
              /api/*   │      │  /*
                       ▼      ▼
              ┌────────┐  ┌──────────┐
              │Backend │  │ Frontend │
              │ (Axum) │  │(SvelteKit)│
              └──┬──┬──┘  └──────────┘
                 │  │
          ┌──────┘  └──────┐
          ▼                ▼
    ┌──────────┐    ┌──────────┐
    │  Redis   │    │ Git Repo │
    │(sessions,│    │  (vault) │
    │  cache)  │    └──────────┘
    └──────────┘
          ▲
          │
    ┌──────────┐
    │ Keycloak │
    │  (OIDC)  │
    └──────────┘
```

## Backend structure

The backend is organized into four modules:

### `auth/` - Authentication and authorization

- `oidc.rs` - OIDC client setup, login and callback handlers
- `session.rs` - Session types, `current_user` extractor
- `middleware.rs` - `require_auth` and `require_role` tower layers

The auth middleware supports two modes (`oidc` and `none`). In `none` mode, a synthetic anonymous user is injected into every request.

### `docs/` - Document serving

- `git_client.rs` - `GitProvider` trait with `LocalGitClient` (git2) and `GiteaClient` implementations
- `vault_config.rs` - `.vault.toml` parser and role resolution logic
- `handlers.rs` - `/api/tree` and `/api/doc/*path` endpoints

Role resolution walks up the directory tree from the requested path to the vault root, using the nearest `.vault.toml` to determine required roles.

### `graph/` - Graph index

- `parser.rs` - Wikilink extraction from Markdown using pulldown-cmark events
- `builder.rs` - Node and edge construction from parsed links
- `handlers.rs` - `/api/graph` endpoint

The graph index is built on startup and stored in Redis. It contains all documents regardless of roles - filtering happens at query time.

### `search/` - Full-text search

- `index.rs` - Tantivy schema definition, index builder, rebuild logic
- `handlers.rs` - `/api/search` endpoint

The search index lives in memory (no persistence). It indexes three fields per document: `path`, `title` (from frontmatter), and `body` (plain text with Markdown stripped).

## Frontend structure

### Data loading

All data fetching happens in SvelteKit `load` functions (`+page.ts`, `+layout.ts`). Components never call `fetch` directly - they receive data as props.

### Auth guard

The root `+layout.ts` calls `/api/me`. If it returns 401, the user is redirected to `/login`. This runs on every navigation.

### Routing

```
/              --> redirect to /docs
/login         --> login page (OIDC redirect)
/docs          --> file tree + document view
/docs/[...path] --> specific document
/graph         --> interactive graph view
```

### State management

Svelte stores are used for global state:
- `userStore` - current user info (sub, name, roles)
- `docStore` - currently viewed document
- `searchStore` - search query, results, palette visibility

## Data flow: viewing a document

1. User navigates to `/docs/guides/setup`
2. `+page.ts` load function calls `GET /api/doc/guides/setup`
3. Backend resolves the vault path, checks `.vault.toml` roles
4. If authorized: reads file via `GitProvider`, renders Markdown to HTML
5. Returns `{ content, frontmatter, path, last_modified }`
6. Frontend renders the HTML in the document view

## Data flow: graph view

1. User navigates to `/graph`
2. `+page.ts` calls `GET /api/graph`
3. Backend loads the full graph from Redis
4. Filters nodes and edges based on user's roles
5. Returns `{ nodes, edges }`
6. Frontend renders with Cytoscape.js using fCoSE layout

## Key design decisions

- **Git as source of truth** - no database for content, Git provides versioning
- **In-memory search index** - fast, no external service, rebuilt on push
- **Role filtering at query time** - index everything once, filter per request
- **Thin handlers** - handlers extract input and delegate to service functions
- **No `unwrap()` in production** - all errors propagated with `?` operator

See also: [[api-reference]], [[tech-debt]]

#architecture #design
