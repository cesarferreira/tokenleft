use anyhow::{Context, Result, bail};
use chrono::DateTime;
use serde_json::Value;
use std::process::Command;

use crate::model::{Provider, Row, money};

const WINDOWS: [(&str, &str); 4] = [
    ("five_hour", "Session (5h)"),
    ("seven_day", "Weekly"),
    ("seven_day_opus", "Weekly Opus"),
    ("seven_day_sonnet", "Weekly Sonnet"),
];

pub fn fetch() -> Result<Provider> {
    let (token, plan) = read_token()?;
    let usage: Value = match ureq::get("https://api.anthropic.com/api/oauth/usage")
        .set("Authorization", &format!("Bearer {token}"))
        .set("anthropic-beta", "oauth-2025-04-20")
        .call()
    {
        Ok(r) => r.into_json()?,
        Err(ureq::Error::Status(401 | 403, _)) => {
            bail!("token rejected (run `claude` once to refresh it)")
        }
        Err(ureq::Error::Status(429, _)) => bail!("rate limited by Anthropic, try again shortly"),
        Err(e) => return Err(e).context("usage request failed"),
    };

    let mut rows = Vec::new();
    for (key, label) in WINDOWS {
        let w = &usage[key];
        if let Some(pct) = w["utilization"].as_f64() {
            rows.push(Row {
                label: label.into(),
                used_pct: Some(pct),
                detail: None,
                resets_at: w["resets_at"]
                    .as_str()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.timestamp()),
            });
        }
    }

    let spend = &usage["spend"];
    if let (Some(used), Some(limit)) = (
        spend["used"]["amount_minor"].as_f64(),
        spend["limit"]["amount_minor"].as_f64(),
    ) {
        let exp = spend["used"]["exponent"].as_u64().unwrap_or(2) as u32;
        rows.push(Row {
            label: "Monthly spend".into(),
            used_pct: spend["percent"].as_f64().or(Some(used / limit * 100.0)),
            detail: Some(format!("{} / {}", money(used, exp), money(limit, exp))),
            resets_at: None,
        });
    }

    Ok(Provider {
        name: "Claude".into(),
        plan,
        rows,
        error: None,
        fetched_at: 0,
    })
}

fn read_token() -> Result<(String, Option<String>)> {
    let out = Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        .context("running /usr/bin/security")?;
    if !out.status.success() {
        bail!("no Claude Code credentials in Keychain (log in with `claude`)");
    }
    let creds: Value = serde_json::from_slice(&out.stdout).context("parsing Keychain item")?;
    let oauth = &creds["claudeAiOauth"];
    let token = oauth["accessToken"].as_str().context("no accessToken")?;
    let plan = oauth["subscriptionType"].as_str().map(title_case);
    Ok((token.to_string(), plan))
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}
