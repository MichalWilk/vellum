---
title: FAQ
tags: [faq, help, troubleshooting]
---

# Frequently Asked Questions

## General

### What is Vellum?

Vellum is a self-hosted document viewer that serves Markdown files from a Git repository. It adds authentication, role-based access control, full-text search, and a graph view on top of your existing documentation.

### How is Vellum different from a wiki?

Vellum is read-only by design. Your documents live in Git, and you edit them with your preferred tools (VS Code, Obsidian, vim, etc.). Vellum focuses on presenting and navigating those documents with access control and search.

### Can I use Vellum with Obsidian?

Yes. Vellum supports `[[wikilinks]]`, callouts, highlights, and tags - all common Obsidian syntax. Point Vellum at the same Git repository your Obsidian vault syncs to, and it will render your notes.

### Does Vellum support editing?

Not yet. Vellum is currently a viewer. Editing support is planned for a future release. For now, edit your Markdown files locally and push to Git.

## Authentication

### Do I need Keycloak?

Not necessarily. Vellum supports any OIDC-compliant identity provider. Keycloak is the reference implementation used in development. Authentik, Zitadel, or any standard provider should work.

### Can I run Vellum without authentication?

Yes. Set `auth.mode = "none"` in your configuration:

```toml
[auth]
mode = "none"
```

This disables all authentication and makes every document publicly accessible. Useful for local development or internal trusted networks.

> [!warning]
> With `mode = "none"`, all `.vault.toml` access rules are ignored. Do not use this in production if your documents contain sensitive content.

### How do roles work?

Roles are extracted from the `realm_access.roles` claim in the OIDC JWT token. Each directory can specify required roles in its `.vault.toml`:

```toml
[access]
roles = ["dev", "admin"]
```

A user needs at least one matching role to access documents in that directory. `["*"]` means any authenticated user.

## Configuration

### Where do I put secrets?

Never in `config.toml` (which is committed to Git). Use either:

- `config.local.toml` (gitignored) for local development
- Environment variables with the `VELLUM__` prefix for production

See [[deployment]] for details.

### Can I have multiple vaults?

Yes. Define multiple `[[vaults]]` entries in `config.toml`:

```toml
[[vaults]]
name = "docs"
path = "vaults/docs"
description = "User documentation"

[[vaults]]
name = "internal"
path = "vaults/internal"
description = "Internal docs"
```

Each vault is an independent Git repository with its own `.vault.toml` access rules.

### How does the config loading order work?

1. `config.toml` - base configuration (committed)
2. `config.local.toml` - local overrides (gitignored)
3. Environment variables (`VELLUM__` prefix) - highest priority

Later sources override earlier ones.

## Troubleshooting

### Documents don't update after pushing

Make sure you have the webhook configured:

1. Set `WEBHOOK_SECRET` in your environment
2. Configure your Git host to POST to `/api/webhook/push` on push events
3. Check backend logs for webhook processing errors

Alternatively, restart the backend - it rebuilds indexes on startup.

### Search returns no results

The search index is built in memory on startup. If you recently added documents:

1. Trigger a webhook push, or restart the backend
2. Check that your documents have content (empty files are not indexed)
3. Verify the vault path is correctly configured

### The graph view is empty

The graph is built from `[[wikilinks]]` in your Markdown files. If your documents don't use wikilinks, the graph will have nodes but no edges. Add links between documents:

```markdown
Related: [[other-document]]
```

### I get a 403 on a document I should have access to

Check the `.vault.toml` chain from the document's directory up to the vault root. The effective roles are determined by the nearest `.vault.toml` in the directory hierarchy. Make sure your user has at least one of the required roles.

See also: [[deployment]] for more configuration details.

#faq #troubleshooting
