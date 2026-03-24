---
title: API Reference
tags: [api, endpoints, backend]
---

# API Reference

All API endpoints served by the Vellum backend. Base path: `/api`.

## Authentication

### GET /api/auth/login

Initiates the OIDC Authorization Code Flow. Redirects the user to Keycloak.

**Response:** `302` redirect to Keycloak authorization URL.

Only available when `auth.mode = "oidc"`. Returns `404` in `none` mode.

### GET /api/auth/callback

OIDC callback endpoint. Keycloak redirects here after authentication.

**Query parameters:**
- `code` - authorization code from Keycloak
- `state` - CSRF state parameter

**Response:** `302` redirect to `/docs` (sets `HttpOnly` session cookie).

**Errors:**
- `400` - missing or invalid code/state
- `500` - token exchange failed

### POST /api/auth/logout

Clears the user's session.

**Response:** `302` redirect to `/login`.

### GET /api/me

Returns the current authenticated user.

**Response (200):**

```json
{
  "sub": "user-uuid",
  "name": "John Doe",
  "email": "john@example.com",
  "roles": ["dev", "admin"]
}
```

**Errors:**
- `401` - not authenticated

In `none` mode, returns a synthetic anonymous user:

```json
{
  "sub": "anonymous",
  "name": "Anonymous",
  "email": "",
  "roles": ["*"]
}
```

---

## Documents

### GET /api/tree

Returns the file tree for all accessible documents, filtered by the user's roles.

**Query parameters:**
- `vault` (optional) - vault name, defaults to first configured vault

**Response (200):**

```json
[
  {
    "name": "guides",
    "path": "guides",
    "type": "dir",
    "children": [
      {
        "name": "setup.md",
        "path": "guides/setup.md",
        "type": "file"
      }
    ]
  },
  {
    "name": "index.md",
    "path": "index.md",
    "type": "file"
  }
]
```

> [!note]
> Directories and files the user cannot access are omitted entirely. The client never learns about hidden paths.

### GET /api/doc/*path

Returns a rendered document.

**Path parameter:**
- `*path` - document path relative to vault root (e.g., `guides/setup.md`)

**Query parameters:**
- `vault` (optional) - vault name

**Response (200):**

```json
{
  "content": "<h1>Setup Guide</h1><p>...</p>",
  "frontmatter": {
    "title": "Setup Guide",
    "tags": ["guide", "setup"]
  },
  "path": "guides/setup.md",
  "last_modified": "2025-12-15T10:30:00Z"
}
```

**Errors:**
- `403` - user lacks required role for this path
- `404` - document not found

---

## Graph

### GET /api/graph

Returns the document graph (nodes and edges from wikilinks), filtered by user roles.

**Query parameters:**
- `vault` (optional) - vault name

**Response (200):**

```json
{
  "nodes": [
    { "id": "index.md", "label": "index" },
    { "id": "features.md", "label": "features" },
    { "id": "setup.md", "label": "setup" }
  ],
  "edges": [
    { "source": "index.md", "target": "features.md" },
    { "source": "index.md", "target": "setup.md" }
  ]
}
```

Nodes represent documents. Edges represent `[[wikilinks]]` from one document to another. Only documents the user can access are included.

---

## Search

### GET /api/search

Full-text search across accessible documents with multiple search modes.

**Query parameters:**
- `q` (optional) - search query string (required for `content`, `files`, `headings` modes; optional for `tags`)
- `type` (optional) - search mode, defaults to `content`
  - `content` - full-text search across titles and bodies
  - `files` - fuzzy search by file name/path (requires min 2 characters in `q`)
  - `tags` - browse all tags with document counts
  - `headings` - search headings across documents
- `tag` (optional) - exact tag name to filter results (used with `files` or `content` types for tag drill-down)
- `limit` (optional) - max results, default `20`
- `vault` (optional) - vault name

**Response (200) - Content/Files/Headings:**

```json
[
  {
    "path": "guides/setup.md",
    "title": "Setup Guide",
    "snippet": "...install Docker and run <mark>docker compose</mark> up...",
    "score": 8.5
  }
]
```

**Response (200) - Tags:**

```json
[
  { "tag": "tutorial", "count": 12 },
  { "tag": "api", "count": 8 },
  { "tag": "setup", "count": 5 }
]
```

**Errors:**
- `400` - missing required `q` parameter for this mode

Results are filtered by user roles. Snippets (in non-tag modes) contain highlighted matching terms.

---

## Webhook

### POST /api/webhook/push

Receives Git push events and triggers an index rebuild.

**Headers:**
- `X-Webhook-Secret` - must match configured `webhook_secret`

**Response:**
- `200` - rebuild triggered
- `401` - invalid or missing secret

The rebuild runs asynchronously. The endpoint returns immediately.

---

## Error format

All error responses follow a consistent format:

```json
{
  "error": "not_found",
  "message": "Document not found: guides/nonexistent.md"
}
```

See also: [[architecture]], [[setup]]

#api #reference
