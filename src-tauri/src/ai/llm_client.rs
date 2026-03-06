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

/// Sends a message to Google Gemini API and returns the response text.
/// 
/// # Arguments
/// * `message` - The user's message to send to Gemini
/// 
/// # Returns
/// * `Ok(String)` - The assistant's response text
/// * `Err(String)` - Error message if the request fails
pub async fn ask_gemini(message: String) -> Result<String, String> {
    // Read API key from environment variable
    let api_key = std::env::var("GEMINI_API_KEY")
        .map_err(|_| "GEMINI_API_KEY environment variable not set".to_string())?;

    // Build the request URL with API key
    let url = format!("{}?key={}", GEMINI_API_ENDPOINT, api_key);

    // Build request body
    let request_body = GeminiRequest {
        contents: vec![Content {
            parts: vec![Part {
                text: message,
            }],
        }],
    };

    // Make the HTTP request
    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Gemini: {}", e))?;

    // Check if the request was successful
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

    // Parse the response
    let gemini_response: GeminiResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

    // Extract the text from the first candidate
    let response_text = gemini_response
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .ok_or_else(|| "No response text from Gemini".to_string())?;

    Ok(response_text)
}
