use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub label: String,
    pub used_pct: Option<f64>,
    pub detail: Option<String>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub name: String,
    pub plan: Option<String>,
    pub rows: Vec<Row>,
    pub error: Option<String>,
    /// Unix time the data was fetched; 0 when unknown.
    #[serde(default)]
    pub fetched_at: i64,
}

impl Provider {
    pub fn failed(name: &str, err: anyhow::Error) -> Self {
        Self {
            name: name.into(),
            plan: None,
            rows: Vec::new(),
            error: Some(format!("{err:#}")),
            fetched_at: 0,
        }
    }
}

pub fn money(minor: f64, exponent: u32) -> String {
    let value = minor / 10f64.powi(exponent as i32);
    format!("${}", group(value))
}

fn group(value: f64) -> String {
    let whole = format!("{:.2}", value);
    let (int, frac) = whole.split_once('.').unwrap_or((&whole, "00"));
    let mut out = String::new();
    for (i, c) in int.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    let int: String = out.chars().rev().collect();
    if frac == "00" {
        int
    } else {
        format!("{int}.{frac}")
    }
}
