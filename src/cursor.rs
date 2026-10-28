use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::DateTime;
use serde_json::Value;
use std::process::Command;

use crate::model::{Provider, Row, money};

pub fn fetch() -> Result<Provider> {
    let jwt = read_jwt()?;
    let user = user_id(&jwt)?;
    let usage: Value = ureq::get("https://cursor.com/api/usage-summary")
        .set(
            "Cookie",
            &format!("WorkosCursorSessionToken={user}%3A%3A{jwt}"),
        )
        .call()
        .context("usage request failed (open Cursor.app to refresh its session)")?
        .into_json()?;

    let resets_at = usage["billingCycleEnd"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp());

    let mut rows = Vec::new();
    let overall = &usage["individualUsage"]["overall"];
    if let (Some(used), Some(limit)) = (overall["used"].as_f64(), overall["limit"].as_f64())
        && limit > 0.0
    {
        rows.push(Row {
            label: "Your spend".into(),
            used_pct: Some(used / limit * 100.0),
            detail: Some(format!("{} / {}", money(used, 2), money(limit, 2))),
            resets_at,
        });
    }

    Ok(Provider {
        name: "Cursor".into(),
        plan: usage["membershipType"].as_str().map(crate::claude_title),
        rows,
        error: None,
        fetched_at: 0,
    })
}

fn read_jwt() -> Result<String> {
    let home = std::env::var("HOME").context("HOME not set")?;
    let db = format!("{home}/Library/Application Support/Cursor/User/globalStorage/state.vscdb");
    let out = Command::new("/usr/bin/sqlite3")
        .args([
            &db,
            "select value from ItemTable where key='cursorAuth/accessToken'",
        ])
        .output()
        .context("running /usr/bin/sqlite3")?;
    let jwt = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || jwt.is_empty() {
        bail!("no Cursor session found (sign in to Cursor.app)");
    }
    Ok(jwt)
}

fn user_id(jwt: &str) -> Result<String> {
    let payload = jwt.split('.').nth(1).context("malformed JWT")?;
    let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?;
    let sub = claims["sub"].as_str().context("JWT has no sub")?;
    Ok(sub.rsplit('|').next().unwrap_or(sub).to_string())
}
