---
title: Development Setup
tags: [setup, dev, local]
---

# Development Setup

Guide for setting up a local Vellum development environment.

## Prerequisites

- Rust 1.75+ (with `cargo`)
- Node.js 20+ (with `npm`)
- Redis 7 (or Docker)
- Git
- (Optional) Keycloak instance for OIDC testing

## Clone and configure

```bash
git clone https://github.com/your-org/vellum.git
cd vellum
```

Create a local config override:

```toml
# config.local.toml
[auth]
mode = "none"

[auth.oidc.session]
cookie_secure = false
```

This disables authentication for local development.

## Backend

### Install dependencies

The backend uses only Cargo dependencies - no system libraries needed beyond what Rust provides, except for `libgit2` (used by the `git2` crate).

On Fedora/RHEL:

```bash
sudo dnf install libgit2-devel cmake
```

On Ubuntu/Debian:

```bash
sudo apt install libgit2-dev cmake
```

### Run the backend

```bash
cargo run --manifest-path backend/Cargo.toml
```

The backend starts on `http://localhost:3000` by default.

### Environment variables

Override any config value with `VELLUM__` prefix:

```bash
VELLUM__APP__LOG_LEVEL=debug cargo run --manifest-path backend/Cargo.toml
```

## Frontend

### Install dependencies

```bash
npm install --prefix frontend
```

### Run in development mode

```bash
npm run dev --prefix frontend
```

The frontend dev server starts on `http://localhost:5173` with hot reload.

### Build for production

```bash
npm run build --prefix frontend
```

## Redis

For sessions and graph cache. Start with Docker:

```bash
docker run -d --name vellum-redis -p 6379:6379 redis:7-alpine
```

> [!tip]
> When running with `auth.mode = "none"`, Redis is only needed for the graph cache. If you're not working on graph features, you can skip it temporarily.

## Test vault

Create a test vault with some documents:

```bash
mkdir -p /tmp/test-vault
cd /tmp/test-vault
git init
echo "# Hello" > index.md
echo "See [[other]]" >> index.md
echo "# Other page" > other.md
git add . && git commit -m "initial"
```

Point Vellum at it:

```toml
# config.local.toml
[[vaults]]
name = "test"
path = "/tmp/test-vault"
description = "Test vault"
```

## Running with Docker Compose

For a full-stack local environment:

```bash
docker compose -f docker-compose.local.yml up
```

This starts all services (backend, frontend, Redis, Caddy) with development-friendly settings.

## Useful commands

| Command | Purpose |
|---------|---------|
| `cargo check --manifest-path backend/Cargo.toml` | Type check backend |
| `cargo test --manifest-path backend/Cargo.toml` | Run backend tests |
| `cargo clippy --manifest-path backend/Cargo.toml` | Lint backend |
| `npm run check --prefix frontend` | Type check frontend |
| `npm run lint --prefix frontend` | Lint frontend |

## Project layout

See [[architecture]] for the full project structure and module descriptions.

## Commit conventions

Use Conventional Commits:

```
feat(auth): add PKCE support
fix(graph): handle circular wikilinks
docs(api): update search endpoint examples
refactor(docs): extract role resolution to module
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `perf`, `ci`

See also: [[todo]], [[api-reference]]

#setup #development
