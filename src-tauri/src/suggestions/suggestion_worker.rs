use std::sync::{Mutex, OnceLock};

use chrono::TimeZone;
use rusqlite::Connection;
use tauri::{AppHandle, Emitter};

use crate::{command_history_service, reminder_store, Event, EventBus};

use super::suggestion_engine;
use super::suggestion_types::{Suggestion, SuggestionContext};

#[derive(Default)]
struct SuggestionLoopState {
    last_run_ts: i64,
}

fn loop_state() -> &'static Mutex<SuggestionLoopState> {
    static STATE: OnceLock<Mutex<SuggestionLoopState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(SuggestionLoopState::default()))
}

pub fn maybe_run_suggestion_cycle(
    conn: &Connection,
    event_bus: &EventBus,
    app_handle: Option<&AppHandle>,
) -> Result<(), String> {
    if suggestions_disabled() {
        return Ok(());
    }

    let now = current_timestamp();
    let interval = suggestion_interval_seconds();

    {
        let mut state = loop_state().lock().map_err(|e| format!("Suggestion state lock error: {}", e))?;
        if now - state.last_run_ts < interval {
            return Ok(());
        }
        state.last_run_ts = now;
    }

    let user_ids = collect_candidate_user_ids(conn)?;

    for user_id in user_ids {
        let context = build_context(conn, &user_id)?;
        let suggestions = suggestion_engine::evaluate_suggestions(&context);

        for suggestion in suggestions {
            publish_suggestion(event_bus, app_handle, &suggestion);
        }
    }

    Ok(())
}

fn build_context(conn: &Connection, user_id: &str) -> Result<SuggestionContext, String> {
    let sys = crate::system::system_context_service::get_system_context();
    let now = current_timestamp();

    let reminders = reminder_store::get_pending_reminders(conn, user_id)?
        .into_iter()
        .map(|r| (r.content, r.trigger_at))
        .collect::<Vec<_>>();

    let recent_commands = command_history_service::get_recent_commands(conn, user_id, 8)?;
    let class_entries = collect_upcoming_classes(conn, user_id)?;

    Ok(SuggestionContext {
        user_id: user_id.to_string(),
        active_application: sys.active_application,
        running_applications: sys.running_applications,
        battery_level: sys.battery_level,
        network_status: sys.network_status,
        is_idle: sys.idle_seconds.unwrap_or(0) >= 300,
        upcoming_reminders: reminders,
        recent_commands,
        upcoming_classes: class_entries,
        now_ts: now,
    })
}

fn collect_candidate_user_ids(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT user_id FROM reminders WHERE user_id IS NOT NULL
             UNION SELECT DISTINCT user_id FROM command_history WHERE user_id IS NOT NULL
             UNION SELECT DISTINCT user_id FROM memories WHERE user_id IS NOT NULL",
        )
        .map_err(|e| format!("Failed to prepare user id query: {}", e))?;

    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Failed to query user ids: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map user ids: {}", e))?;

    Ok(ids)
}

fn collect_upcoming_classes(conn: &Connection, user_id: &str) -> Result<Vec<(String, i64)>, String> {
    let now = current_timestamp();
    let horizon = now + 1800;

    let weekday = chrono::Local::now().format("%A").to_string().to_lowercase();

    let mut stmt = conn
        .prepare(
            "SELECT content FROM memories WHERE user_id = ?1 AND content LIKE '%class of %' AND content LIKE ?2 ORDER BY created_at DESC LIMIT 50",
        )
        .map_err(|e| format!("Failed to prepare class memory query: {}", e))?;

    let pattern = format!("%on {}%", weekday);
    let memories = stmt
        .query_map([user_id, pattern.as_str()], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Failed to query class memories: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to map class memories: {}", e))?;

    let mut out = Vec::new();
    for memory in memories {
        if let Some((subject, class_ts)) = parse_class_memory_time(&memory) {
            if class_ts >= now && class_ts <= horizon {
                out.push((subject, class_ts));
            }
        }
    }

    Ok(out)
}

fn parse_class_memory_time(content: &str) -> Option<(String, i64)> {
    let lower = content.to_lowercase();
    let marker = "class of ";
    let subject_start = lower.find(marker)? + marker.len();
    let remainder = &content[subject_start..];
    let remainder_lower = &lower[subject_start..];

    let at_idx = remainder_lower.find(" at ")?;
    let subject = remainder[..at_idx].trim().to_string();
    let tail = &remainder[at_idx + 4..];

    let on_idx = tail.to_lowercase().find(" on ")?;
    let time_text = tail[..on_idx].trim().to_string();

    let class_ts = time_to_today_timestamp(&time_text)?;
    Some((subject, class_ts))
}

fn time_to_today_timestamp(value: &str) -> Option<i64> {
    let cleaned = value.to_lowercase().replace(' ', "");
    let suffix = if cleaned.ends_with("am") {
        "am"
    } else if cleaned.ends_with("pm") {
        "pm"
    } else {
        return None;
    };

    let base = cleaned.trim_end_matches("am").trim_end_matches("pm");
    let (mut hour, minute) = if let Some((h, m)) = base.split_once(':') {
        (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?)
    } else {
        (base.parse::<u32>().ok()?, 0)
    };

    if hour == 0 || hour > 12 || minute > 59 {
        return None;
    }

    if suffix == "pm" && hour != 12 {
        hour += 12;
    }
    if suffix == "am" && hour == 12 {
        hour = 0;
    }

    let now = chrono::Local::now();
    let date = now.date_naive();
    let naive = chrono::NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(hour, minute, 0)?);
    let local = chrono::Local.from_local_datetime(&naive).single()?;
    Some(local.timestamp())
}

fn publish_suggestion(event_bus: &EventBus, app_handle: Option<&AppHandle>, suggestion: &Suggestion) {
    event_bus.emit(&Event::SuggestionGenerated(suggestion.clone()));

    if let Some(app) = app_handle {
        let _ = app.emit(
            "suggestion_generated",
            serde_json::json!({
                "id": suggestion.id,
                "user_id": suggestion.user_id,
                "message": suggestion.message,
                "action_intent": suggestion.action_intent,
                "parameters": suggestion.parameters,
                "priority": suggestion.priority,
                "timestamp": suggestion.timestamp.timestamp(),
            }),
        );
    }
}

fn suggestion_interval_seconds() -> i64 {
    std::env::var("NODDY_SUGGESTION_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|v| *v >= 15)
        .unwrap_or(60)
}

fn suggestions_disabled() -> bool {
    std::env::var("NODDY_SUGGESTIONS_DISABLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
