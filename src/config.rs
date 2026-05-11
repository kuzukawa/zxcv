use std::collections::HashMap;
use std::fs;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::Cli;
use crate::paths;
use crate::providers::{Provider, Settings};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    pub provider: Option<String>,
    #[serde(default)]
    pub providers: HashMap<String, ProviderEntry>,
    #[serde(default)]
    pub safety: SafetyConfig,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ProviderEntry {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SafetyConfig {
    #[serde(default)]
    pub extra_patterns: Vec<String>,
}

pub fn load() -> Result<Config> {
    let path = paths::config_file()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    let cfg: Config = toml::from_str(&text)
        .with_context(|| format!("failed to parse config file {}", path.display()))?;
    Ok(cfg)
}

/// Resolve effective settings from CLI args, env vars, and config file.
/// Precedence: CLI > env > config > built-in default.
pub fn resolve(cli: &Cli, config: &Config) -> Result<Settings> {
    let provider_str = cli
        .provider
        .clone()
        .or_else(|| std::env::var("ZXCV_PROVIDER").ok())
        .or_else(|| config.provider.clone())
        .unwrap_or_else(|| "anthropic".into());
    let provider = Provider::parse(&provider_str)?;

    let entry = config
        .providers
        .get(provider.id())
        .cloned()
        .unwrap_or_default();

    let model = cli
        .model
        .clone()
        .or_else(|| std::env::var("ZXCV_MODEL").ok())
        .or(entry.model)
        .unwrap_or_else(|| provider.default_model().into());

    let api_key = provider
        .api_key_env()
        .and_then(|var| std::env::var(var).ok())
        .or(entry.api_key);

    let endpoint = entry.endpoint;

    Ok(Settings {
        provider,
        api_key,
        model,
        endpoint,
    })
}
