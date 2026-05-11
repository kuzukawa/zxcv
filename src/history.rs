use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::candidate::Candidate;
use crate::paths;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct History {
    #[serde(default)]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Entry {
    pub query: String,
    pub command: String,
    pub description: String,
    pub count: u32,
    pub last_at: u64,
}

impl Entry {
    pub fn to_candidate(&self) -> Candidate {
        Candidate {
            command: self.command.clone(),
            description: self.description.clone(),
        }
    }
}

pub fn load() -> Result<History> {
    let path = paths::history_file()?;
    if !path.exists() {
        return Ok(History::default());
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read history file {}", path.display()))?;
    let history: History = toml::from_str(&text)
        .with_context(|| format!("failed to parse history file {}", path.display()))?;
    Ok(history)
}

pub fn save(history: &History) -> Result<()> {
    let path = paths::history_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create history directory {}", parent.display()))?;
    }
    let text = toml::to_string(history).context("failed to serialize history")?;
    fs::write(&path, text)
        .with_context(|| format!("failed to write history file {}", path.display()))?;
    Ok(())
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// zoxide-style frecency score.
pub fn frecency(count: u32, last_at: u64, now: u64) -> f64 {
    let age = now.saturating_sub(last_at);
    let factor = if age < 3_600 {
        4.0
    } else if age < 86_400 {
        2.0
    } else if age < 86_400 * 7 {
        0.5
    } else {
        0.25
    };
    f64::from(count) * factor
}

/// Return entries sorted by frecency descending, then last_at descending.
pub fn sorted_by_frecency(history: &History) -> Vec<&Entry> {
    let now = now();
    let mut entries: Vec<&Entry> = history.entries.iter().collect();
    entries.sort_by(|a, b| {
        let sa = frecency(a.count, a.last_at, now);
        let sb = frecency(b.count, b.last_at, now);
        sb.partial_cmp(&sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.last_at.cmp(&a.last_at))
    });
    entries
}

/// Record a selection: bump count + last_at on existing entry, or insert new.
pub fn record(history: &mut History, query: &str, candidate: &Candidate) {
    let now = now();
    if let Some(entry) = history
        .entries
        .iter_mut()
        .find(|e| e.query == query && e.command == candidate.command)
    {
        entry.count += 1;
        entry.last_at = now;
        entry.description = candidate.description.clone();
    } else {
        history.entries.push(Entry {
            query: query.to_string(),
            command: candidate.command.clone(),
            description: candidate.description.clone(),
            count: 1,
            last_at: now,
        });
    }
}
