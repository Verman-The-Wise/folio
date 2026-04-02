use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub site: SiteConfig,
    #[serde(default)]
    pub feed: FeedConfig,
    #[serde(default)]
    pub nav: Vec<NavLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub title:       String,
    pub description: String,
    pub author:      String,
    #[serde(default = "default_base_url")]
    pub base_url:    String,
    /// Markdown/HTML rendered above the post feed on the home page
    #[serde(default)]
    pub home_header: Option<String>,
    /// Optional image URL shown at the very top of the home page
    #[serde(default)]
    pub home_image: Option<String>,
    /// Path to a logo image shown in the nav (optional)
    #[serde(default)]
    pub logo:        Option<String>,
    #[serde(default)]
    pub footer:      Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavLink {
    pub label: String,
    pub url:   String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    #[serde(default = "default_home_count")]
    pub home_count:    usize,
    #[serde(default = "default_excerpt_words")]
    pub excerpt_words: usize,
}

impl Default for FeedConfig {
    fn default() -> Self { Self { home_count: 12, excerpt_words: 55 } }
}

fn default_base_url()      -> String { "/".into() }
fn default_home_count()    -> usize  { 12 }
fn default_excerpt_words() -> usize  { 55 }

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Path::new("folio.toml");
        if !path.exists() {
            return Err("folio.toml not found. Run `folio init` first.".into());
        }
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    }

    pub fn default_toml() -> &'static str {
        r#"[site]
title = "folio"
description = "a personal wiki and journal"
author = "your name"
base_url = "/"
# home_header shown above the post feed — Markdown or plain HTML
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
home_count = 12
excerpt_words = 55
"#
    }
}
