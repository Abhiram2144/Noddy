use serde_json::Value;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum PluginEvent {
    ReminderScheduled { content: String },
    ReminderFired { content: String },
    TaskCompleted { task_id: String, task_type: String },
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn provider(&self) -> &'static str;
    fn capabilities(&self) -> &'static [&'static str];
    async fn initialize(&self, config_json: Option<&str>) -> Result<(), String>;
    async fn handle_event(&self, event: &PluginEvent, config_json: Option<&str>) -> Result<(), String>;
    async fn execute_command(&self, command: &str, config_json: Option<&str>) -> Result<Value, String>;
    async fn authenticate(&self, app_handle: &tauri::AppHandle) -> Result<String, String>;
}