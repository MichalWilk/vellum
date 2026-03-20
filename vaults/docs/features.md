---
title: Features
tags: [features, overview]
---

# Features

Vellum provides a complete set of tools for hosting and browsing documentation from a Git repository.

## Git-backed storage

Your documents live in a standard Git repository. Vellum reads from a local bare repo (via `git2`) or from a Gitea/Forgejo instance via its HTTP API. This means:

- Full version history for every document
- Branching and merging workflows
- Collaboration through pull requests
- No vendor lock-in - your content is always plain Markdown

## OIDC authentication

Vellum integrates with any OIDC-compliant identity provider (Keycloak, Authentik, Zitadel, etc.):

- Authorization Code Flow with PKCE
- Session management via Redis
- HttpOnly cookies for security
- Realm roles extracted from JWT claims

## Role-based access control

Control document visibility per directory using `.vault.toml` files:

```toml
[access]
roles = ["dev", "admin"]
```

**Resolution rules:**
- No `.vault.toml` in a directory - inherit from parent
- Root default - any authenticated user (`["*"]`)
- The file tree API filters out nodes the user cannot access
- Hidden paths are never exposed to unauthorized users

> [!tip]
> Use `roles = ["*"]` to make a section visible to any authenticated user, regardless of their specific roles.

## Graph view

Vellum builds a graph of document relationships from `[[wikilinks]]`:

- Interactive visualization powered by Cytoscape.js
- fCoSE layout for readable node placement
- Click a node to navigate to that document
- Graph is role-filtered - you only see nodes you have access to
- Index stored in Redis, rebuilt on webhook push

## Full-text search

Search across all accessible documents instantly:

- ==Tantivy==-powered in-memory index (Lucene-equivalent)
- Indexed fields: path, title (from frontmatter), body (plain text)
- Results filtered by user roles
- Command palette UI (`Cmd+K` / `Ctrl+K`)
- Debounced input with highlighted snippets

## Markdown rendering

Vellum supports rich Markdown with extensions:

- Standard Markdown (headings, lists, links, images, code blocks)
- Fenced code blocks with syntax highlighting (highlight.js)
- Tables with alignment
- Task lists (`- [x] done`)
- Callouts / admonitions (`> [!tip]`, `> [!warning]`, etc.)
- `[[Wikilinks]]` for internal linking
- ==Highlights== for emphasis
- Footnotes[^1]
- Frontmatter metadata (YAML)

[^1]: Like this one.

## File tree sidebar

A navigable tree view of all accessible documents:

- Directories expand/collapse
- Current document highlighted
- Filtered by user roles - unauthorized paths hidden
- Sorted alphabetically, directories first

## Webhook integration

Vellum listens for Git push events to keep content fresh:

```
POST /api/webhook/push
```

On push, Vellum:
1. Rebuilds the graph index
2. Rebuilds the search index
3. Invalidates cached content

## Multiple vaults

Configure multiple document repositories in a single Vellum instance:

```toml
[[vaults]]
name = "docs"
path = "vaults/docs"
description = "User documentation"

[[vaults]]
name = "dev"
path = "vaults/dev"
description = "Technical documentation"
```

Each vault is an independent Git repository with its own access rules.

## Auth modes

Vellum supports two authentication modes:

| Mode | Use case | Auth required | Roles enforced |
|------|----------|---------------|----------------|
| `oidc` | Production | Yes | Yes |
| `none` | Local dev / demo | No | No |

See [[deployment]] for configuration details.

#features #overview
