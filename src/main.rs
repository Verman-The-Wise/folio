mod config;
mod content;
mod generator;
mod server;

use colored::*;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("build");

    match cmd {
        "build" => {
            println!("{}", "▸ folio — building…".cyan().bold());
            match generator::build() {
                Ok(s) => println!(
                    "{} {} posts  {} wiki  {} pages → {}",
                    "✔".green().bold(),
                    s.posts.to_string().yellow(),
                    s.wiki.to_string().yellow(),
                    s.pages.to_string().yellow(),
                    "dist/".bold()
                ),
                Err(e) => { eprintln!("{} {}", "✘".red().bold(), e); process::exit(1); }
            }
        }
        "serve" => {
            let port = args.get(2).and_then(|s| s.parse::<u16>().ok()).unwrap_or(3000);
            println!("{} http://localhost:{}", "▸ folio —".cyan().bold(), port);
            // Build first
            if let Err(e) = generator::build() {
                eprintln!("{} {}", "✘".red().bold(), e); process::exit(1);
            }
            if let Err(e) = server::serve(port) {
                eprintln!("{} {}", "✘".red().bold(), e); process::exit(1);
            }
        }
        "new" => {
            let kind = args.get(2).map(|s| s.as_str()).unwrap_or("post");
            let title = args[3..].join(" ");
            if title.is_empty() {
                eprintln!("Usage: folio new [post|wiki|page] <title>"); process::exit(1);
            }
            match content::create_new(kind, &title) {
                Ok(p)  => println!("{} {}", "✔".green().bold(), p.display()),
                Err(e) => { eprintln!("{} {}", "✘".red().bold(), e); process::exit(1); }
            }
        }
        "init" => {
            match generator::init_project() {
                Ok(_)  => println!("{} project initialised", "✔".green().bold()),
                Err(e) => { eprintln!("{} {}", "✘".red().bold(), e); process::exit(1); }
            }
        }
        _ => println!("{}", HELP),
    }
}

const HELP: &str = "\
folio — flat-file personal site generator

  folio init                 scaffold a new project
  folio build                build → dist/
  folio serve [port]         build & serve locally (default 3000)
  folio new post  <title>    create content/posts/<slug>.md
  folio new wiki  <title>    create content/wiki/<slug>.md
  folio new page  <title>    create content/pages/<slug>.md
";
