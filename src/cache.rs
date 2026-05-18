use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::candidate::Candidate;
use crate::paths;

#[derive(Debug, Deserialize, Serialize)]
struct CacheFile {
    candidates: Vec<Candidate>,
}

fn cache_path(provider: &str, model: &str, query: &str, context_key: &str) -> Result<PathBuf> {
    let mut h = DefaultHasher::new();
    provider.hash(&mut h);
    model.hash(&mut h);
    query.hash(&mut h);
    context_key.hash(&mut h);
    let key = format!("{:016x}.json", h.finish());
    Ok(paths::llm_cache_dir()?.join(key))
}

pub fn load(
    provider: &str,
    model: &str,
    query: &str,
    context_key: &str,
) -> Result<Option<Vec<Candidate>>> {
    let path = cache_path(provider, model, query, context_key)?;
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read cache file {}", path.display()))?;
    let file: CacheFile = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse cache file {}", path.display()))?;
    Ok(Some(file.candidates))
}

pub fn save(
    provider: &str,
    model: &str,
    query: &str,
    context_key: &str,
    candidates: &[Candidate],
) -> Result<()> {
    let path = cache_path(provider, model, query, context_key)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create cache directory {}", parent.display()))?;
    }
    let file = CacheFile {
        candidates: candidates.to_vec(),
    };
    let text = serde_json::to_string_pretty(&file).context("failed to serialize cache contents")?;
    fs::write(&path, text)
        .with_context(|| format!("failed to write cache file {}", path.display()))?;
    Ok(())
}
