---
title: Deployment Guide
tags: [deployment, docker, configuration]
---

# Deployment Guide

This guide covers deploying Vellum in production with Docker Compose, including TLS, authentication, and multi-vault configuration.

## Architecture overview

```
Internet --> Frontend (SvelteKit, port 7000)
               |
               +--> /api/*  --> Backend (Rust/Axum, port 3000) [server-side proxy]
               +--> /*      --> SSR pages
               |
             Keycloak (OIDC provider, port 8080)
             Redis (sessions + graph cache)
```

## Docker Compose deployment

### Directory layout

```
vellum/
├── docker-compose.yml
├── config.toml
├── config.local.toml   # gitignored, secrets
├── .env                 # gitignored, docker secrets
└── vaults/
    ├── docs/            # each vault is a git repo
    └── dev/
```

### Configuration

Vellum loads configuration in this order (later overrides earlier):

1. `config.toml` - committed defaults
2. `config.local.toml` - local overrides (gitignored)
3. Environment variables - `VELLUM__` prefix, `__` as separator

#### config.toml (committed)

```toml
[app]
host = "0.0.0.0"
port = 3000
log_level = "info"

[auth]
mode = "oidc"
default_roles = ["*"]

[auth.oidc]
issuer_url = "https://keycloak.example.com/realms/vellum"
client_id = "vellum"

[auth.oidc.session]
redis_url = "redis://redis:6379"
cookie_name = "vellum_session"
cookie_secure = true

[[vaults]]
name = "docs"
path = "vaults/docs"
description = "User documentation"
```

> [!warning]
> Never put secrets in `config.toml`. Use `config.local.toml` or environment variables.

#### Secrets

Set these in `.env` (never committed):

```env
CLIENT_SECRET=your-oidc-client-secret
SESSION_SECRET=generate-a-random-32-byte-hex
WEBHOOK_SECRET=shared-secret-for-git-webhooks
```

### Ports

Default ports: `7000` (app), `8080` (Keycloak). Override via `PORT`, `KC_PORT` in `.env`.

For TLS, put a reverse proxy (nginx, Caddy, Cloudflare Tunnel) in front of the frontend.

### Starting the stack

```bash
docker compose up -d
```

Verify everything is running:

```bash
docker compose ps
docker compose logs backend
```

## Keycloak setup

### Create a realm

1. Log in to Keycloak admin console
2. Create a new realm (e.g., `vellum`)
3. Create a client:
   - Client ID: `vellum`
   - Client Protocol: `openid-connect`
   - Access Type: `confidential`
   - Valid Redirect URIs: `https://your-domain.com/api/auth/callback`

### Configure roles

Create realm roles that match your `.vault.toml` access rules:

- `dev` - access to developer documentation
- `admin` - full access

Assign roles to users in Keycloak. Vellum reads roles from the `realm_access.roles` JWT claim.

> [!tip]
> Use `roles = ["*"]` in `.vault.toml` to grant access to any authenticated user, regardless of their assigned roles.

## Webhook setup

Configure your Git hosting to send push webhooks to Vellum:

```
POST https://your-domain.com/api/webhook/push
Content-Type: application/json
X-Webhook-Secret: your-webhook-secret
```

This triggers a rebuild of the graph and search indexes.

## Resource requirements

Vellum is lightweight:

| Component | RAM | CPU | Storage |
|-----------|-----|-----|---------|
| Backend | ~50 MB + index size | Minimal | None |
| Frontend | ~30 MB | Minimal | None |
| Redis | ~10 MB + sessions | Minimal | Persistent volume |
| Caddy | ~30 MB | Minimal | Certificate storage |

The search and graph indexes are held in memory. For a vault with 1000 documents, expect roughly 20-50 MB of additional RAM.

## Troubleshooting

### Backend won't start

Check logs: `docker compose logs backend`

Common issues:
- Redis not reachable - verify `redis_url` in config
- OIDC issuer unreachable - verify Keycloak is running and the URL is correct
- Vault path doesn't exist - verify the path in `[[vaults]]` config

### Authentication loop

- Verify `cookie_secure = true` only when serving over HTTPS
- Check that the callback URL in Keycloak matches your domain exactly
- Ensure the client secret matches between Keycloak and your config

See also: [[faq]] for more common issues.

#deployment #docker #production
