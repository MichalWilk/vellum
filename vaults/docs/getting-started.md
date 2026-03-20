---
title: Getting Started
tags: [guide, setup, quickstart]
---

# Getting Started

This guide walks you through setting up Vellum for the first time. By the end, you'll have a running instance serving documents from a Git repository.

## Prerequisites

- Docker and Docker Compose
- A Git repository with Markdown files
- (Optional) A Keycloak instance for authentication

## Quick start with Docker Compose

### 1. Clone the repository

```bash
git clone https://github.com/your-org/vellum.git
cd vellum
```

### 2. Create your environment file

Copy the example and fill in your secrets:

```bash
cp .env.example .env
```

Edit `.env` with your values:

```env
CLIENT_SECRET=your-keycloak-client-secret
SESSION_SECRET=a-random-32-byte-hex-string
WEBHOOK_SECRET=your-webhook-secret
```

### 3. Configure your vault

Edit `config.toml` to point at your document repository:

```toml
[[vaults]]
name = "docs"
path = "vaults/docs"
description = "My documentation"
```

### 4. Start everything

```bash
docker compose up -d
```

Vellum will be available at `https://localhost:7443` (or your configured domain). Keycloak admin at `http://localhost:8080`.

> [!note]
> On first startup, Vellum builds the search index and graph. This takes a few seconds depending on vault size.

## Running without authentication

For local development or quick evaluation, you can disable authentication entirely:

```toml
# config.local.toml
[auth]
mode = "none"
```

This skips OIDC and makes all documents visible without login. See [[deployment]] for production configuration.

> [!warning]
> Never use `mode = "none"` in production. All documents will be publicly accessible.

## Writing documents

Vellum reads standard Markdown files. Create `.md` files in your vault repository and push them:

```bash
echo "# Hello World" > docs/hello.md
git add docs/hello.md
git commit -m "add hello doc"
git push
```

Vellum picks up changes via a webhook (or on restart).

### Frontmatter

Each document can include YAML frontmatter:

```yaml
---
title: My Document
tags: [guide, important]
---
```

### Wikilinks

Link between documents using `[[wikilinks]]`:

```markdown
See [[features]] for the full feature list.
Check the [[faq]] for common questions.
```

These links also appear in the [[features|graph view]].

## Access control

Create a `.vault.toml` file in any directory to control access:

```toml
[access]
roles = ["dev", "admin"]
```

See [[features]] for details on role-based access control.

## Next steps

- [[features]] - Learn what Vellum supports
- [[deployment]] - Configure for production
- [[faq]] - Troubleshooting and tips

#quickstart #setup
