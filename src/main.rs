mod claude;
mod codex;
mod cursor;
mod model;
mod render;

use anyhow::Result;
use clap::Parser;
use model::Provider;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "agent-usage",
    version,
    about = "Show remaining Claude, Codex and Cursor usage"
)]
struct Cli {
    /// Print machine-readable JSON instead of bars
    #[arg(long)]
    json: bool,

    /// Refresh every N seconds (default 120 when given without a value)
    #[arg(short, long, num_args = 0..=1, default_missing_value = "120", value_name = "SECS")]
    watch: Option<u64>,

    /// Only show these providers (claude, codex, cursor)
    #[arg(short, long, value_delimiter = ',')]
    only: Vec<String>,
}

pub fn claude_title(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

fn collect(only: &[String]) -> Vec<Provider> {
    let want = |n: &str| only.is_empty() || only.iter().any(|o| o.eq_ignore_ascii_case(n));
    type Fetcher = fn() -> Result<Provider>;
    let all: [(&'static str, Fetcher); 3] = [
        ("claude", claude::fetch),
        ("codex", codex::fetch),
        ("cursor", cursor::fetch),
    ];
    let handles: Vec<_> = all
        .into_iter()
        .filter(|(n, _)| want(n))
        .map(|(n, f)| (n, std::thread::spawn(f)))
        .collect();
    handles
        .into_iter()
        .map(|(n, h)| match h.join() {
            Ok(Ok(p)) => p,
            Ok(Err(e)) => Provider::failed(label(n), e),
            Err(_) => Provider::failed(label(n), anyhow::anyhow!("fetcher panicked")),
        })
        .collect()
}

fn label(n: &str) -> &'static str {
    match n {
        "claude" => "Claude",
        "codex" => "Codex",
        _ => "Cursor",
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    loop {
        let providers = collect(&cli.only);
        let now = chrono::Utc::now().timestamp();
        if cli.json {
            println!("{}", serde_json::to_string_pretty(&providers)?);
        } else {
            if cli.watch.is_some() {
                print!("\x1b[2J\x1b[H");
            }
            print!("{}", render::render(&providers, now));
        }
        match cli.watch {
            Some(secs) => std::thread::sleep(Duration::from_secs(secs.max(10))),
            None => return Ok(()),
        }
    }
}
