# folio

A flat-file personal journal and wiki — static site generator written in Rust.

Aesthetic influenced by [XXIIVV](https://wiki.xxiivv.com) and [wellobserve](https://www.wellobserve.com): monochrome, paper-and-ink, no chrome.

---

## quick start

```bash
cargo build --release

mkdir my-site && cd my-site
cp ../folio/folio.toml .          # or run folio init
folio build
folio serve 3000
# → http://localhost:3000
```

## commands

```
folio init                 scaffold a new project in the current directory
folio build                build static site → dist/
folio serve [port]         build and serve locally (default: 3000)
folio new post  <title>    create content/posts/<slug>.md
folio new wiki  <title>    create content/wiki/<slug>.md
folio new page  <title>    create content/pages/<slug>.md
```

## project layout

```
my-site/
├── folio.toml               site config
├── content/
│   ├── posts/               journal entries (.md)
│   ├── wiki/                wiki pages (.md)
│   └── pages/               standalone pages — about, colophon, etc. (.md)
├── templates/               override any template (optional)
├── static/                  copied verbatim into dist/
└── dist/                    generated output (git-ignored)
```

## content format

All files are Markdown with TOML front matter (`+++` delimiters).

### post

```toml
+++
title = "entry title"
date = "2024-03-15"
tags = ["tag1", "tag2"]
draft = false
pinned = false          # float to top of feed
+++

Content here. Link to [[wiki pages]] or [[other posts]].
```

### wiki page

```toml
+++
title = "page title"
description = "one liner shown in the wiki index"
tags = ["tag1"]
category = "general"    # groups pages in wiki index
updated = "2024-03-01"
draft = false
+++
```

### standalone page (about, colophon, etc.)

```toml
+++
title = "about"
draft = false
+++
```

Pages live at `/<slug>/` — e.g. `/about/`, `/colophon/`.

## wikilinks

Link between any content type using `[[slug]]` or `[[slug|display text]]`:

```markdown
See [[on-attention]] for more.
Also: [[tools-for-thinking|my thinking toolkit]].
```

- Resolves across posts, wiki pages, and standalone pages
- Dead links render with a dashed underline (not broken HTML)
- Every page shows an outbound "links from this page" section
- Every page shows an inbound "referenced by" backlinks section

## features

| Feature | Detail |
|---------|--------|
| Wikilinks | `[[slug]]` / `[[slug\|text]]` across all content types |
| Backlinks | auto-computed, shown on every page |
| Search | ⌘K overlay, searches posts + wiki full-text |
| Image feed | posts with only images render as thumbnail grid on home |
| Home header | custom HTML/Markdown above the feed via `folio.toml` |
| Standalone pages | `content/pages/` — about, colophon, etc. |
| Post tag pages | `/tags/<tag>/` |
| Wiki tag pages | `/wiki/tags/<tag>/` |
| RSS | `/feed.xml` always generated |
| TOC | auto-generated sidebar on wiki pages |
| Templates | override any in `templates/` (MiniJinja/Jinja2 syntax) |

## configuration (`folio.toml`)

```toml
[site]
title = "folio"
description = "a personal wiki and journal"
author = "your name"
base_url = "/"
# Markdown/HTML shown above post feed on home page
home_header = "thinking out loud."
# logo = "/static/logo.png"
# footer = "name — <a href='/about/'>about</a>"

[[nav]]
label = "journal"
url = "/posts/"

[[nav]]
label = "wiki"
url = "/wiki/"

[[nav]]
label = "about"
url = "/about/"

[feed]
home_count = 12       # posts on home page
excerpt_words = 55    # words in feed excerpt
```

## templates

Drop any of these into `templates/` to override:

| File | Page |
|------|------|
| `base.html` | HTML shell, nav, footer |
| `home.html` | home feed |
| `post.html` | individual post |
| `wiki_index.html` | wiki listing |
| `wiki_page.html` | individual wiki page |
| `tag.html` | post and wiki tag pages |
| `page.html` | standalone pages |

Templates use [MiniJinja](https://docs.rs/minijinja) (Jinja2-compatible) syntax.

## dependencies

| Crate | Purpose |
|-------|---------|
| `pulldown-cmark` | Markdown → HTML |
| `toml` + `serde` | front matter parsing |
| `minijinja` | templates |
| `chrono` | dates |
| `walkdir` | directory traversal |
| `slug` | URL slug generation |
| `colored` | CLI output |

Requires Rust 1.75+. `cargo build --release` produces a single static binary.

## license

MIT
