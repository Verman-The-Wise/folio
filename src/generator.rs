use crate::config::Config;
use crate::content::*;
use minijinja::{context, Environment};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub struct BuildStats { pub posts: usize, pub wiki: usize, pub pages: usize }

pub fn build() -> Result<BuildStats, Box<dyn std::error::Error>> {
    let cfg = Config::load()?;

    // 1. Build slug registry (first pass, no content loaded yet)
    let registry = build_registry("content/posts", "content/wiki", "content/pages");

    // 2. Load all content (wikilinks resolved against registry)
    let mut posts = load_posts(cfg.feed.excerpt_words, &registry)?;
    let mut wiki  = load_wiki(&registry)?;
    let mut pages = load_pages(&registry)?;

    // 3. Compute backlinks
    compute_backlinks(&mut posts, &mut wiki, &mut pages);

    // 4. Prepare dist/
    let dist = Path::new("dist");
    if dist.exists() { fs::remove_dir_all(dist)?; }
    fs::create_dir_all(dist.join("posts"))?;
    fs::create_dir_all(dist.join("wiki"))?;
    fs::create_dir_all(dist.join("tags"))?;
    fs::create_dir_all(dist.join("wiki/tags"))?;

    // Copy user static dir
    if Path::new("static").exists() {
        copy_dir(Path::new("static"), &dist.join("static"))?;
    }
    // Write embedded assets (overwrite with embedded versions)
    let css_dir = dist.join("static/css");
    fs::create_dir_all(&css_dir)?;
    fs::write(css_dir.join("main.css"), MAIN_CSS)?;
    let js_dir = dist.join("static/js");
    fs::create_dir_all(&js_dir)?;
    fs::write(js_dir.join("main.js"), MAIN_JS)?;

    // 5. Setup template env
    let mut env = Environment::new();
    macro_rules! tmpl {
        ($name:expr, $embedded:expr) => {
            env.add_template_owned(
                $name,
                fs::read_to_string(format!("templates/{}", $name))
                    .unwrap_or_else(|_| $embedded.to_string()),
            )?
        };
    }
    tmpl!("base.html",       TMPL_BASE);
    tmpl!("home.html",       TMPL_HOME);
    tmpl!("post.html",       TMPL_POST);
    tmpl!("wiki_index.html", TMPL_WIKI_INDEX);
    tmpl!("wiki_page.html",  TMPL_WIKI_PAGE);
    tmpl!("tag.html",        TMPL_TAG);
    tmpl!("page.html",       TMPL_PAGE);

    // Build search index (posts + wiki, written to dist/)
    let search_index = build_search_index(&posts, &wiki);
    fs::write(dist.join("search-index.json"), serde_json::to_string(&search_index)?)?;

    let recent: Vec<_> = posts.iter().take(cfg.feed.home_count).collect();

    // home_header: render as markdown if set
    let home_header_html = cfg.site.home_header.as_deref()
        .map(|s| crate::content::markdown_to_html(s))
        .unwrap_or_default();
    let home_image = cfg.site.home_image.clone().unwrap_or_default();

    // Partition posts: image-only (thumbnail grid) vs text (feed rows).
    // recent is Vec<&Post>, so partition it directly — no extra .iter() needed.
    let (img_recent, txt_recent): (Vec<&Post>, Vec<&Post>) =
        recent.into_iter().partition(|p| !p.images.is_empty() && p.word_count < 30);

    // ── Home ──────────────────────────────────────────────────────────────────
    let html = env.get_template("home.html")?.render(context! {
        site        => &cfg.site,
        nav         => &cfg.nav,
        image_posts     => &img_recent,
        text_posts      => &txt_recent,
        all_posts_count => posts.len(),
        show_all        => posts.len() > cfg.feed.home_count,
        wiki_pages      => &wiki,
        home_header     => home_header_html,
        home_image      => &home_image,
        page_title      => &cfg.site.title,
    })?;
    fs::write(dist.join("index.html"), html)?;

    // ── All posts archive ─────────────────────────────────────────────────────
    let (img_all, txt_all): (Vec<&Post>, Vec<&Post>) =
        posts.iter().partition(|p| !p.images.is_empty() && p.word_count < 30);
    let html = env.get_template("home.html")?.render(context! {
        site        => &cfg.site,
        nav         => &cfg.nav,
        image_posts     => &img_all,
        text_posts      => &txt_all,
        all_posts_count => posts.len(),
        show_all        => false,
        wiki_pages      => &wiki,
        home_header     => "",
        is_archive      => true,
        page_title  => format!("journal — {}", cfg.site.title),
    })?;
    fs::write(dist.join("posts/index.html"), html)?;

    // ── Individual posts ──────────────────────────────────────────────────────
    let n = posts.len();
    for (i, post) in posts.iter().enumerate() {
        let prev = if i + 1 < n { Some(&posts[i + 1]) } else { None };
        let next = if i > 0     { Some(&posts[i - 1]) } else { None };
        let html = env.get_template("post.html")?.render(context! {
            site       => &cfg.site,
            nav        => &cfg.nav,
            post       => post,
            prev_post  => prev,
            next_post  => next,
            page_title => format!("{} — {}", post.title, cfg.site.title),
        })?;
        let d = dist.join("posts").join(&post.slug);
        fs::create_dir_all(&d)?;
        fs::write(d.join("index.html"), html)?;
    }

    // ── Wiki index ────────────────────────────────────────────────────────────
    let mut cats: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for p in &wiki {
        cats.entry(p.category.clone().unwrap_or_else(|| "general".into()))
            .or_default()
            .push(p);
    }
    let cats_json: Vec<_> = cats.iter()
        .map(|(k, v)| serde_json::json!({"name": k, "pages": v}))
        .collect();
    let html = env.get_template("wiki_index.html")?.render(context! {
        site       => &cfg.site,
        nav        => &cfg.nav,
        wiki_pages => &wiki,
        categories => cats_json,
        page_title => format!("wiki — {}", cfg.site.title),
    })?;
    fs::write(dist.join("wiki/index.html"), html)?;

    // ── Individual wiki pages ─────────────────────────────────────────────────
    for page in &wiki {
        let html = env.get_template("wiki_page.html")?.render(context! {
            site       => &cfg.site,
            nav        => &cfg.nav,
            page       => page,
            all_wiki   => &wiki,
            page_title => format!("{} — wiki — {}", page.title, cfg.site.title),
        })?;
        let d = dist.join("wiki").join(&page.slug);
        fs::create_dir_all(&d)?;
        fs::write(d.join("index.html"), html)?;
    }

    // ── Post tag pages ────────────────────────────────────────────────────────
    let mut post_tags: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for p in &posts {
        for t in &p.tags { post_tags.entry(t.clone()).or_default().push(p); }
    }
    for (tag, tagged) in &post_tags {
        let sl = slug::slugify(tag);
        let html = env.get_template("tag.html")?.render(context! {
            site       => &cfg.site,
            nav        => &cfg.nav,
            tag        => tag,
            posts      => tagged,
            wiki_posts => Vec::<&WikiPage>::new(),
            kind       => "posts",
            page_title => format!("#{} — {}", tag, cfg.site.title),
        })?;
        let d = dist.join("tags").join(&sl);
        fs::create_dir_all(&d)?;
        fs::write(d.join("index.html"), html)?;
    }

    // ── Wiki tag pages ────────────────────────────────────────────────────────
    let mut wiki_tags: BTreeMap<String, Vec<_>> = BTreeMap::new();
    for p in &wiki {
        for t in &p.tags { wiki_tags.entry(t.clone()).or_default().push(p); }
    }
    for (tag, tagged) in &wiki_tags {
        let sl = slug::slugify(tag);
        let html = env.get_template("tag.html")?.render(context! {
            site       => &cfg.site,
            nav        => &cfg.nav,
            tag        => tag,
            posts      => Vec::<&Post>::new(),
            wiki_posts => tagged,
            kind       => "wiki",
            page_title => format!("#{} — wiki — {}", tag, cfg.site.title),
        })?;
        let d = dist.join("wiki/tags").join(&sl);
        fs::create_dir_all(&d)?;
        fs::write(d.join("index.html"), html)?;
    }

    // ── Standalone pages ──────────────────────────────────────────────────────
    for page in &pages {
        let html = env.get_template("page.html")?.render(context! {
            site       => &cfg.site,
            nav        => &cfg.nav,
            page       => page,
            page_title => format!("{} — {}", page.title, cfg.site.title),
        })?;
        let d = dist.join(&page.slug);
        fs::create_dir_all(&d)?;
        fs::write(d.join("index.html"), html)?;
    }

    // ── RSS ───────────────────────────────────────────────────────────────────
    fs::write(dist.join("feed.xml"), rss(&cfg, &posts))?;

    Ok(BuildStats { posts: posts.len(), wiki: wiki.len(), pages: pages.len() })
}

