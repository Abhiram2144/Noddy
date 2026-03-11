use super::{orchestrator::StructuredIntent, tool_executor};
use serde_json::json;

pub async fn route_intent(
    user_message: &str,
    structured_intent: StructuredIntent,
    user_id: &str,
    app_handle: &tauri::AppHandle,
    registry: &crate::AppRegistry,
    memory_store: &crate::MemoryStore,
    plugin_registry: &crate::plugin_registry::PluginRegistry,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    if structured_intent.confidence <= 0.0 {
        return Ok("I'm not sure what you meant. Could you rephrase that?".to_string());
    }

    if structured_intent.confidence < 0.35 {
        return Ok(format!(
            "I'm not entirely sure what you'd like me to do. Could you be more specific?",
        ));
    }

    match structured_intent.intent.as_str() {
        "set_reminder" => tool_executor::execute_set_reminder(
            &structured_intent.parameters,
            user_message,
            user_id,
            app_handle,
            memory_store,
            event_bus,
            permissions,
        ),
        "save_memory" => {
            tool_executor::execute_save_memory(
                &structured_intent.parameters,
                user_id,
                memory_store,
                event_bus,
                permissions,
            )
            .await
        }
        "update_memory" => tool_executor::execute_update_memory(
            &structured_intent.parameters,
            user_id,
            memory_store,
            event_bus,
            permissions,
        ),
        "delete_memory" | "forget_memory" => tool_executor::execute_delete_memory(
            &structured_intent.parameters,
            user_id,
            memory_store,
            event_bus,
            permissions,
        ),
        "search_memory" => tool_executor::execute_search_memory(
            &structured_intent.parameters,
            user_id,
            memory_store,
            event_bus,
            permissions,
        ),
        "query_timetable" => {
            // Backward compatibility: legacy timetable intent is now handled via memory search.
            let query = extract_subject_query(user_message)
                .map(|subject| format!("{} class", subject))
                .unwrap_or_else(|| "class schedule".to_string());
            let params = json!({ "query": query });
            tool_executor::execute_search_memory(
                &params,
                user_id,
                memory_store,
                event_bus,
                permissions,
            )
        }
        "open_app" => tool_executor::execute_open_app(
            &structured_intent.parameters,
            registry,
            event_bus,
            permissions,
        ),
        "search_web" => tool_executor::execute_search_web(
            &structured_intent.parameters,
            app_handle,
            event_bus,
            permissions,
        ),
        "plugin_action" => tool_executor::execute_plugin_action(
            &structured_intent.parameters,
            memory_store,
            plugin_registry,
            event_bus,
        ),
        "ai_query" => {
            tool_executor::execute_ai_query(
                &structured_intent.parameters,
                user_message,
                user_id,
                memory_store,
            )
            .await
        }
        "unknown" => Ok("No actionable system command detected.".to_string()),
        other => Err(format!("Unsupported intent returned by LLM: {}", other)),
    }
}

fn extract_subject_query(message: &str) -> Option<String> {
    let lower = message.to_lowercase();

    // Keep full timetable requests on the dedicated timetable intent.
    let full_schedule_phrases = [
        "full timetable",
        "weekly timetable",
        "my timetable",
        "show me my timetable",
        "show me my schedule",
        "this week",
        "whole schedule",
    ];
    if full_schedule_phrases.iter().any(|p| lower.contains(p)) {
        return None;
    }

    if !(lower.contains("class") || lower.contains("lecture") || lower.contains("subject")) {
        return None;
    }

    let class_idx = lower.find("class").or_else(|| lower.find("lecture")).or_else(|| lower.find("subject"))?;
    let prefix = &message[..class_idx];

    let stop_words = [
        "when", "will", "i", "usually", "have", "my", "the", "a", "an", "do", "did", "is", "are",
        "what", "time", "which", "day", "on", "at", "for", "tell", "me", "can", "you", "please",
    ];

    let cleaned_words = prefix
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .filter(|w| !stop_words.contains(&w.as_str()))
        .collect::<Vec<_>>();

    if cleaned_words.is_empty() {
        None
    } else {
        Some(cleaned_words.join(" "))
    }
}
