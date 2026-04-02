+++
title = "colophon"
draft = false
description = "how this site is made"
+++

# colophon

Everything here is written in plain Markdown, stored in a git repository, and compiled to static HTML by a Rust binary called **folio**.

## structure

```
content/
  posts/    journal entries, newest first
  wiki/     reference pages, linked by topic
  pages/    standalone pages (this one, about, etc.)
```

## wikilinks

Any page can reference any other using `[[page title]]` or `[[slug|display text]]`. The build resolves these to real URLs. Dead links render with a dashed underline rather than breaking.

## search

Full-text search across posts and wiki runs client-side against a JSON index generated at build time. No server, no third-party service.

## hosting

Static files. Deploy anywhere — Netlify, Cloudflare Pages, a $5 VPS, a USB stick if needed.