// ── Search index ──────────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct SearchEntry {
    title: String,
    url:   String,
    kind:  String,
    tags:  Vec<String>,
    body:  String,
}

fn build_search_index(posts: &[Post], wiki: &[WikiPage]) -> Vec<SearchEntry> {
    let mut idx = Vec::new();
    for p in posts {
        idx.push(SearchEntry {
            title: p.title.clone(),
            url:   format!("/posts/{}/", p.slug),
            kind:  "post".into(),
            tags:  p.tags.clone(),
            body:  p.raw_content.chars().take(400).collect(),
        });
    }
    for p in wiki {
        idx.push(SearchEntry {
            title: p.title.clone(),
            url:   format!("/wiki/{}/", p.slug),
            kind:  "wiki".into(),
            tags:  p.tags.clone(),
            body:  p.raw_content.chars().take(400).collect(),
        });
    }
    idx
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn copy_dir(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dst)?;
    for e in walkdir::WalkDir::new(src) {
        let e = e?;
        let rel = e.path().strip_prefix(src)?;
        let t = dst.join(rel);
        if e.file_type().is_dir() { fs::create_dir_all(&t)?; }
        else { fs::copy(e.path(), &t)?; }
    }
    Ok(())
}

fn rss(cfg: &Config, posts: &[Post]) -> String {
    let items: String = posts.iter().take(20).map(|p| format!(
        "<item><title><![CDATA[{}]]></title><link>{}/posts/{}/</link>\
         <pubDate>{}</pubDate><description><![CDATA[{}]]></description></item>",
        p.title,
        cfg.site.base_url.trim_end_matches('/'),
        p.slug,
        p.date.format("%a, %d %b %Y %H:%M:%S +0000"),
        p.excerpt_html,
    )).collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <rss version=\"2.0\"><channel>\
         <title>{}</title><link>{}</link><description>{}</description>\
         {}</channel></rss>",
        cfg.site.title, cfg.site.base_url, cfg.site.description, items
    )
}

