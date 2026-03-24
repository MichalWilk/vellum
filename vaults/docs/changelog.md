---
title: Changelog
tags: [changelog, releases]
---

# Changelog

All notable changes to Vellum are documented here. This project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Multi-vault support - configure multiple independent document repositories
- Full-text search powered by Tantivy with `Cmd+K` command palette
  - Four search modes: Content, Files, Tags, Headings
  - Alt+1-4 shortcuts to switch modes
  - Last used mode persisted in browser
  - Tag drill-down: click tag to see related documents
- Graph view showing `[[wikilink]]` relationships between documents
- Role-based access control via `.vault.toml` per directory
- OIDC authentication with Keycloak (Authorization Code Flow)
- `auth.mode = "none"` for local development and demos
- Webhook endpoint for Git push events (triggers index rebuild)
- Markdown rendering with callouts, highlights, task lists, footnotes
- Syntax highlighting for fenced code blocks
- File tree sidebar with role-filtered navigation
- Docker Compose deployment with Caddy, Redis, and Keycloak

### Technical
- Backend: Rust with Axum 0.8, `pulldown-cmark`, `tantivy`, `git2`
- Frontend: SvelteKit with TypeScript, Tailwind CSS v4, Cytoscape.js
- Session storage: Redis via `tower-sessions`
- Git provider abstraction: local bare repo (default) or Gitea/Forgejo API

---

> [!note]
> Vellum is in active development. This changelog will be updated with each release.

See [[features]] for the full feature list, or [[todo|dev roadmap]] for what's coming next.

#changelog #releases
