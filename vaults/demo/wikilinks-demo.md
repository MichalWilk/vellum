---
title: Wikilinks Demo
tags: [demo, wikilinks, graph]
---

# Wikilinks Demo

Wikilinks are Vellum's mechanism for connecting documents. They create navigable links and power the graph view.

## Basic syntax

A wikilink is written with double square brackets:

```markdown
See [[markdown-showcase]] for all features.
```

This renders as a clickable link to the `markdown-showcase` document.

## Display text

You can customize the display text with a pipe:

```markdown
Check the [[markdown-showcase|feature showcase]] for examples.
Check the [[index|demo home page]] for an overview.
```

Renders as: Check the [[markdown-showcase|feature showcase]] for examples.

## How wikilinks build the graph

Every `[[wikilink]]` in your documents creates an edge in the graph:

```
This document ──[[markdown-showcase]]──> markdown-showcase.md
This document ──[[index]]──────────────> index.md
```

The graph view visualizes these connections. Documents are nodes, wikilinks are edges.

## Cross-document linking

Link to any document in the vault by its filename (without extension):

- [[index]] - the vault index page
- [[markdown-showcase]] - Markdown feature demo

## Link resolution

Vellum resolves wikilinks by matching the link text against document filenames:

1. Strip the extension if present
2. Search for a matching `.md` file in the vault
3. If found, create a navigable link and a graph edge
4. If not found, render as plain text (no broken links)

## Bidirectional awareness

The graph tracks links in both directions. If document A links to document B, both appear connected in the graph. This lets you discover ==backlinks== - documents that reference the current page.

## Role-filtered graph

The graph respects access control. If a linked document is in a restricted directory and the user lacks the required role:

- The node is hidden from the graph
- The edge is removed
- The wikilink renders as plain text instead of a clickable link

This ensures no information leaks through the graph view.

## Tips for effective linking

> [!tip]
> - Link generously - more links make the graph more useful
> - Use descriptive display text: `[[setup|development setup guide]]` rather than just `[[setup]]`
> - Link from context - place links where they naturally fit in the text
> - Check the graph view periodically to find orphaned documents (nodes with no edges)

## Example link map

This document links to:
- [[index]] (demo home)
- [[markdown-showcase]] (feature showcase)

And the [[markdown-showcase]] links back here, creating a bidirectional connection visible in the graph.

#wikilinks #graph #demo
