use serde::{Deserialize, Serialize};
use std::time::Duration;

const GEMINI_API_ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-3-flash-preview:generateContent";

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct Part {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: String,
}

/// Sends a prompt to Gemini and returns the response text.
pub async fn generate_structured_response(prompt: String) -> Result<String, String> {
    let mut last_error = String::new();
    let retry_delays_ms = [250_u64, 500_u64, 1000_u64];

    for (idx, delay) in retry_delays_ms.iter().enumerate() {
        match generate_structured_response_once(prompt.clone()).await {
            Ok(value) => return Ok(value),
            Err(err) => {
                last_error = err;
                if idx + 1 < retry_delays_ms.len() {
                    tokio::time::sleep(Duration::from_millis(*delay)).await;
                }
            }
        }
    }

    if prompt.contains("AI system controller") {
        return Ok(fallback_rule_based_intent(&prompt));
    }

    if prompt.contains("action planning engine") {
        let single = fallback_rule_based_intent(&prompt);
        let intent_value: serde_json::Value = serde_json::from_str(&single)
            .unwrap_or_else(|_| serde_json::json!({ "intent": "unknown", "parameters": {} }));
        let plan = serde_json::json!({
            "actions": [
                {
                    "intent": intent_value.get("intent").and_then(serde_json::Value::as_str).unwrap_or("unknown"),
                    "parameters": intent_value.get("parameters").cloned().unwrap_or_else(|| serde_json::json!({})),
                    "requires_confirmation": false
                }
            ],
            "reasoning": "fallback"
        });
        return Ok(plan.to_string());
    }

    Err(last_error)
}

pub async fn request_action_plan(prompt: String) -> Result<String, String> {
    generate_structured_response(prompt).await
}

async fn generate_structured_response_once(prompt: String) -> Result<String, String> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| "GEMINI_API_KEY environment variable not set".to_string())?;

    let url = format!("{}?key={}", GEMINI_API_ENDPOINT, api_key);

    let request_body = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: prompt,
            }],
        }],
    };

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Gemini: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!(
            "Gemini API error (status {}): {}",
            status, error_text
        ));
    }

    let gemini_response: GeminiResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

    let response_text = gemini_response
        .candidates
        .first()
        .map(|candidate| {
            candidate
                .content
                .parts
                .iter()
                .map(|part| part.text.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .ok_or_else(|| "No response text from Gemini".to_string())?;

    Ok(response_text)
}

fn fallback_rule_based_intent(prompt: &str) -> String {
    let message = extract_user_request(prompt).to_lowercase();

    if message.contains("remind me") || message.contains("set reminder") {
        return "{\"intent\":\"set_reminder\",\"parameters\":{\"content\":\"reminder\",\"time_description\":\"soon\"},\"confidence\":0.5}".to_string();
    }
    if message.starts_with("remember") || message.contains("remember that") {
        return format!(
            "{{\"intent\":\"save_memory\",\"parameters\":{{\"content\":\"{}\"}},\"confidence\":0.55}}",
            message.replace('"', "'")
        );
    }
    if message.contains("forget") || message.contains("delete memory") {
        return "{\"intent\":\"forget_memory\",\"parameters\":{\"query\":\"memory\"},\"confidence\":0.5}".to_string();
    }
    if message.contains("open ") {
        return "{\"intent\":\"open_app\",\"parameters\":{\"target\":\"chrome\"},\"confidence\":0.45}".to_string();
    }
    if message.contains("class") || message.contains("schedule") || message.contains("timetable") {
        return format!(
            "{{\"intent\":\"search_memory\",\"parameters\":{{\"query\":\"{}\"}},\"confidence\":0.45}}",
            message.replace('"', "'")
        );
    }
    if message.contains("what") || message.contains("why") || message.contains("how") {
        return format!(
            "{{\"intent\":\"ai_query\",\"parameters\":{{\"query\":\"{}\"}},\"confidence\":0.4}}",
            message.replace('"', "'")
        );
    }

    "{\"intent\":\"unknown\",\"parameters\":{},\"confidence\":0.2}".to_string()
}

fn extract_user_request(prompt: &str) -> String {
    let marker = "User request:";
    if let Some(idx) = prompt.rfind(marker) {
        let tail = &prompt[idx + marker.len()..];
        return tail.lines().next().unwrap_or("unknown").trim().to_string();
    }
    "unknown".to_string()
}
