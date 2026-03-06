use super::llm_client;

/// Handles a chat message from the user.
/// This is the main entry point for chat functionality.
/// 
/// # Arguments
/// * `message` - The user's message
/// 
/// # Returns
/// * `Ok(String)` - The AI assistant's response
/// * `Err(String)` - Error message if something goes wrong
pub async fn handle_chat(message: String) -> Result<String, String> {
    // Validate input
    if message.trim().is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    // Call the LLM client
    let response = llm_client::ask_gemini(message).await?;

    Ok(response)
}
