use serde::{Deserialize, Serialize};

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
