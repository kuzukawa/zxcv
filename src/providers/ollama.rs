use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};

use crate::candidate::Candidate;
use crate::providers::{MAX_CANDIDATES, SYSTEM_PROMPT, Settings, parse_candidates_json};

const DEFAULT_ENDPOINT: &str = "http://localhost:11434";

pub async fn generate(settings: &Settings, query: &str) -> Result<Vec<Candidate>> {
    let endpoint = settings
        .endpoint
        .as_deref()
        .unwrap_or(DEFAULT_ENDPOINT)
        .trim_end_matches('/');
    let url = format!("{endpoint}/api/chat");

    let body = json!({
        "model": settings.model,
        "stream": false,
        "format": candidate_schema(),
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": query}
        ]
    });

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .with_context(|| format!("failed to send request to Ollama at {url}"))?;

    let status = response.status();
    let payload: Value = response
        .json()
        .await
        .context("failed to parse Ollama response as JSON")?;

    if !status.is_success() {
        return Err(super::api_error(
            "Ollama",
            status.as_u16(),
            &payload.to_string(),
            None,
        ));
    }

    let content = payload
        .pointer("/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Ollama response missing message.content: {payload}"))?;

    parse_candidates_json(content)
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
                        "command": {"type": "string"},
                        "description": {"type": "string"}
                    },
                    "required": ["command", "description"]
                }
            }
        },
        "required": ["candidates"]
    })
}
