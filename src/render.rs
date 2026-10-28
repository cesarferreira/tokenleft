use crate::model::{Provider, Row};

const WIDTH: usize = 24;

fn color(pct: f64) -> &'static str {
    if pct >= 90.0 {
        "\x1b[31m"
    } else if pct >= 70.0 {
        "\x1b[33m"
    } else {
        "\x1b[34m"
    }
}

pub fn render(providers: &[Provider], now: i64) -> String {
    let mut out = String::from("\x1b[1mAgent usage\x1b[0m\n");
    for p in providers {
        out.push('\n');
        out.push_str(&format!("\x1b[1m{}\x1b[0m", p.name));
        if let Some(plan) = &p.plan {
            out.push_str(&format!("  \x1b[2m{plan}\x1b[0m"));
        }
        if p.fetched_at > 0 {
            out.push_str(&format!(
                "  \x1b[2m· updated {}\x1b[0m",
                ago(now - p.fetched_at)
            ));
        }
        out.push('\n');
        if let Some(err) = &p.error {
            out.push_str(&format!("  \x1b[31m{err}\x1b[0m\n"));
        }
        for r in &p.rows {
            out.push_str(&row(r, now));
        }
    }
    out
}

fn row(r: &Row, now: i64) -> String {
    let pct = r.used_pct.unwrap_or(0.0).clamp(0.0, 100.0);
    let filled = ((pct / 100.0) * WIDTH as f64).round() as usize;
    let bar = format!(
        "{}{}\x1b[2m{}\x1b[0m",
        color(pct),
        "━".repeat(filled),
        "━".repeat(WIDTH - filled)
    );
    let mut tail = format!("{pct:>3.0}% used");
    if let Some(d) = &r.detail {
        tail = format!("{d}  ({tail})");
    }
    let mut s = format!("  {:<16} {bar}\x1b[0m  {tail}", r.label);
    if let Some(at) = r.resets_at {
        s.push_str(&format!("  \x1b[2mresets {}\x1b[0m", until(at - now)));
    }
    s.push('\n');
    s
}

fn until(secs: i64) -> String {
    if secs <= 0 {
        return "now".into();
    }
    let (d, h, m) = (secs / 86400, secs % 86400 / 3600, secs % 3600 / 60);
    match (d, h) {
        (0, 0) => format!("in {m}m"),
        (0, _) => format!("in {h}h {m}m"),
        _ => format!("in {d}d {h}h"),
    }
}

pub fn compact(providers: &[Provider]) -> String {
    let mut out = String::new();
    for p in providers {
        out.push_str(&format!("{:<7}", p.name));
        match (&p.error, p.rows.iter().rev().find_map(|r| r.used_pct)) {
            (Some(_), _) => out.push_str("\x1b[31merror\x1b[0m"),
            (None, Some(pct)) => {
                let pct = pct.clamp(0.0, 100.0);
                let filled = ((pct / 100.0) * 10.0).round() as usize;
                out.push_str(&format!(
                    "{}{}\x1b[2m{}\x1b[0m {pct:>3.0}%",
                    color(pct),
                    "▰".repeat(filled),
                    "▱".repeat(10 - filled)
                ));
            }
            (None, None) => out.push_str("\x1b[2mn/a\x1b[0m"),
        }
        out.push('\n');
    }
    out
}

fn ago(secs: i64) -> String {
    match secs {
        s if s < 60 => "just now".into(),
        s if s < 3600 => format!("{}m ago", s / 60),
        s => format!("{}h ago", s / 3600),
    }
}
