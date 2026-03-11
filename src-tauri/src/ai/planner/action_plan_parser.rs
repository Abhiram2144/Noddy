use serde::Deserialize;
use serde_json::Value;

use super::action_plan::{ActionPlan, ActionStep};

const MAX_ACTIONS: usize = 5;

#[derive(Debug, Deserialize)]
struct LegacySingleIntent {
    intent: String,
    #[serde(default)]
    parameters: Value,
}

pub fn parse_and_validate_action_plan(raw: &str) -> Result<ActionPlan, String> {
    let json_text = extract_json_payload(raw);

    if let Ok(mut plan) = serde_json::from_str::<ActionPlan>(&json_text) {
        validate_plan(&mut plan)?;
        return Ok(plan);
    }

    // Backward compatibility: if planning parse fails, accept legacy single-intent object.
    if let Ok(single) = serde_json::from_str::<LegacySingleIntent>(&json_text) {
        let mut plan = ActionPlan {
            actions: vec![ActionStep {
                intent: single.intent,
                parameters: single.parameters,
                requires_confirmation: false,
            }],
            reasoning: Some("Fallback from single-intent payload".to_string()),
        };
        validate_plan(&mut plan)?;
        return Ok(plan);
    }

    Err(format!("Invalid action plan payload: {}", raw))
}

fn validate_plan(plan: &mut ActionPlan) -> Result<(), String> {
    if plan.actions.is_empty() {
        return Err("Action plan has no actions".to_string());
    }

    if plan.actions.len() > MAX_ACTIONS {
        return Err(format!("Action plan exceeds max actions ({})", MAX_ACTIONS));
    }

    for step in &mut plan.actions {
        step.intent = step.intent.trim().to_string();
        if step.intent.is_empty() {
            return Err("Action step contains empty intent".to_string());
        }

        if !is_supported_intent(&step.intent) {
            return Err(format!("Unknown intent in action plan: {}", step.intent));
        }

        if !step.parameters.is_object() {
            return Err(format!("Parameters must be an object for intent {}", step.intent));
        }

        validate_parameters(&step.intent, &step.parameters)?;
    }

    Ok(())
}

fn validate_parameters(intent: &str, params: &Value) -> Result<(), String> {
    let has = |k: &str| params.get(k).and_then(Value::as_str).map(|v| !v.trim().is_empty()).unwrap_or(false);

    match intent {
        "set_reminder" => {
            if !has("content") && !has("task") && !has("message") {
                return Err("set_reminder requires content/task/message".to_string());
            }
        }
        "save_memory" => {
            if !has("content") && !has("memory") && !has("text") {
                return Err("save_memory requires content/memory/text".to_string());
            }
        }
        "search_memory" | "delete_memory" | "forget_memory" => {
            if !has("query") && !has("keyword") && !has("target") {
                return Err(format!("{} requires query/keyword/target", intent));
            }
        }
        "update_memory" => {
            if !has("query") && !has("keyword") && !has("target") {
                return Err("update_memory requires query/keyword/target".to_string());
            }
            if !has("new_content") && !has("new_time") && !has("content") {
                return Err("update_memory requires new_content/new_time/content".to_string());
            }
        }
        "open_app" => {
            if !has("target") && !has("app") && !has("app_name") {
                return Err("open_app requires target/app/app_name".to_string());
            }
        }
        "search_web" => {
            if !has("query") && !has("url") {
                return Err("search_web requires query/url".to_string());
            }
        }
        "plugin_action" => {
            if !has("plugin_id") || !has("command") {
                return Err("plugin_action requires plugin_id and command".to_string());
            }
        }
        "ai_query" => {
            if !has("query") {
                return Err("ai_query requires query".to_string());
            }
        }
        "unknown" => {}
        _ => {}
    }

    Ok(())
}

fn is_supported_intent(intent: &str) -> bool {
    matches!(
        intent,
        "set_reminder"
            | "save_memory"
            | "update_memory"
            | "delete_memory"
            | "forget_memory"
            | "search_memory"
            | "open_app"
            | "search_web"
            | "plugin_action"
            | "ai_query"
            | "unknown"
    )
}

fn extract_json_payload(raw: &str) -> String {
    let trimmed = raw.trim();
    let without_fence = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if (without_fence.starts_with('{') && without_fence.ends_with('}'))
        || (without_fence.starts_with('[') && without_fence.ends_with(']'))
    {
        return without_fence.to_string();
    }

    let start_obj = without_fence.find('{');
    let end_obj = without_fence.rfind('}').map(|idx| idx + 1);
    match (start_obj, end_obj) {
        (Some(start), Some(end)) if end > start => without_fence[start..end].trim().to_string(),
        _ => without_fence.to_string(),
    }
}
