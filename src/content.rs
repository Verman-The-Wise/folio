use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use slug::slugify;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ── Front matter structs ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostMeta {
    pub title: String,
    #[serde(default)] pub date:        Option<String>,
    #[serde(default)] pub tags:        Vec<String>,
    #[serde(default)] pub draft:       bool,
    #[serde(default)] pub slug:        Option<String>,
    #[serde(default)] pub description: Option<String>,
    #[serde(default)] pub pinned:      bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WikiMeta {
    pub title: String,
    #[serde(default)] pub description: Option<String>,
    #[serde(default)] pub tags:        Vec<String>,
    #[serde(default)] pub slug:        Option<String>,
    #[serde(default)] pub draft:       bool,
    #[serde(default)] pub category:    Option<String>,
    #[serde(default)] pub updated:     Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageMeta {
    pub title: String,
    #[serde(default)] pub slug:        Option<String>,
    #[serde(default)] pub draft:       bool,
    #[serde(default)] pub description: Option<String>,
    #[serde(default)] pub in_nav:      bool,
}

// ── Output structs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub title:           String,
    pub slug:            String,
    pub date:            DateTime<Utc>,
    pub date_display:    String,
    pub date_iso:        String,
    pub tags:            Vec<String>,
    pub draft:           bool,
    pub pinned:          bool,
    pub description:     Option<String>,
    pub raw_content:     String,
    pub html_content:    String,
    pub excerpt_html:    String,
    pub images:          Vec<String>,
    pub word_count:      usize,
    pub reading_minutes: usize,
    pub wikilinks:       Vec<WikilinkRef>,
    pub backlinks:       Vec<BacklinkRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WikiPage {
    pub title:        String,
    pub slug:         String,
    pub description:  Option<String>,
    pub tags:         Vec<String>,
    pub draft:        bool,
    pub category:     Option<String>,
    pub updated:      Option<String>,
    pub raw_content:  String,
    pub html_content: String,
    pub toc:          Vec<TocEntry>,
    pub wikilinks:    Vec<WikilinkRef>,
    pub backlinks:    Vec<BacklinkRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Page {
    pub title:        String,
    pub slug:         String,
    pub description:  Option<String>,
    pub draft:        bool,
    pub in_nav:       bool,
    pub raw_content:  String,
    pub html_content: String,
    pub wikilinks:    Vec<WikilinkRef>,
    pub backlinks:    Vec<BacklinkRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TocEntry {
    pub level:  u8,
    pub text:   String,
    pub anchor: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WikilinkRef {
    pub text: String,
    pub url:  String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacklinkRef {
    pub title: String,
    pub url:   String,
    pub kind:  String,
}

// ── Link registry ─────────────────────────────────────────────────────────────

pub struct LinkRegistry {
    pub entries: HashMap<String, (String, String, String)>, // key → (url, title, kind)
}

impl LinkRegistry {
    pub fn new() -> Self { Self { entries: HashMap::new() } }

    pub fn register(&mut self, slug: &str, url: &str, title: &str, kind: &str) {
        self.entries.insert(slug.to_lowercase(), (url.into(), title.into(), kind.into()));
        let ts = slugify(title);
        if ts != slug {
            self.entries.entry(ts).or_insert_with(|| (url.into(), title.into(), kind.into()));
        }
    }

    pub fn resolve(&self, target: &str) -> Option<&(String, String, String)> {
        self.entries.get(&slugify(target))
            .or_else(|| self.entries.get(&target.to_lowercase()))
    }
}

// ── Wikilink handling ─────────────────────────────────────────────────────────

/// Replace [[target]] / [[target|display]] with HTML links or dead-link spans.
pub fn resolve_wikilinks(md: &str, registry: &LinkRegistry) -> (String, Vec<WikilinkRef>) {
    let bytes = md.as_bytes();
    let len   = bytes.len();
    let mut spans: Vec<(usize, usize, String, String)> = Vec::new();
    let mut i = 0;
    while i + 1 < len {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            let mut j = i + 2;
            while j + 1 < len {
                if bytes[j] == b']' && bytes[j + 1] == b']' {
                    let inner = &md[i + 2..j];
                    let parts: Vec<&str> = inner.splitn(2, '|').collect();
                    let target  = parts[0].trim().to_string();
                    let display = parts.get(1).map(|s| s.trim().to_string())
                        .unwrap_or_else(|| target.clone());
                    spans.push((i, j + 2, target, display));
                    i = j + 2;
                    break;
                }
                j += 1;
            }
            if j + 1 >= len { i += 1; }
        } else {
            i += 1;
        }
    }

    let mut result = md.to_string();
    let mut refs   = Vec::new();
    for (start, end, target, display) in spans.iter().rev() {
        let replacement = if let Some((url, _, kind)) = registry.resolve(target) {
            refs.push(WikilinkRef { text: display.clone(), url: url.clone(), kind: kind.clone() });
            format!("[{}]({})", display, url)
        } else {
            refs.push(WikilinkRef { text: display.clone(), url: String::new(), kind: "unresolved".into() });
            format!("<span class=\"wikilink-dead\" title=\"'{}' not found\">{}</span>", target, display)
        };
        result.replace_range(start..end, &replacement);
    }
    refs.reverse();
    (result, refs)
}

// ── Markdown helpers ──────────────────────────────────────────────────────────

fn parse_front_matter(source: &str) -> Result<(toml::Value, &str), Box<dyn std::error::Error>> {
    let source = source.trim_start_matches('\u{feff}');
    if let Some(rest) = source.strip_prefix("+++") {
        if let Some(end) = rest.find("\n+++") {
            return Ok((toml::from_str(rest[..end].trim())?, rest[end + 4..].trim_start_matches('\n')));
        }
    }
    Ok((toml::Value::Table(Default::default()), source))
}

pub fn markdown_to_html(md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_SMART_PUNCTUATION);
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(md, opts));
    out
}

fn add_heading_anchors(html: &str) -> String {
    let mut out = html.to_string();
    for level in 1..=4u8 {
        let open  = format!("<h{}>", level);
        let close = format!("</h{}>", level);
        let mut result = String::new();
        let mut rest   = out.as_str();
        while let Some(start) = rest.find(&open) {
            result.push_str(&rest[..start]);
            let after = &rest[start + open.len()..];
            if let Some(end) = after.find(&close) {
                let text   = &after[..end];
                let anchor = slugify(text);
                result.push_str(&format!("<h{} id=\"{}\">{}</h{}>", level, anchor, text, level));
                rest = &after[end + close.len()..];
            } else {
                result.push_str(&open);
                rest = after;
            }
        }
        result.push_str(rest);
        out = result;
    }
    out
}

fn extract_toc(md: &str) -> Vec<TocEntry> {
    let mut entries = Vec::new();
    for line in md.lines() {
        if let Some(r)      = line.strip_prefix("#### ") { entries.push(TocEntry { level: 4, text: r.trim().into(), anchor: slugify(r.trim()) }); }
        else if let Some(r) = line.strip_prefix("### ")  { entries.push(TocEntry { level: 3, text: r.trim().into(), anchor: slugify(r.trim()) }); }
        else if let Some(r) = line.strip_prefix("## ")   { entries.push(TocEntry { level: 2, text: r.trim().into(), anchor: slugify(r.trim()) }); }
        else if let Some(r) = line.strip_prefix("# ")    { entries.push(TocEntry { level: 1, text: r.trim().into(), anchor: slugify(r.trim()) }); }
    }
    entries
}

fn extract_images(html: &str) -> Vec<String> {
    let mut imgs = Vec::new();
    let mut rest = html;
    while let Some(pos) = rest.find("<img") {
        let after = &rest[pos..];
        if let Some(sp) = after.find("src=\"") {
            let val = &after[sp + 5..];
            if let Some(eq) = val.find('"') {
                imgs.push(val[..eq].to_string());
            }
        }
        rest = &rest[pos + 4..];
    }
    imgs
}

fn word_count(text: &str) -> usize { text.split_whitespace().count() }

fn excerpt(md: &str, max_words: usize) -> String {
    let first_para = md.lines()
        .take_while(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let words: Vec<&str> = first_para.split_whitespace().collect();
    let s = if words.len() > max_words {
        format!("{}\u{2026}", words[..max_words].join(" "))
    } else {
        words.join(" ")
    };
    markdown_to_html(&s)
}

fn parse_date(s: &str) -> DateTime<Utc> {
    if let Ok(d)  = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Utc.from_utc_datetime(&d.and_hms_opt(0, 0, 0).unwrap());
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc);
    }
    Utc::now()
}

