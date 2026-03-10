use super::{orchestrator::StructuredIntent, tool_executor};

pub fn route_intent(
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
        return Err("LLM did not provide a usable confidence score".to_string());
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
        "save_memory" => tool_executor::execute_save_memory(
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
        "unknown" => Ok("No actionable system command detected.".to_string()),
        other => Err(format!("Unsupported intent returned by LLM: {}", other)),
    }
}