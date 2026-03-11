use serde_json::Value;
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Weekday};
use tauri::Emitter;

pub fn execute_set_reminder(
    parameters: &Value,
    user_message: &str,
    user_id: &str,
    app_handle: &tauri::AppHandle,
    memory_store: &crate::MemoryStore,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    permissions.check_permission(crate::Capability::ReminderSchedule)?;

    let content = first_nonempty_string_param(
        parameters,
        &["content", "task", "message", "text", "title", "reminder", "note"],
    )
    .or_else(|| {
        parameters
            .get("reminder")
            .and_then(|value| {
                first_nonempty_string_param(value, &["content", "task", "message", "text", "title"])
            })
    })
    .map(str::to_string)
    .unwrap_or_else(|| infer_reminder_content_from_message(user_message));

    let content = if content.trim().is_empty() {
        "Reminder".to_string()
    } else {
        content
    };

    let trigger_at = timestamp_param(parameters)
        .or_else(|| {
            parameters
                .get("time_description")
                .and_then(Value::as_str)
                .and_then(parse_time_description)
        })
        .or_else(|| {
            parameters
                .get("time")
                .and_then(Value::as_str)
                .and_then(parse_time_description)
        })
        .ok_or_else(|| "Reminder intent requires a supported trigger_at or time description".to_string())?;

    let payload = serde_json::json!({
        "content": content,
        "trigger_at": trigger_at,
    })
    .to_string();

    crate::set_reminder(memory_store, user_id, &payload)?;
    event_bus.emit(&crate::Event::ReminderScheduled(content.clone()));
    let _ = app_handle.emit(
        "reminder_scheduled",
        serde_json::json!({
            "user_id": user_id,
            "content": content,
            "trigger_at": trigger_at
        }),
    );

    Ok(format!("Reminder scheduled for {}.", trigger_at))
}

pub fn execute_save_memory(
    parameters: &Value,
    user_id: &str,
    memory_store: &crate::MemoryStore,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    permissions.check_permission(crate::Capability::MemoryWrite)?;

    let content = string_param(parameters, &["content", "memory", "text"])?;
    crate::save_memory(memory_store, user_id, content)?;
    event_bus.emit(&crate::Event::MemorySaved(content.to_string()));

    Ok("Memory saved.".to_string())
}

pub fn execute_search_memory(
    parameters: &Value,
    user_id: &str,
    memory_store: &crate::MemoryStore,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    permissions.check_permission(crate::Capability::MemoryRead)?;

    let query = string_param(parameters, &["query", "keyword", "text"])?;
    let results = crate::search_memories(memory_store, user_id, query)?;
    event_bus.emit(&crate::Event::IntentExecuted {
        intent_name: "search_memory".to_string(),
        duration_ms: 0,
    });

    if results.is_empty() {
        Ok(format!("No memories found for '{}'.", query))
    } else {
        Ok(format!("Found memories: {}", results.join(" | ")))
    }
}

pub fn execute_open_app(
    parameters: &Value,
    registry: &crate::AppRegistry,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    permissions.check_permission(crate::Capability::OpenApp)?;

    let target = string_param(parameters, &["target", "app", "app_name"])?;
    crate::open_app_internal(target, registry)?;
    event_bus.emit(&crate::Event::IntentExecuted {
        intent_name: "open_app".to_string(),
        duration_ms: 0,
    });

    Ok(format!("Opened app: {}", target))
}

pub fn execute_search_web(
    parameters: &Value,
    app_handle: &tauri::AppHandle,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    permissions.check_permission(crate::Capability::WebSearch)?;

    let destination = parameters
        .get("url")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            parameters
                .get("query")
                .and_then(Value::as_str)
                .map(build_search_url)
        })
        .ok_or_else(|| "Web search intent requires url or query".to_string())?;

    let final_url = if crate::is_valid_url(&destination) {
        destination
    } else {
        crate::build_fallback_url(&destination)
    };

    crate::open_url_internal(&final_url, app_handle)?;
    event_bus.emit(&crate::Event::IntentExecuted {
        intent_name: "search_web".to_string(),
        duration_ms: 0,
    });

    Ok(format!("Opened: {}", final_url))
}

pub fn execute_plugin_action(
    parameters: &Value,
    memory_store: &crate::MemoryStore,
    plugin_registry: &crate::plugin_registry::PluginRegistry,
    event_bus: &crate::EventBus,
) -> Result<String, String> {
    let plugin_id = string_param(parameters, &["plugin_id", "plugin"])?;
    let command = string_param(parameters, &["command", "action"])?;

    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let result = crate::plugin_registry::execute_plugin_command(&conn, plugin_registry, plugin_id, command)?;
    event_bus.emit(&crate::Event::IntentExecuted {
        intent_name: "plugin_action".to_string(),
        duration_ms: 0,
    });

    Ok(format!("Plugin result: {}", result))
}

