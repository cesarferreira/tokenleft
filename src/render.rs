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
    let bar = bar(pct, WIDTH, "━", "━");
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

fn bar(pct: f64, width: usize, on: &str, off: &str) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    format!(
        "{}{}\x1b[2m{}\x1b[0m",
        color(pct),
        on.repeat(filled),
        off.repeat(width - filled)
    )
}

/// The row that best summarises a provider: spend when there is one, else the last window.
fn headline(p: &Provider) -> Option<&Row> {
    let with_pct = || p.rows.iter().filter(|r| r.used_pct.is_some());
    with_pct()
        .find(|r| r.detail.is_some())
        .or(with_pct().last())
}

fn pct_of(r: &Row) -> f64 {
    r.used_pct.unwrap_or(0.0).clamp(0.0, 100.0)
}

pub fn compact(providers: &[Provider]) -> String {
    let mut out = String::new();
    for p in providers {
        out.push_str(&format!("{:<7}", p.name));
        match (&p.error, headline(p)) {
            (Some(_), None) => out.push_str("\x1b[31merror\x1b[0m"),
            (_, Some(r)) => {
                let pct = pct_of(r);
                out.push_str(&format!("{} {pct:>3.0}%", bar(pct, 10, "▰", "▱")));
            }
            (None, None) => out.push_str("\x1b[2mn/a\x1b[0m"),
        }
        out.push('\n');
    }
    out
}

/// Three aligned lines: bar, money or credits used, percent used.
pub fn summary(providers: &[Provider]) -> String {
    let rows: Vec<_> = providers.iter().map(|p| (p, headline(p))).collect();
    let detail_w = rows
        .iter()
        .filter_map(|(_, r)| r.and_then(|r| r.detail.as_ref()))
        .map(|d| d.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for (p, r) in rows {
        out.push_str(&format!("{:<7}", p.name));
        match r {
            Some(r) => {
                let pct = pct_of(r);
                let detail = r.detail.as_deref().unwrap_or("");
                out.push_str(&format!(
                    "{}  {detail:<detail_w$}  {pct:>3.0}%",
                    bar(pct, 20, "━", "━")
                ));
            }
            None => out.push_str("\x1b[31merror\x1b[0m"),
        }
        out.push('\n');
    }
    out
}

/// One line of circle glyphs that fill as usage grows.
pub fn rings(providers: &[Provider]) -> String {
    let cells: Vec<String> = providers
        .iter()
        .map(|p| match headline(p) {
            Some(r) => {
                let pct = pct_of(r);
                let glyph = match pct {
                    x if x < 12.5 => "○",
                    x if x < 37.5 => "◔",
                    x if x < 62.5 => "◑",
                    x if x < 87.5 => "◕",
                    _ => "●",
                };
                format!("{} {}{glyph}\x1b[0m {pct:.0}%", p.name, color(pct))
            }
            None => format!("{} \x1b[31m!\x1b[0m", p.name),
        })
        .collect();
    format!("{}\n", cells.join("   "))
}

/// Uncoloured single line for status bars and scripts.
pub fn plain(providers: &[Provider]) -> String {
    let cells: Vec<String> = providers
        .iter()
        .map(|p| match headline(p) {
            Some(r) => format!("{} {:.0}%", p.name, pct_of(r)),
            None => format!("{} err", p.name),
        })
        .collect();
    format!("{}\n", cells.join(" · "))
}

fn ago(secs: i64) -> String {
    match secs {
        s if s < 60 => "just now".into(),
        s if s < 3600 => format!("{}m ago", s / 60),
        s => format!("{}h ago", s / 3600),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str, pct: f64, detail: Option<&str>) -> Row {
        Row {
            label: label.into(),
            used_pct: Some(pct),
            detail: detail.map(Into::into),
            resets_at: None,
        }
    }

    fn provider(name: &str, rows: Vec<Row>, error: Option<&str>) -> Provider {
        Provider {
            name: name.into(),
            plan: None,
            rows,
            error: error.map(Into::into),
            fetched_at: 0,
        }
    }

    #[test]
    fn headline_prefers_spend_over_windows() {
        let p = provider(
            "Codex",
            vec![
                row("Weekly", 80.0, None),
                row("Monthly", 37.0, Some("$1 / $2")),
            ],
            None,
        );
        assert_eq!(headline(&p).unwrap().label, "Monthly");
    }

    #[test]
    fn headline_falls_back_to_last_window() {
        let p = provider(
            "Claude",
            vec![row("5h", 10.0, None), row("Weekly", 20.0, None)],
            None,
        );
        assert_eq!(headline(&p).unwrap().label, "Weekly");
    }

    #[test]
    fn plain_has_no_escape_codes_and_marks_errors() {
        let ps = [
            provider("Claude", vec![row("m", 22.4, Some("$1 / $2"))], None),
            provider("Cursor", vec![], Some("nope")),
        ];
        let out = plain(&ps);
        assert_eq!(out, "Claude 22% · Cursor err\n");
    }

    #[test]
    fn summary_has_one_line_per_provider() {
        let ps = [
            provider("Claude", vec![row("m", 22.0, Some("$330 / $1,500"))], None),
            provider("Cursor", vec![], Some("nope")),
        ];
        assert_eq!(summary(&ps).lines().count(), 2);
    }

    #[test]
    fn rings_glyph_tracks_usage() {
        let ps = [provider("A", vec![row("m", 100.0, None)], None)];
        assert!(rings(&ps).contains('●'));
    }
}
