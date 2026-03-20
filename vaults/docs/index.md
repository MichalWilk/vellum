---
title: Welcome to Vellum
tags: [home, overview]
---

# Welcome to Vellum

Vellum is a self-hosted document viewer that turns your Git repository into a browsable, searchable knowledge base with role-based access control and a graph view of linked documents.

> [!tip] Getting started?
> Head to [[getting-started]] for setup instructions, or browse [[features]] to see what Vellum can do.

## Why Vellum?

- **Git as storage** - your documents live in a Git repo. Version history, branching, and collaboration come for free.
- **OIDC authentication** - integrate with Keycloak or any OIDC provider for single sign-on.
- **Role-based access** - control who sees what using simple `.vault.toml` files per directory.
- **Graph view** - visualize how your documents connect through `[[wikilinks]]`.
- **Full-text search** - find anything instantly with the `Cmd+K` command palette.
- **Markdown-first** - write in Markdown with callouts, code blocks, highlights, and more.

## Quick links

- [[getting-started]] - Set up Vellum in minutes
- [[features]] - Full feature list
- [[deployment]] - Production deployment guide
- [[faq]] - Common questions and answers
- [[changelog]] - Version history

## How it works

```
Git repo (your docs) --> Vellum backend (Rust/Axum) --> SvelteKit frontend
                              |
                         Keycloak (OIDC)
                         Redis (sessions + cache)
```

Vellum reads Markdown files from a Git repository (local bare repo or Gitea/Forgejo), renders them to HTML, and serves them through a clean web interface. Authentication is handled via OIDC, and access control is defined per-directory using `.vault.toml` configuration files.

#documentation #overview
