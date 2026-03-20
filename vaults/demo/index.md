---
title: Demo Vault
tags: [demo, showcase]
---

# Welcome to the Demo Vault

This vault demonstrates Vellum's features and capabilities. It contains examples of all supported Markdown features, wikilinks, and role-based access control.

## What's inside

- [[markdown-showcase]] - A comprehensive demo of every Markdown feature Vellum supports
- [[wikilinks-demo]] - How document linking and the graph view work
- **protected/** - A restricted section only visible to users with the `dev` role

> [!tip] Try the graph view
> Navigate to the graph view to see how these documents are connected through wikilinks. Each `[[link]]` creates an edge in the graph.

## About access control

This vault uses mixed access rules:

- The root is public (`roles = ["*"]`) - anyone can see this page
- The `protected/` directory requires the `dev` role
- If you don't have the `dev` role, you won't even see the protected section in the file tree

This demonstrates how Vellum hides unauthorized content completely - there's no "access denied" page, the content simply doesn't appear.

## Try searching

Press `Cmd+K` (or `Ctrl+K`) to open the command palette and search across all documents you have access to. Try searching for "callout" or "wikilink" to find relevant demo content.

#demo #showcase
