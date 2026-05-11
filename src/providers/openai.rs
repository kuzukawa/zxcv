use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};

use crate::candidate::Candidate;
use crate::providers::{
    MAX_CANDIDATES, SYSTEM_PROMPT, Settings, parse_candidates_json, require_api_key,
};

const API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub async fn generate(settings: &Settings, query: &str) -> Result<Vec<Candidate>> {
    let api_key = require_api_key(settings)?;

    let body = json!({
        "model": settings.model,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": query}
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "suggest_commands",
                "strict": true,
                "schema": candidate_schema(),
            }
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(API_URL)
        .bearer_auth(api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("failed to send request to OpenAI API")?;

    let status = response.status();
    let payload: Value = response
        .json()
        .await
        .context("failed to parse OpenAI API response as JSON")?;

    if !status.is_success() {
        bail!("OpenAI API returned {}: {}", status, payload);
    }

    let content = payload
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("OpenAI response missing choices[0].message.content: {payload}"))?;

    parse_candidates_json(content)
}

fn candidate_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "candidates": {
                "type": "array",
                "maxItems": MAX_CANDIDATES,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
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
