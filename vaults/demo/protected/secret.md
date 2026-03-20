---
title: Protected Document
tags: [demo, protected, rbac]
---

# Protected Document

If you can see this page, you have the `dev` role.

This document lives in the `protected/` directory, which requires `roles = ["dev"]` as defined in the vault's `.vault.toml`:

```toml
[access.paths]
"protected/" = ["dev"]
```

## How it works

1. The root `.vault.toml` sets `roles = ["*"]` - anyone can access the vault
2. The `access.paths` table overrides this for the `protected/` directory
3. Only users with the `dev` role in their Keycloak `realm_access.roles` claim can see this content
4. Users without the `dev` role don't see `protected/` in the file tree at all

## Access control in practice

This demonstrates Vellum's ==per-directory access control==. You can mix public and restricted content in the same vault:

```
demo/
├── .vault.toml          # roles = ["*"], paths: {"protected/" = ["dev"]}
├── index.md             # visible to everyone
├── markdown-showcase.md # visible to everyone
├── wikilinks-demo.md    # visible to everyone
└── protected/
    └── secret.md        # visible only to dev role
```

> [!note]
> In the graph view, this document only appears for users with the `dev` role. The node and any edges to it are filtered out for other users.

## Verify it yourself

1. Log in with a user that has the `dev` role - you see this page
2. Log in with a user that does not have the `dev` role - this page (and the entire `protected/` directory) is invisible

There's no "403 Forbidden" page. The content simply doesn't exist from the unauthorized user's perspective.

See [[index|back to demo home]].

#protected #rbac #access-control
