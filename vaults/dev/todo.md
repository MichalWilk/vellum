---
title: Todo and Roadmap
tags: [todo, roadmap, planning]
---

# Todo and Roadmap

Current tasks, priorities, and future plans for Vellum.

## MVP - In progress

### Completed

- [x] OIDC authentication flow with Keycloak
- [x] Session management with Redis
- [x] `/api/me` endpoint
- [x] `LocalGitClient` via git2
- [x] `/api/tree` with role filtering
- [x] `.vault.toml` parsing and role resolution
- [x] `/api/doc/*path` with Markdown rendering
- [x] Pulldown-cmark HTML rendering with sanitization
- [x] SvelteKit shell - login page, auth guard
- [x] File tree sidebar component
- [x] Markdown document view
- [x] Multi-vault support in config

### In progress

- [ ] Wikilink parser (pulldown-cmark event stream)
- [ ] Graph index builder (nodes + edges)
- [ ] `/api/graph` endpoint with role filtering
- [ ] Cytoscape.js graph view in frontend
- [ ] Tantivy search index builder
- [ ] `/api/search` endpoint
- [ ] Command palette UI (`Cmd+K`)
- [ ] Webhook endpoint for Git push events
- [ ] Vault content reorganization

### Not started

- [ ] Frontend search result highlighting
- [ ] Graph node click navigation
- [ ] Document breadcrumb navigation
- [ ] Responsive mobile layout
- [ ] Loading states and error pages

## Post-MVP

> [!note]
> These are ideas, not commitments. Priorities may shift based on usage and feedback.

### Near term
- [ ] Gitea/Forgejo HTTP provider implementation
- [ ] Document edit API (PUT /api/doc/*path)
- [ ] Version history view (git log per file)
- [ ] Table of contents sidebar for long documents
- [ ] Dark mode toggle

### Medium term
- [ ] Canvas view (spatial document layout)
- [ ] Comments / annotations on documents
- [ ] PDF export
- [ ] Mermaid diagram rendering
- [ ] LaTeX math rendering (KaTeX)

### Long term
- [ ] Real-time collaboration (CRDT-based editing)
- [ ] Plugin system for custom renderers
- [ ] Vault-level search scoping
- [ ] Custom themes

## Known blockers

| Issue | Impact | Status |
|-------|--------|--------|
| git2 crate blocks async runtime | Must use `spawn_blocking` for all git ops | Mitigated |
| Tantivy index rebuild is synchronous | Brief pause on webhook push for large vaults | Accepted for MVP |

See also: [[tech-debt]], [[architecture]]

#todo #roadmap #planning
