use chrono::Local;
use serde::Deserialize;
use serde_json::Value;

use super::{intent_router, llm_client, prompt_templates};

#[derive(Debug, Clone, Deserialize)]
pub struct StructuredIntent {
    pub intent: String,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub confidence: f64,
}

pub async fn process_user_command(
    message: String,
    user_id: &str,
    app_handle: &tauri::AppHandle,
    registry: &crate::AppRegistry,
    memory_store: &crate::MemoryStore,
    plugin_registry: &crate::plugin_registry::PluginRegistry,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    let history_text = {
        let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
        let items = crate::chat_history_store::get_messages(&conn, user_id, 3).unwrap_or_default();
        items.iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let prompt = prompt_templates::build_intent_prompt(&message, &history_text);
    let raw_response = llm_client::generate_structured_response(prompt).await?;
    let mut structured_intent = parse_structured_intent(&raw_response)?;
    disambiguate_intent(&message, &mut structured_intent);

    if structured_intent.intent == "set_reminder" {
        normalize_reminder_parameters_with_llm(&message, &mut structured_intent).await;
    }

    intent_router::route_intent(
        &message,
        structured_intent,
        user_id,
        app_handle,
        registry,
        memory_store,
        plugin_registry,
        event_bus,
        permissions,
    )
    .await
}

async fn normalize_reminder_parameters_with_llm(
    message: &str,
    structured_intent: &mut StructuredIntent,
) {
    let now = Local::now();
    let prompt = prompt_templates::build_reminder_normalization_prompt(
        message,
        now.timestamp(),
        &now.to_rfc3339(),
    );

    let raw = match llm_client::generate_structured_response(prompt).await {
        Ok(value) => value,
        Err(_) => return,
    };

    let parsed = match parse_json_value(&raw) {
        Ok(value) => value,
        Err(_) => return,
    };

    let mut merged = structured_intent.parameters.clone();

    if let Some(content) = parsed
        .get("content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        merged["content"] = Value::String(content.to_string());
    }

    if let Some(trigger_at) = parsed
        .get("trigger_at")
        .and_then(Value::as_i64)
        .filter(|v| *v > 0)
    {
        merged["trigger_at"] = Value::Number(trigger_at.into());
    }

    if let Some(time_description) = parsed
        .get("time_description")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        merged["time_description"] = Value::String(time_description.to_string());
    }

    structured_intent.parameters = merged;

    if let Some(confidence) = parsed
        .get("confidence")
        .and_then(Value::as_f64)
        .filter(|v| *v > 0.0)
    {
        structured_intent.confidence = structured_intent.confidence.max(confidence);
    }
}

fn parse_structured_intent(raw_response: &str) -> Result<StructuredIntent, String> {
    let json_text = extract_json_object(raw_response);
    serde_json::from_str::<StructuredIntent>(&json_text).map_err(|e| {
        format!(
            "Failed to parse structured Gemini response: {}. Raw: {}",
            e, raw_response
        )
    })
}

fn parse_json_value(raw_response: &str) -> Result<Value, String> {
    let json_text = extract_json_object(raw_response);
    serde_json::from_str::<Value>(&json_text)
        .map_err(|e| format!("Failed to parse Gemini JSON response: {}. Raw: {}", e, raw_response))
}

fn extract_json_object(raw_response: &str) -> String {
    let trimmed = raw_response.trim();

    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return trimmed.to_string();
    }

    let without_fence = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if without_fence.starts_with('{') && without_fence.ends_with('}') {
        return without_fence.to_string();
    }

    let start = trimmed.find('{').unwrap_or(0);
    let end = trimmed
        .rfind('}')
        .map(|index| index + 1)
        .unwrap_or(trimmed.len());
    trimmed[start..end].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::extract_json_object;

    #[test]
    fn extracts_json_from_code_fence() {
        let value = extract_json_object("```json\n{\"intent\":\"open_app\"}\n```");
        assert_eq!(value, "{\"intent\":\"open_app\"}");
    }
}

fn disambiguate_intent(message: &str, structured_intent: &mut StructuredIntent) {
    let normalized = message.trim().to_lowercase();
    let looks_like_memory = normalized.starts_with("remember ") || normalized.starts_with("remember that ");
    let explicit_reminder = normalized.contains("remind me")
        || normalized.contains("set reminder")
        || normalized.contains("set a reminder")
        || normalized.contains("alert me");

    if looks_like_memory && !explicit_reminder {
        structured_intent.intent = "save_memory".to_string();

        let has_content = structured_intent
            .parameters
            .get("content")
            .and_then(Value::as_str)
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);

        if !has_content {
            let extracted = normalized
                .strip_prefix("remember that ")
                .or_else(|| normalized.strip_prefix("remember "))
                .unwrap_or(normalized.as_str())
                .trim()
                .to_string();

            if !extracted.is_empty() {
                structured_intent.parameters = serde_json::json!({ "content": extracted });
            }
        }

        if structured_intent.confidence <= 0.0 {
            structured_intent.confidence = 0.8;
        }
    }
}