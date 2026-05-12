use std::path::PathBuf;

use anyhow::{Result, anyhow};

const APP_DIR_NAME: &str = "zxcv";

fn home_dir() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))
}

/// `~/.config/zxcv/config.toml`
pub fn config_file() -> Result<PathBuf> {
    let base = if let Ok(config) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(config)
    } else {
        home_dir()?.join(".config")
    };
    Ok(base.join(APP_DIR_NAME).join("config.toml"))
}

/// `~/.local/state/zxcv/history.toml`
pub fn history_file() -> Result<PathBuf> {
    let base = if let Ok(state) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(state)
    } else {
        home_dir()?.join(".local").join("state")
    };
    Ok(base.join(APP_DIR_NAME).join("history.toml"))
}

/// `~/.local/state/zxcv/hint-shown` — sentinel file written after the one-time setup hint
/// has been displayed.
pub fn hint_flag_file() -> Result<PathBuf> {
    let base = if let Ok(state) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(state)
    } else {
        home_dir()?.join(".local").join("state")
    };
    Ok(base.join(APP_DIR_NAME).join("hint-shown"))
}

/// Default install prefix for `zxcv install-man`. Pages are placed under
/// `<prefix>/man/man1/`. Honors `XDG_DATA_HOME`, falling back to `~/.local/share`.
pub fn default_data_prefix() -> Result<PathBuf> {
    let base = if let Ok(data) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(data)
    } else {
        home_dir()?.join(".local").join("share")
    };
    Ok(base)
}

/// `~/.cache/zxcv/llm_cache/`
pub fn llm_cache_dir() -> Result<PathBuf> {
    let base = if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(cache)
    } else {
        home_dir()?.join(".cache")
    };
    Ok(base.join(APP_DIR_NAME).join("llm_cache"))
}
