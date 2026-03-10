use super::orchestrator;

/// Handles a user message through the AI orchestrator.
pub async fn handle_chat(
    message: String,
    user_id: &str,
    app_handle: &tauri::AppHandle,
    registry: &crate::AppRegistry,
    memory_store: &crate::MemoryStore,
    plugin_registry: &crate::plugin_registry::PluginRegistry,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("Message cannot be empty".to_string());
    }

    orchestrator::process_user_command(
        message,
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
