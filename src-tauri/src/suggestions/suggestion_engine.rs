use chrono::Local;
use uuid::Uuid;

use super::suggestion_types::{Suggestion, SuggestionContext};

pub fn evaluate_suggestions(context: &SuggestionContext) -> Vec<Suggestion> {
    if context.is_idle {
        return Vec::new();
    }

    let mut out = Vec::new();

    // Rule 1: upcoming reminder in <= 10 minutes
    for (content, trigger_at) in &context.upcoming_reminders {
        let delta = *trigger_at - context.now_ts;
        if (0..=600).contains(&delta) {
            out.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                user_id: context.user_id.clone(),
                message: format!("Reminder soon: {}. Open your notes app?", content),
                action_intent: Some("open_app".to_string()),
                parameters: Some(serde_json::json!({ "target": "notepad" })),
                priority: 9,
                timestamp: Local::now(),
            });
            break;
        }
    }

    // Rule 2: frequent app usage pattern proxy from command history
    let opened_vscode_recently = context
        .recent_commands
        .iter()
        .any(|c| c.to_lowercase().contains("open") && c.to_lowercase().contains("vscode"));
    if opened_vscode_recently && !contains_app(&context.running_applications, "code") {
        out.push(Suggestion {
            id: Uuid::new_v4().to_string(),
            user_id: context.user_id.clone(),
            message: "You often start coding around now. Open VSCode?".to_string(),
            action_intent: Some("open_app".to_string()),
            parameters: Some(serde_json::json!({ "target": "code" })),
            priority: 7,
            timestamp: Local::now(),
        });
    }

    // Rule 3: Spotify active (paused state unavailable, use active/running signal)
    if contains_app(&context.running_applications, "spotify") {
        out.push(Suggestion {
            id: Uuid::new_v4().to_string(),
            user_id: context.user_id.clone(),
            message: "Spotify is active. Resume your focus music?".to_string(),
            action_intent: Some("open_app".to_string()),
            parameters: Some(serde_json::json!({ "target": "spotify" })),
            priority: 4,
            timestamp: Local::now(),
        });
    }

    // Rule 4: upcoming class in <= 30 minutes
    for (subject, class_ts) in &context.upcoming_classes {
        let delta = *class_ts - context.now_ts;
        if (0..=1800).contains(&delta) {
            out.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                user_id: context.user_id.clone(),
                message: format!("{} starts soon. Want to review your notes?", subject),
                action_intent: Some("search_memory".to_string()),
                parameters: Some(serde_json::json!({ "keyword": subject })),
                priority: 8,
                timestamp: Local::now(),
            });
            break;
        }
    }

    // Basic system-state hint (low priority)
    if let Some(battery) = context.battery_level {
        if battery <= 20 {
            out.push(Suggestion {
                id: Uuid::new_v4().to_string(),
                user_id: context.user_id.clone(),
                message: "Battery is getting low. Consider plugging in soon.".to_string(),
                action_intent: None,
                parameters: None,
                priority: 2,
                timestamp: Local::now(),
            });
        }
    }

    out.sort_by(|a, b| b.priority.cmp(&a.priority));
    out.truncate(3);
    out
}

fn contains_app(apps: &[String], needle: &str) -> bool {
    apps.iter().any(|a| a.to_lowercase().contains(needle))
}