pub async fn execute_ai_query(
    parameters: &Value,
    user_message: &str,
) -> Result<String, String> {
    let query = parameters
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or(user_message);

    if query.trim().is_empty() {
        return Ok("I'm not sure what you're asking about.".to_string());
    }

    let prompt = super::prompt_templates::build_ai_assistant_query_prompt(query);

    let answer = super::llm_client::generate_structured_response(prompt).await?;
    Ok(answer)
}


fn first_nonempty_string_param<'a>(parameters: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| parameters.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn string_param<'a>(parameters: &'a Value, keys: &[&str]) -> Result<&'a str, String> {
    first_nonempty_string_param(parameters, keys)
        .ok_or_else(|| format!("Missing required parameter. Expected one of: {}", keys.join(", ")))
}

fn infer_reminder_content_from_message(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return "Reminder".to_string();
    }

    let normalized = trimmed.to_lowercase();
    let candidates = ["remind me to ", "remind me about ", "remind me ", "set a reminder to ", "set reminder to "];

    for prefix in candidates {
        if normalized.starts_with(prefix) {
            let inferred = trimmed[prefix.len()..].trim();
            if !inferred.is_empty() {
                return inferred.to_string();
            }
        }
    }

    trimmed.to_string()
}

fn timestamp_param(parameters: &Value) -> Option<i64> {
    parameters
        .get("trigger_at")
        .and_then(Value::as_i64)
        .or_else(|| parameters.get("timestamp").and_then(Value::as_i64))
}

fn parse_time_description(value: &str) -> Option<i64> {
    let normalized = normalize_time_description(value);
    let now = Local::now();

    if let Some(ts) = parse_relative_time_description(&normalized, now) {
        return Some(ts);
    }

    if let Some(ts) = parse_tomorrow_time(&normalized, now) {
        return Some(ts);
    }

    if let Some(ts) = parse_weekday_time(&normalized, now) {
        return Some(ts);
    }

    if let Some(ts) = parse_absolute_datetime(&normalized) {
        return Some(ts);
    }

    parse_time_only(&normalized, now)
}

fn normalize_time_description(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(',', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_relative_time_description(value: &str, now: chrono::DateTime<Local>) -> Option<i64> {
    let rest = value.strip_prefix("in ")?;
    let tokens = rest.split_whitespace().collect::<Vec<_>>();

    if tokens.len() >= 2 {
        let amount = tokens[0].parse::<i64>().ok()?.max(1);
        let unit = tokens[1];

        let seconds = if unit.starts_with("min") || unit == "m" {
            amount * 60
        } else if unit.starts_with("hour") || unit == "hr" || unit == "h" {
            amount * 3600
        } else if unit.starts_with("day") || unit == "d" {
            amount * 86_400
        } else if unit.starts_with("week") || unit == "w" {
            amount * 604_800
        } else {
            return None;
        };

        return Some(now.timestamp() + seconds);
    }

    None
}

fn parse_tomorrow_time(value: &str, now: chrono::DateTime<Local>) -> Option<i64> {
    if value == "tonight" {
        let evening = NaiveTime::from_hms_opt(21, 0, 0)?;
        let mut date = now.date_naive();
        if now.time() >= evening {
            date = date + Duration::days(1);
        }
        return build_local_timestamp(date, evening);
    }

    let remainder = if value == "tomorrow" {
        ""
    } else {
        value.strip_prefix("tomorrow")?.trim()
    };

    let date = now.date_naive() + Duration::days(1);
    let default_time = NaiveTime::from_hms_opt(9, 0, 0)?;

    if remainder.is_empty() {
        return build_local_timestamp(date, default_time);
    }

    let (period, rest_after_period) = if let Some(r) = remainder.strip_prefix("morning") {
        (Some("morning"), r.trim())
    } else if let Some(r) = remainder.strip_prefix("afternoon") {
        (Some("afternoon"), r.trim())
    } else if let Some(r) = remainder.strip_prefix("evening") {
        (Some("evening"), r.trim())
    } else if let Some(r) = remainder.strip_prefix("night") {
        (Some("night"), r.trim())
    } else {
        (None, remainder)
    };

    let period_default = match period {
        Some("morning") => NaiveTime::from_hms_opt(9, 0, 0)?,
        Some("afternoon") => NaiveTime::from_hms_opt(14, 0, 0)?,
        Some("evening") | Some("night") => NaiveTime::from_hms_opt(19, 0, 0)?,
        _ => default_time,
    };

    let explicit = rest_after_period
        .strip_prefix("at ")
        .or_else(|| Some(rest_after_period))
        .and_then(parse_clock_time)
        .or(Some(period_default))?;

    build_local_timestamp(date, explicit)
}

fn parse_weekday_time(value: &str, now: chrono::DateTime<Local>) -> Option<i64> {
    let tokens = value.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return None;
    }

    let mut idx = 0usize;
    let force_next_week = if tokens[idx] == "next" {
        idx += 1;
        true
    } else {
        false
    };

    let weekday = parse_weekday(tokens.get(idx).copied()?)?;
    idx += 1;

    let tail = tokens[idx..].join(" ");
    let time = if tail.is_empty() {
        NaiveTime::from_hms_opt(9, 0, 0)?
    } else if let Some(at_tail) = tail.strip_prefix("at ") {
        parse_clock_time(at_tail)?
    } else {
        parse_clock_time(&tail)?
    };

    let mut days_ahead = days_until_weekday(now.weekday(), weekday);
    if force_next_week {
        days_ahead = if days_ahead == 0 { 7 } else { days_ahead + 7 };
    } else if days_ahead == 0 {
        let current_time = now.time();
        if time <= current_time {
            days_ahead = 7;
        }
    }

    let target_date = now.date_naive() + Duration::days(days_ahead as i64);
    build_local_timestamp(target_date, time)
}

fn parse_absolute_datetime(value: &str) -> Option<i64> {
    for fmt in [
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d %I:%M %p",
        "%Y-%m-%d %I %p",
        "%Y/%m/%d %H:%M",
        "%Y/%m/%d %I:%M %p",
        "%Y/%m/%d %I %p",
    ] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(value, fmt) {
            if let Some(local) = Local.from_local_datetime(&naive).single() {
                return Some(local.timestamp());
            }
        }
    }

    for fmt in ["%Y-%m-%d", "%Y/%m/%d"] {
        if let Ok(date) = NaiveDate::parse_from_str(value, fmt) {
            let time = NaiveTime::from_hms_opt(9, 0, 0)?;
            return build_local_timestamp(date, time);
        }
    }

    None
}

