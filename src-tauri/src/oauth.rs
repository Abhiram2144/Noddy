use std::thread;
use tiny_http::{Response, Server};
use url::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64, // Unix timestamp
    pub provider: String,
}

pub struct OAuthManager {
    pub redirect_port: u16,
}

impl OAuthManager {
    pub fn new() -> Self {
        Self { redirect_port: 1421 } // Choose a fixed port for redirect
    }

    pub fn get_redirect_uri(&self) -> String {
        format!("http://localhost:{}", self.redirect_port)
    }

    pub fn start_callback_listener(&self, app_handle: tauri::AppHandle, plugin_id: String) {
        let port = self.redirect_port;
        let plugin_id_clone = plugin_id.clone();
        
        thread::spawn(move || {
            let server = Server::http(format!("0.0.0.0:{}", port)).unwrap();
            
            if let Some(request) = server.incoming_requests().next() {
                let url = format!("http://localhost:{}{}", port, request.url());
                let parsed_url = Url::parse(&url).unwrap();
                let code = parsed_url.query_pairs()
                    .find(|(key, _)| key == "code")
                    .map(|(_, value)| value.into_owned());

                if let Some(code) = code {
                    // Send success response to browser
                    let response = Response::from_string("<h1>Authentication Successful!</h1><p>You can close this window and return to Noddy.</p>")
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
                    let _ = request.respond(response);

                    // Emit event back to main thread or handle token exchange here
                    // For now, we'll emit a tauri event
                    let _ = tauri::Emitter::emit(&app_handle, "oauth_code_received", serde_json::json!({
                        "plugin_id": plugin_id_clone,
                        "code": code
                    }));
                } else {
                    let response = Response::from_string("<h1>Authentication Failed</h1><p>No authorization code found.</p>")
                        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
                    let _ = request.respond(response);
                }
            }
        });
    }
}

pub async fn exchange_code_for_tokens(
    provider: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<OAuthTokens, String> {
    let client = reqwest::Client::new();
    
    let (token_url, client_id, client_secret) = match provider {
        "google" => (
            "https://oauth2.googleapis.com/token",
            std::env::var("GOOGLE_CLIENT_ID").map_err(|_| "Missing GOOGLE_CLIENT_ID")?,
            std::env::var("GOOGLE_CLIENT_SECRET").map_err(|_| "Missing GOOGLE_CLIENT_SECRET")?,
        ),
        "outlook" => (
            "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            std::env::var("OUTLOOK_CLIENT_ID").map_err(|_| "Missing OUTLOOK_CLIENT_ID")?,
            std::env::var("OUTLOOK_CLIENT_SECRET").map_err(|_| "Missing OUTLOOK_CLIENT_SECRET")?,
        ),
        _ => return Err("Unsupported provider".to_string()),
    };

    let params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ];

    let response = client.post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Token request failed: {}", e))?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        return Err(format!("Token exchange error: {}", err_text));
    }

    #[derive(Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: Option<String>,
        expires_in: u64,
    }

    let tokens: TokenResponse = response.json().await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    let expires_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + tokens.expires_in;

    Ok(OAuthTokens {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at,
        provider: provider.to_string(),
    })
}
