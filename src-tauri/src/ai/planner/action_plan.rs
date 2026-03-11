use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub actions: Vec<ActionStep>,
    #[serde(default)]
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    pub intent: String,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub requires_confirmation: bool,
}