fn derive_slug(path: &Path, title: &str, explicit: Option<String>) -> String {
    explicit.unwrap_or_else(|| {
        path.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| slugify(s))
            .unwrap_or_else(|| slugify(title))
    })
}

// ── Registry builder ──────────────────────────────────────────────────────────

pub fn build_registry(posts_dir: &str, wiki_dir: &str, pages_dir: &str) -> LinkRegistry {
    let mut reg = LinkRegistry::new();

    fn scan(dir: &str, prefix: &str, kind: &str, reg: &mut LinkRegistry) {
        let p = Path::new(dir);
        if !p.exists() { return; }
        for e in WalkDir::new(p).follow_links(true).into_iter().flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some("md") { continue; }
            let Ok(src) = fs::read_to_string(path) else { continue };
            let Ok((meta, _)) = parse_front_matter(&src) else { continue };
            if meta.get("draft").and_then(|v| v.as_bool()).unwrap_or(false) { continue; }
            let title = meta.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let explicit = meta.get("slug").and_then(|v| v.as_str()).map(|s| s.to_string());
            let slug = explicit.unwrap_or_else(||
                path.file_stem().and_then(|s| s.to_str()).map(slugify).unwrap_or_default());
            reg.register(&slug, &format!("{}{}/", prefix, slug), &title, kind);
        }
    }

    scan(posts_dir, "/posts/", "post",  &mut reg);
    scan(wiki_dir,  "/wiki/",  "wiki",  &mut reg);
    scan(pages_dir, "/",       "page",  &mut reg);
    reg
}

