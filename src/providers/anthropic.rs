use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::candidate::Candidate;
use crate::providers::{MAX_CANDIDATES, SYSTEM_PROMPT, Settings, require_api_key};

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";

#[derive(Debug, Deserialize)]
struct SuggestCommandsInput {
    candidates: Vec<Candidate>,
}

pub async fn generate(settings: &Settings, query: &str) -> Result<Vec<Candidate>> {
    let api_key = require_api_key(settings)?;

    let body = json!({
        "model": settings.model,
        "max_tokens": 1024,
        "system": format!("{SYSTEM_PROMPT}\n- Always call the `suggest_commands` tool. Do not respond in plain text."),
        "tool_choice": {"type": "tool", "name": "suggest_commands"},
        "tools": [{
            "name": "suggest_commands",
            "description": "Return up to 5 one-liner shell command candidates matching the user's request.",
            "input_schema": candidate_schema(),
        }],
        "messages": [
            {"role": "user", "content": query}
        ]
    });

    let client = reqwest::Client::new();
    let response = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", API_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("failed to send request to Anthropic API")?;

    let status = response.status();
    let payload: Value = response
        .json()
        .await
        .context("failed to parse Anthropic API response as JSON")?;

    if !status.is_success() {
        bail!("Anthropic API returned {}: {}", status, payload);
    }

    extract_candidates(&payload)
}

fn candidate_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "candidates": {
                "type": "array",
                "maxItems": MAX_CANDIDATES,
                "items": {
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "A single-line shell command."},
                        "description": {"type": "string", "description": "Short English explanation."}
                    },
                    "required": ["command", "description"]
                }
            }
        },
        "required": ["candidates"]
    })
}

fn extract_candidates(payload: &Value) -> Result<Vec<Candidate>> {
    let content = payload
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("response has no `content` array: {}", payload))?;

    let tool_use = content
        .iter()
        .find(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
        .ok_or_else(|| anyhow!("response contains no tool_use block: {}", payload))?;

    let input = tool_use
        .get("input")
        .ok_or_else(|| anyhow!("tool_use block has no `input` field: {}", tool_use))?;

    let parsed: SuggestCommandsInput =
        serde_json::from_value(input.clone()).context("failed to parse tool_use input")?;

    if parsed.candidates.is_empty() {
        bail!("LLM returned no candidates");
    }

    Ok(parsed.candidates.into_iter().take(MAX_CANDIDATES).collect())
}
