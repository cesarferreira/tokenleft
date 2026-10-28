use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::model::{Provider, Row};

pub fn fetch() -> Result<Provider> {
    let mut child = Command::new("codex")
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("starting `codex app-server` (is codex installed?)")?;

    let mut stdin = child.stdin.take().context("no stdin")?;
    let stdout = child.stdout.take().context("no stdout")?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let init = json!({"jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{"clientInfo":{"name":"agent-usage","version":env!("CARGO_PKG_VERSION")}}});
    let initialized = json!({"jsonrpc":"2.0","method":"initialized"});
    let read = json!({"jsonrpc":"2.0","id":2,"method":"account/rateLimits/read"});
    for msg in [init, initialized, read] {
        writeln!(stdin, "{msg}")?;
    }
    stdin.flush()?;

    let result = loop {
        let line = match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(l) => l,
            Err(_) => {
                let _ = child.kill();
                bail!("timed out waiting for codex app-server");
            }
        };
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if v["id"] == 2 {
            break v;
        }
    };
    let _ = child.kill();
    let _ = child.wait();

    if let Some(err) = result["error"]["message"].as_str() {
        bail!("codex: {err}");
    }
    let limits = &result["result"]["rateLimits"];
    let mut rows = Vec::new();

    for (key, fallback) in [
        ("primary", "Primary window"),
        ("secondary", "Secondary window"),
    ] {
        let w = &limits[key];
        if let Some(pct) = w["usedPercent"].as_f64() {
            rows.push(Row {
                label: w["windowDurationMins"]
                    .as_i64()
                    .map(window_label)
                    .unwrap_or(fallback.into()),
                used_pct: Some(pct),
                detail: None,
                resets_at: w["resetsAt"].as_i64(),
            });
        }
    }

    let ind = &limits["individualLimit"];
    if let (Some(limit), Some(used)) = (num(&ind["limit"]), num(&ind["used"])) {
        let pct = ind["remainingPercent"]
            .as_f64()
            .map(|r| 100.0 - r)
            .or(Some(used / limit * 100.0));
        rows.push(Row {
            label: "Monthly credits".into(),
            used_pct: pct,
            detail: Some(format!("{} / {} credits", short(used), short(limit))),
            resets_at: ind["resetsAt"].as_i64(),
        });
    }

    let plan = limits["planType"].as_str().map(|s| match s {
        "business" => "ChatGPT Business".to_string(),
        other => crate::claude_title(other),
    });
    Ok(Provider {
        name: "Codex",
        plan,
        rows,
        error: None,
    })
}

fn num(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

fn short(n: f64) -> String {
    if n >= 1000.0 {
        format!("{:.1}k", n / 1000.0)
    } else {
        format!("{n:.0}")
    }
}

fn window_label(mins: i64) -> String {
    match mins {
        300 => "Session (5h)".into(),
        10080 => "Weekly".into(),
        m if m % 1440 == 0 => format!("{}-day window", m / 1440),
        m if m % 60 == 0 => format!("{}h window", m / 60),
        m => format!("{m}m window"),
    }
}