pub fn init_project() -> Result<(), Box<dyn std::error::Error>> {
    for d in &["content/posts","content/wiki","content/pages","templates","static/css","static/js"] {
        fs::create_dir_all(d)?;
    }
    if !Path::new("folio.toml").exists() {
        fs::write("folio.toml", Config::default_toml())?;
    }
    fs::write("content/posts/first-entry.md", SAMPLE_POST)?;
    fs::write("content/wiki/getting-started.md", SAMPLE_WIKI)?;
    fs::write("content/pages/about.md", SAMPLE_PAGE)?;
    println!("  folio.toml");
    println!("  content/posts/first-entry.md");
    println!("  content/wiki/getting-started.md");
    println!("  content/pages/about.md");
    Ok(())
}

// ── Embedded assets ───────────────────────────────────────────────────────────

const MAIN_CSS:        &str = include_str!("../static/css/main.css");
const MAIN_JS:         &str = include_str!("../static/js/main.js");
const TMPL_BASE:       &str = include_str!("../templates/base.html");
const TMPL_HOME:       &str = include_str!("../templates/home.html");
const TMPL_POST:       &str = include_str!("../templates/post.html");
const TMPL_WIKI_INDEX: &str = include_str!("../templates/wiki_index.html");
const TMPL_WIKI_PAGE:  &str = include_str!("../templates/wiki_page.html");
const TMPL_TAG:        &str = include_str!("../templates/tag.html");
const TMPL_PAGE:       &str = include_str!("../templates/page.html");

const SAMPLE_POST: &str = "\
+++
title = \"first entry\"
date = \"2024-01-01\"
tags = [\"meta\"]
draft = false
+++

This is the first entry. You can link to wiki pages with [[getting-started]] like this.
";

const SAMPLE_WIKI: &str = "\
+++
title = \"getting started\"
description = \"how to use folio\"
tags = [\"meta\"]
category = \"documentation\"
+++

# getting started

See the [[first-entry]] post for an example of wikilinks in action.

Run `folio new post \"my title\"` to create a new post.
";

const SAMPLE_PAGE: &str = "\
+++
title = \"about\"
draft = false
+++

# about

This site is built with **folio**.
";
