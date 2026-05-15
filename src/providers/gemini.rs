use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};

use crate::candidate::Candidate;
use crate::providers::{
    GenerationContext, MAX_CANDIDATES, Settings, build_system_prompt, parse_candidates_json,
    require_api_key,
};

const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

pub async fn generate(
    settings: &Settings,
    query: &str,
    context: &GenerationContext,
) -> Result<Vec<Candidate>> {
    let api_key = require_api_key(settings)?;
    let system_prompt = build_system_prompt(context);
    let base = format!("{API_BASE}/{model}:generateContent", model = settings.model);
    let mut url = reqwest::Url::parse(&base).context("failed to parse Gemini URL")?;
    url.query_pairs_mut().append_pair("key", api_key);

    let body = json!({
        "systemInstruction": {"parts": [{"text": system_prompt}]},
        "contents": [{"role": "user", "parts": [{"text": query}]}],
        "generationConfig": {
            "responseMimeType": "application/json",
            "responseSchema": candidate_schema(),
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("failed to send request to Gemini API")?;

    let status = response.status();
    let payload: Value = response
        .json()
        .await
        .context("failed to parse Gemini response as JSON")?;

    if !status.is_success() {
        return Err(super::api_error(
            "Gemini",
            status.as_u16(),
            &payload.to_string(),
            settings.provider.api_key_env(),
        ));
    }

    let content = payload
        .pointer("/candidates/0/content/parts/0/text")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            anyhow!("Gemini response missing candidates[0].content.parts[0].text: {payload}")
        })?;

    parse_candidates_json(content)
}

fn candidate_schema() -> Value {
    // Gemini uses an OpenAPI-style subset; "maxItems" is supported.
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
