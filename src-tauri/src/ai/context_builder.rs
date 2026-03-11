use chrono::Local;
use serde_json::Value;

pub fn build_current_datetime_context() -> String {
    let now = Local::now();
    let tz = now.offset().to_string();
    format!(
        "Current date and time: {}, {}, {}, timezone {}",
        now.format("%A"),
        now.format("%Y-%m-%d"),
        now.format("%I:%M %p"),
        tz
    )
}

pub fn build_runtime_context(
    memory_store: &crate::MemoryStore,
    user_id: &str,
) -> String {
    let datetime = build_current_datetime_context();
    let system = crate::system::system_context_service::get_system_context();

    let recent_commands = {
        let conn = match memory_store.conn.lock() {
            Ok(conn) => conn,
            Err(_) => {
                return format!("{}\nSystem context: unavailable\nRecent commands: unavailable", datetime);
            }
        };

        crate::command_history_service::get_recent_commands(&conn, user_id, 5)
            .unwrap_or_default()
    };

    let running = if system.running_applications.is_empty() {
        "none".to_string()
    } else {
        system.running_applications.join(", ")
    };

    let commands = if recent_commands.is_empty() {
        "none".to_string()
    } else {
        recent_commands.join(" | ")
    };

    let battery = system
        .battery_level
        .map(|v| format!("{}%", v))
        .unwrap_or_else(|| "unknown".to_string());
    let network = system
        .network_status
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    format!(
        "{}\nSystem context: Active app: {} | Running apps: {} | Battery: {} | Network: {} | Captured: {}\nRecent commands: {}",
        datetime,
        system.active_application,
        running,
        battery,
        network,
        system.timestamp.format("%H:%M:%S"),
        commands,
    )
}

pub async fn extract_semantic_keywords(query: &str) -> Vec<String> {
    let prompt = format!(
        "Extract 3 to 8 semantic keywords from this user query for memory retrieval. Return JSON array only.\\nQuery: {}",
        query.trim()
    );

    let llm_keywords = super::llm_client::generate_structured_response(prompt)
        .await
        .ok()
        .and_then(|raw| parse_keyword_array(&raw));

    llm_keywords.unwrap_or_else(|| fallback_keywords(query))
}

pub fn detect_correction_parameters(message: &str) -> Option<Value> {
    let lower = message.trim().to_lowercase();
    let starts_like_correction = lower.starts_with("no")
        || lower.starts_with("actually")
        || lower.starts_with("correction")
        || lower.starts_with("it's ")
        || lower.starts_with("it is ");

    if !starts_like_correction {
        return None;
    }

    if let Some(idx) = lower.find(" at ") {
        let new_time = message[idx + 4..].trim();
        if !new_time.is_empty() {
            return Some(serde_json::json!({
                "query": "class",
                "new_time": new_time,
                "raw_correction": message.trim()
            }));
        }
    }

    Some(serde_json::json!({
        "query": "class",
        "new_content": message.trim(),
        "raw_correction": message.trim()
    }))
}

fn parse_keyword_array(raw: &str) -> Option<Vec<String>> {
    let trimmed = raw.trim();
    let stripped = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let mut out = serde_json::from_str::<Vec<String>>(stripped).ok()?;
    out.retain(|s| !s.trim().is_empty());
    out.truncate(8);
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn fallback_keywords(query: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "a", "an", "and", "are", "about", "can", "do", "does", "for", "have", "hey", "i",
        "in", "is", "know", "me", "my", "of", "on", "or", "please", "that", "the", "to",
        "what", "when", "where", "who", "why", "you", "tell", "show",
    ];

    let mut seen = std::collections::HashSet::new();
    let mut keywords = query
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|t| t.len() >= 2)
        .filter(|t| !STOP.contains(t))
        .filter(|t| seen.insert((*t).to_string()))
        .map(|t| t.to_string())
        .collect::<Vec<_>>();

    if keywords.is_empty() {
        keywords.push("memory".to_string());
    }

    keywords.truncate(8);
    keywords
}
