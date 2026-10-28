mod cache;
mod claude;
mod codex;
mod cursor;
mod model;
mod render;

use anyhow::Result;
use cache::Cache;
use clap::{Parser, ValueEnum};
use model::Provider;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "tokenleft",
    version,
    about = "Show remaining Claude, Codex and Cursor usage"
)]
struct Cli {
    /// Output format
    #[arg(short, long, value_enum, default_value_t = Format::Table)]
    format: Format,

    /// Shorthand for --format json
    #[arg(long, conflicts_with = "format")]
    json: bool,

    /// Refresh every N seconds (default 120 when given without a value)
    #[arg(short, long, num_args = 0..=1, default_missing_value = "120", value_name = "SECS")]
    watch: Option<u64>,

    /// Seconds a successful fetch is reused (0 disables the cache)
    #[arg(long, default_value_t = 300, value_name = "SECS")]
    ttl: i64,

    /// Ignore the cache and fetch fresh data
    #[arg(short, long)]
    refresh: bool,

    /// Only show these providers (claude, codex, cursor)
    #[arg(short, long, value_delimiter = ',')]
    only: Vec<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq)]
enum Format {
    /// Detailed bars with spend and reset times
    Table,
    /// One compact line per provider
    Bars,
    /// Three lines: bar, money or credits used, percent used
    Summary,
    /// One line of circle glyphs, like a sidebar ring
    Rings,
    /// One uncoloured line, for status bars and scripts
    Plain,
    /// Machine-readable JSON
    Json,
}

pub fn claude_title(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

fn collect(cli: &Cli, now: i64) -> Vec<Provider> {
    let want = |n: &str| cli.only.is_empty() || cli.only.iter().any(|o| o.eq_ignore_ascii_case(n));
    type Fetcher = fn() -> Result<Provider>;
    let all: [(&'static str, Fetcher); 3] = [
        ("claude", claude::fetch),
        ("codex", codex::fetch),
        ("cursor", cursor::fetch),
    ];
    let mut cache = Cache::load();
    let (ttl, refresh) = (cli.ttl, cli.refresh);
    let handles: Vec<_> = all
        .into_iter()
        .filter(|(n, _)| want(n))
        .map(|(n, f)| {
            let cached = cache.get(n).cloned();
            (
                n,
                std::thread::spawn(move || resolve(n, f, cached, now, ttl, refresh)),
            )
        })
        .collect();
    let mut out = Vec::new();
    for (n, h) in handles {
        let (provider, fresh) = h.join().unwrap_or_else(|_| {
            (
                Provider::failed(label(n), anyhow::anyhow!("fetcher panicked")),
                false,
            )
        });
        if fresh {
            cache.put(n, provider.clone());
        }
        out.push(provider);
    }
    if ttl > 0 {
        cache.save();
    }
    out
}

fn resolve(
    key: &str,
    fetch: fn() -> Result<Provider>,
    cached: Option<Provider>,
    now: i64,
    ttl: i64,
    refresh: bool,
) -> (Provider, bool) {
    if !refresh
        && let Some(c) = &cached
        && now - c.fetched_at < ttl
    {
        return (c.clone(), false);
    }
    match fetch() {
        Ok(mut p) => {
            p.fetched_at = now;
            (p, true)
        }
        Err(e) => match cached {
            Some(mut c) => {
                c.error = Some(format!("{e:#} (showing cached data)"));
                (c, false)
            }
            None => (Provider::failed(label(key), e), false),
        },
    }
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
        let now = chrono::Utc::now().timestamp();
        let providers = collect(&cli, now);
        let format = if cli.json { Format::Json } else { cli.format };
        if format == Format::Json {
            println!("{}", serde_json::to_string_pretty(&providers)?);
        } else {
            if cli.watch.is_some() {
                print!("\x1b[2J\x1b[H");
            }
            match format {
                Format::Bars => print!("{}", render::compact(&providers)),
                Format::Summary => print!("{}", render::summary(&providers)),
                Format::Rings => print!("{}", render::rings(&providers)),
                Format::Plain => print!("{}", render::plain(&providers)),
                _ => print!("{}", render::render(&providers, now)),
            }
        }
        match cli.watch {
            Some(secs) => std::thread::sleep(Duration::from_secs(secs.max(10))),
            None => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(fetched_at: i64) -> Provider {
        Provider {
            name: "Claude".into(),
            plan: None,
            rows: Vec::new(),
            error: None,
            fetched_at,
        }
    }

    fn ok() -> Result<Provider> {
        Ok(provider(0))
    }

    fn boom() -> Result<Provider> {
        anyhow::bail!("boom")
    }

    #[test]
    fn fresh_cache_skips_fetch() {
        let (p, fresh) = resolve("claude", boom, Some(provider(900)), 1000, 300, false);
        assert!(p.error.is_none());
        assert!(!fresh);
    }

    #[test]
    fn expired_cache_refetches_and_stamps_time() {
        let (p, fresh) = resolve("claude", ok, Some(provider(100)), 1000, 300, false);
        assert_eq!(p.fetched_at, 1000);
        assert!(fresh);
    }

    #[test]
    fn refresh_bypasses_fresh_cache() {
        let (p, fresh) = resolve("claude", ok, Some(provider(999)), 1000, 300, true);
        assert_eq!(p.fetched_at, 1000);
        assert!(fresh);
    }

    #[test]
    fn zero_ttl_never_uses_cache() {
        let (p, _) = resolve("claude", ok, Some(provider(1000)), 1000, 0, false);
        assert_eq!(p.fetched_at, 1000);
    }

    #[test]
    fn failed_fetch_falls_back_to_stale_data() {
        let (p, fresh) = resolve("claude", boom, Some(provider(100)), 1000, 300, false);
        assert_eq!(p.fetched_at, 100);
        assert!(p.error.unwrap().contains("showing cached data"));
        assert!(!fresh);
    }

    #[test]
    fn failed_fetch_without_cache_reports_error() {
        let (p, fresh) = resolve("claude", boom, None, 1000, 300, false);
        assert!(p.error.unwrap().contains("boom"));
        assert!(!fresh);
    }
}
