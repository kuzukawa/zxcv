pub mod anthropic;
pub mod gemini;
pub mod ollama;
pub mod openai;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::candidate::Candidate;

pub const MAX_CANDIDATES: usize = 5;

pub const BASE_SYSTEM_PROMPT: &str = "\
You generate shell one-liner commands for macOS and Linux.
Given a natural-language request (which may be in Japanese), return up to 5 candidate one-liner commands that fulfill it.

Rules:
- Each candidate must be a single line that can be pasted directly into a POSIX shell. No multi-line scripts, no leading $ or # prompt characters.
- Prefer portable POSIX / GNU / BSD-compatible commands. Tools that ship on a typical macOS or Linux system are preferred over uncommon ones.
- The `description` field must be written in English and concisely explain what the command does.
- Order candidates from most to least likely to be what the user wants.";

#[derive(Debug, Clone, Copy)]
pub enum CommandMode {
    InstalledOnly,
    AllowUninstalled,
}

#[derive(Debug, Clone)]
pub struct GenerationContext {
    pub mode: CommandMode,
    pub installed_commands: Vec<String>,
    pub omitted_installed_count: usize,
}

pub fn build_system_prompt(ctx: &GenerationContext) -> String {
    let mut prompt = String::from(BASE_SYSTEM_PROMPT);
    prompt.push_str("\n\nDetected installed commands on this machine:\n");
    if ctx.installed_commands.is_empty() {
        prompt.push_str("- (none detected)\n");
    } else {
        prompt.push_str("- ");
        prompt.push_str(&ctx.installed_commands.join(", "));
        prompt.push('\n');
        if ctx.omitted_installed_count > 0 {
            prompt.push_str(&format!(
                "- (plus {} additional installed commands omitted from this list)\n",
                ctx.omitted_installed_count
            ));
        }
    }

    match ctx.mode {
        CommandMode::InstalledOnly => {
            prompt.push_str(
                "\
\nMode: installed-only.
- For every candidate, all non-shell-builtin commands must be from the installed command list above.
- Do not use tools that are not on that installed list.
- If perfect coverage is not possible, still return best-effort installed-only candidates.
",
            );
        }
        CommandMode::AllowUninstalled => {
            prompt.push_str(
                "\
\nMode: fallback-allow-uninstalled.
- Prefer installed commands when possible.
- If installed commands cannot satisfy the request well, you may include uninstalled tools.
- When a candidate uses uninstalled tools, include `Requires install: <tool1>, <tool2>.` in the description.
",
            );
        }
    }

    prompt
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Anthropic,
    OpenAI,
    Ollama,
    Gemini,
}

impl Provider {
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "anthropic" => Ok(Self::Anthropic),
            "openai" => Ok(Self::OpenAI),
            "ollama" => Ok(Self::Ollama),
            "gemini" => Ok(Self::Gemini),
            other => Err(anyhow!(
                "unknown provider {other:?}; expected one of: anthropic, openai, ollama, gemini"
            )),
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::OpenAI => "openai",
            Self::Ollama => "ollama",
            Self::Gemini => "gemini",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Self::Anthropic => "claude-sonnet-4-6",
            Self::OpenAI => "gpt-5",
            Self::Ollama => "llama3",
            Self::Gemini => "gemini-2.5-flash",
        }
    }

    /// Environment variable name that holds the API key for this provider, if any.
    pub fn api_key_env(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            Self::OpenAI => Some("OPENAI_API_KEY"),
            Self::Gemini => Some("GEMINI_API_KEY"),
            Self::Ollama => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub provider: Provider,
    pub api_key: Option<String>,
    pub model: String,
    pub endpoint: Option<String>,
}

pub async fn generate(
    settings: &Settings,
    query: &str,
    context: &GenerationContext,
) -> Result<Vec<Candidate>> {
    match settings.provider {
        Provider::Anthropic => anthropic::generate(settings, query, context).await,
        Provider::OpenAI => openai::generate(settings, query, context).await,
        Provider::Ollama => ollama::generate(settings, query, context).await,
        Provider::Gemini => gemini::generate(settings, query, context).await,
    }
}

pub fn require_api_key(settings: &Settings) -> Result<&str> {
    settings.api_key.as_deref().ok_or_else(|| {
        let env = settings
            .provider
            .api_key_env()
            .unwrap_or("<no env var defined>");
        anyhow!(
            "API key for provider {} is not set (env: {})",
            settings.provider.id(),
            env
        )
    })
}

/// Parse a JSON payload produced by any provider into a Vec<Candidate>.
/// Accepts `{ "candidates": [{"command": "...", "description": "..."}] }`.
pub fn parse_candidates_json(raw: &str) -> Result<Vec<Candidate>> {
    #[derive(Deserialize)]
    struct Outer {
        candidates: Vec<Candidate>,
    }
    let outer: Outer = serde_json::from_str(raw)
        .map_err(|e| anyhow!("failed to parse JSON candidates payload: {e}: raw={raw}"))?;
    if outer.candidates.is_empty() {
        return Err(anyhow!("LLM returned no candidates"));
    }
    Ok(outer.candidates.into_iter().take(MAX_CANDIDATES).collect())
}