// ── Loaders ───────────────────────────────────────────────────────────────────

pub fn load_posts(excerpt_words: usize, registry: &LinkRegistry) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
    let dir = Path::new("content/posts");
    if !dir.exists() { return Ok(vec![]); }
    let mut posts = Vec::new();
    for entry in WalkDir::new(dir).follow_links(true) {
        let entry = entry?;
        let path  = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") { continue; }
        let source = fs::read_to_string(path)?;
        let (meta_val, body) = parse_front_matter(&source)?;
        let meta: PostMeta = meta_val.try_into()?;
        if meta.draft { continue; }
        let slug = derive_slug(path, &meta.title, meta.slug.clone());
        let date = meta.date.as_deref().map(parse_date).unwrap_or_else(Utc::now);
        let (resolved_md, wikilinks) = resolve_wikilinks(body, registry);
        let html   = add_heading_anchors(&markdown_to_html(&resolved_md));
        let images = extract_images(&html);
        let wc     = word_count(body);
        posts.push(Post {
            title: meta.title, slug, date,
            date_display:    date.format("%Y.%m.%d").to_string(),
            date_iso:        date.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            tags: meta.tags, draft: meta.draft, pinned: meta.pinned,
            description: meta.description,
            raw_content:  body.to_string(),
            html_content: html,
            excerpt_html: excerpt(body, excerpt_words),
            images, word_count: wc,
            reading_minutes: (wc / 200).max(1),
            wikilinks, backlinks: vec![],
        });
    }
    posts.sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.date.cmp(&a.date)));
    Ok(posts)
}

