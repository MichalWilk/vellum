---
title: Developer Documentation
tags: [dev, index]
---

# Developer Documentation

Technical documentation for Vellum contributors and maintainers.

> [!note]
> This vault requires the `dev` role. If you're reading this, you have developer access.

## Documentation index

- [[architecture]] - System architecture, component overview, data flow
- [[api-reference]] - All API endpoints with request/response examples
- [[setup]] - Local development environment setup
- [[todo]] - Current tasks and roadmap
- [[tech-debt]] - Known issues, technical debt, improvement ideas
- [[snippets]] - Useful code patterns and examples

## Tech stack summary

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Backend | Rust + Axum 0.8 | API server |
| Auth | openidconnect crate | OIDC flow |
| Sessions | tower-sessions + Redis | Session storage |
| Markdown | pulldown-cmark | MD parsing, wikilink extraction |
| Search | Tantivy | Full-text index |
| Frontend | SvelteKit + TypeScript | Web UI |
| Styling | Tailwind CSS v4 | Component styling |
| Graph | Cytoscape.js + fCoSE | Interactive graph view |
| Proxy | Caddy | TLS termination, routing |
| Auth provider | Keycloak | OIDC identity provider |
| Cache | Redis 7 | Sessions, graph cache |

## Contributing

1. Read [[setup]] to get your local environment running
2. Check [[todo]] for open tasks
3. Follow Conventional Commits: `feat(scope): description`
4. Submit a PR against `master`

#dev #index
