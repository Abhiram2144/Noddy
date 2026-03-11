use chrono::{DateTime, Local};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub id: String,
    pub user_id: String,
    pub message: String,
    pub action_intent: Option<String>,
    pub parameters: Option<Value>,
    pub priority: u8,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Clone)]
pub struct SuggestionContext {
    pub user_id: String,
    pub active_application: String,
    pub running_applications: Vec<String>,
    pub battery_level: Option<u8>,
    pub network_status: Option<String>,
    pub is_idle: bool,
    pub upcoming_reminders: Vec<(String, i64)>,
    pub recent_commands: Vec<String>,
    pub upcoming_classes: Vec<(String, i64)>,
    pub now_ts: i64,
}