fn parse_time_only(value: &str, now: chrono::DateTime<Local>) -> Option<i64> {
    let candidate = value.strip_prefix("at ").unwrap_or(value).trim();
    let time = parse_clock_time(candidate)?;

    let mut date = now.date_naive();
    if time <= now.time() {
        date = date + Duration::days(1);
    }

    build_local_timestamp(date, time)
}

fn parse_clock_time(raw: &str) -> Option<NaiveTime> {
    let cleaned = raw.trim().replace('.', "").replace(' ', "");
    if cleaned.is_empty() {
        return None;
    }

    if cleaned == "noon" {
        return NaiveTime::from_hms_opt(12, 0, 0);
    }

    if cleaned == "midnight" {
        return NaiveTime::from_hms_opt(0, 0, 0);
    }

    let (core, meridiem) = if let Some(c) = cleaned.strip_suffix("am") {
        (c, Some("am"))
    } else if let Some(c) = cleaned.strip_suffix("pm") {
        (c, Some("pm"))
    } else {
        (cleaned.as_str(), None)
    };

    let (hour, minute) = if let Some((h, m)) = core.split_once(':') {
        (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?)
    } else {
        (core.parse::<u32>().ok()?, 0)
    };

    if minute > 59 {
        return None;
    }

    let hour_24 = match meridiem {
        Some("am") => {
            if hour == 12 {
                0
            } else {
                hour
            }
        }
        Some("pm") => {
            if hour < 12 {
                hour + 12
            } else {
                hour
            }
        }
        None => hour,
        _ => return None,
    };

    if hour_24 > 23 {
        return None;
    }

    NaiveTime::from_hms_opt(hour_24, minute, 0)
}

fn build_local_timestamp(date: NaiveDate, time: NaiveTime) -> Option<i64> {
    let naive = NaiveDateTime::new(date, time);
    Local
        .from_local_datetime(&naive)
        .single()
        .map(|dt| dt.timestamp())
}

fn parse_weekday(value: &str) -> Option<Weekday> {
    match value {
        "monday" | "mon" => Some(Weekday::Mon),
        "tuesday" | "tue" | "tues" => Some(Weekday::Tue),
        "wednesday" | "wed" => Some(Weekday::Wed),
        "thursday" | "thu" | "thurs" => Some(Weekday::Thu),
        "friday" | "fri" => Some(Weekday::Fri),
        "saturday" | "sat" => Some(Weekday::Sat),
        "sunday" | "sun" => Some(Weekday::Sun),
        _ => None,
    }
}

fn days_until_weekday(from: Weekday, to: Weekday) -> i64 {
    let from_num = from.num_days_from_monday() as i64;
    let to_num = to.num_days_from_monday() as i64;
    (to_num - from_num).rem_euclid(7)
}

fn build_search_url(query: &str) -> String {
    let encoded = query
        .split_whitespace()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("+");
    format!("https://www.google.com/search?q={}", encoded)
}