pub fn load_wiki(registry: &LinkRegistry) -> Result<Vec<WikiPage>, Box<dyn std::error::Error>> {
    let dir = Path::new("content/wiki");
    if !dir.exists() { return Ok(vec![]); }
    let mut pages = Vec::new();
    for entry in WalkDir::new(dir).follow_links(true) {
        let entry = entry?;
        let path  = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") { continue; }
        let source = fs::read_to_string(path)?;
        let (meta_val, body) = parse_front_matter(&source)?;
        let meta: WikiMeta = meta_val.try_into()?;
        if meta.draft { continue; }
        let slug = derive_slug(path, &meta.title, meta.slug.clone());
        let toc  = extract_toc(body);
        let (resolved_md, wikilinks) = resolve_wikilinks(body, registry);
        let html = add_heading_anchors(&markdown_to_html(&resolved_md));
        pages.push(WikiPage {
            title: meta.title, slug, description: meta.description,
            tags: meta.tags, draft: meta.draft, category: meta.category,
            updated: meta.updated, raw_content: body.to_string(),
            html_content: html, toc, wikilinks, backlinks: vec![],
        });
    }
    pages.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(pages)
}

pub fn load_pages(registry: &LinkRegistry) -> Result<Vec<Page>, Box<dyn std::error::Error>> {
    let dir = Path::new("content/pages");
    if !dir.exists() { return Ok(vec![]); }
    let mut pages = Vec::new();
    for entry in WalkDir::new(dir).follow_links(true) {
        let entry = entry?;
        let path  = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") { continue; }
        let source = fs::read_to_string(path)?;
        let (meta_val, body) = parse_front_matter(&source)?;
        let meta: PageMeta = meta_val.try_into()?;
        if meta.draft { continue; }
        let slug = derive_slug(path, &meta.title, meta.slug.clone());
        let (resolved_md, wikilinks) = resolve_wikilinks(body, registry);
        let html = add_heading_anchors(&markdown_to_html(&resolved_md));
        pages.push(Page {
            title: meta.title, slug, description: meta.description,
            draft: meta.draft, in_nav: meta.in_nav,
            raw_content: body.to_string(), html_content: html,
            wikilinks, backlinks: vec![],
        });
    }
    pages.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(pages)
}

// ── Backlink computation ──────────────────────────────────────────────────────

pub fn compute_backlinks(posts: &mut Vec<Post>, wiki: &mut Vec<WikiPage>, pages: &mut Vec<Page>) {
    let mut map: HashMap<String, Vec<BacklinkRef>> = HashMap::new();

    macro_rules! gather {
        ($items:expr, $prefix:expr, $kind:expr) => {
            for item in $items.iter() {
                let src_url = format!("{}{}/", $prefix, item.slug);
                for wl in &item.wikilinks {
                    if wl.kind != "unresolved" && !wl.url.is_empty() {
                        map.entry(wl.url.clone()).or_default().push(BacklinkRef {
                            title: item.title.clone(),
                            url:   src_url.clone(),
                            kind:  $kind.to_string(),
                        });
                    }
                }
            }
        };
    }
    gather!(posts, "/posts/", "post");
    gather!(wiki,  "/wiki/",  "wiki");
    gather!(pages, "/",       "page");

    macro_rules! apply {
        ($items:expr, $prefix:expr) => {
            for item in $items.iter_mut() {
                let url = format!("{}{}/", $prefix, item.slug);
                if let Some(bls) = map.get(&url) { item.backlinks = bls.clone(); }
            }
        };
    }
    apply!(posts, "/posts/");
    apply!(wiki,  "/wiki/");
    apply!(pages, "/");
}

// ── Scaffold helper ───────────────────────────────────────────────────────────

pub fn create_new(kind: &str, title: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let now  = Utc::now();
    let slug = slugify(title);
    let (dir, front) = match kind {
        "wiki" => (PathBuf::from("content/wiki"), format!(
            "+++\ntitle = \"{}\"\ndescription = \"\"\ntags = []\ncategory = \"general\"\n+++\n\n# {}\n\n",
            title, title)),
        "page" => (PathBuf::from("content/pages"), format!(
            "+++\ntitle = \"{}\"\ndraft = false\n+++\n\n# {}\n\n",
            title, title)),
        _ => (PathBuf::from("content/posts"), format!(
            "+++\ntitle = \"{}\"\ndate = \"{}\"\ntags = []\ndraft = false\n+++\n\n",
            title, now.format("%Y-%m-%d"))),
    };
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.md", slug));
    if path.exists() { return Err(format!("{} already exists", path.display()).into()); }
    fs::write(&path, front)?;
    Ok(path)
}
