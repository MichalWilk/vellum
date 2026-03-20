# Vellum

Self-hosted document viewer with OIDC auth, role-based access, graph view, and full-text search. Obsidian-compatible Markdown rendering.

**Stack:** Rust/Axum, SvelteKit, Keycloak, Redis

## Quick start

```bash
cp .env.example .env    # edit secrets
just up                 # http://localhost:7000
```

## Local development

```bash
just dev       # Redis + Keycloak
just be        # terminal 1
just fe        # terminal 2
```

Open http://localhost:5173

## Documentation

- [Features](vaults/docs/features.md)
- [Getting Started](vaults/docs/getting-started.md)
- [Deployment](vaults/docs/deployment.md)
- [FAQ](vaults/docs/faq.md)
- [Changelog](vaults/docs/changelog.md)
- [Architecture](vaults/dev/architecture.md)
- [API Reference](vaults/dev/api-reference.md)
- [Development Setup](vaults/dev/setup.md)

## License

MIT
