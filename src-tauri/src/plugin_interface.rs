use serde_json::Value;

#[derive(Debug, Clone)]
pub enum PluginEvent {
    ReminderScheduled { content: String },
    ReminderFired { content: String },
    TaskCompleted { task_id: String, task_type: String },
}

pub trait Plugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn provider(&self) -> &'static str;
    fn capabilities(&self) -> &'static [&'static str];
    fn initialize(&self, config_json: Option<&str>) -> Result<(), String>;
    fn handle_event(&self, event: &PluginEvent, config_json: Option<&str>) -> Result<(), String>;
    fn execute_command(&self, command: &str, config_json: Option<&str>) -> Result<Value, String>;
}