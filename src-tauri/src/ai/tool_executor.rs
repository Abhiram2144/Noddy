use serde_json::Value;
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Weekday};
use tauri::Emitter;

fn parse_json_array(raw: &str) -> Option<Vec<Value>> {
    // Try direct parse first
    if let Ok(arr) = serde_json::from_str::<Vec<Value>>(raw.trim()) {
        return Some(arr);
    }
    // Strip markdown code fences that LLMs sometimes add
    let stripped = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str::<Vec<Value>>(stripped).ok()
}

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

pub async fn execute_save_memory(
    parameters: &Value,
    user_id: &str,
    memory_store: &crate::MemoryStore,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    permissions.check_permission(crate::Capability::MemoryWrite)?;

    let content = string_param(parameters, &["content", "memory", "text"])?;

    // Bulk schedule paste path (e.g. "Mon: Big Data 10am, OS 2pm").
    let bulk_entries = super::schedule_parser::parse_bulk_schedule_input(content);
    if !bulk_entries.is_empty() {
        let labels = save_schedule_entries(memory_store, user_id, event_bus, &bulk_entries)?;
        return Ok(format!("Got it, saved to memory: {}.", labels.join("; ")));
    }

    // If the content looks like a class schedule, parse it into structured memory entries
    if looks_like_schedule(content) {
        let parse_prompt = super::prompt_templates::build_timetable_parser_prompt(content);
        if let Ok(raw) = super::llm_client::generate_structured_response(parse_prompt).await {
            if let Some(parsed) = parse_json_array(&raw) {
                if !parsed.is_empty() {
                    let mut labels = Vec::new();
                    for entry in &parsed {
                        let day = entry.get("day").and_then(Value::as_str).unwrap_or("").to_lowercase();
                        let subject = match entry.get("subject").and_then(Value::as_str) {
                            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
                            _ => continue,
                        };
                        let time = entry.get("time").and_then(Value::as_str).unwrap_or("").to_string();

                        let memory_text = match (day.is_empty(), time.is_empty()) {
                            (false, false) => format!("class of {} at {} on {}", subject, time, day),
                            (false, true)  => format!("class of {} on {}", subject, day),
                            (true,  false) => format!("class of {} at {}", subject, time),
                            (true,  true)  => format!("class of {}", subject),
                        };

                        crate::save_memory(memory_store, user_id, &memory_text)?;
                        event_bus.emit(&crate::Event::MemorySaved(memory_text));

                        let label = match (day.is_empty(), time.is_empty()) {
                            (false, false) => format!("{} on {} at {}", subject, day, time),
                            (false, true)  => format!("{} on {}", subject, day),
                            (true,  false) => format!("{} at {}", subject, time),
                            (true,  true)  => subject.clone(),
                        };
                        labels.push(label);
                    }
                    if !labels.is_empty() {
                        return Ok(format!("Got it, saved to memory: {}.", labels.join("; ")));
                    }
                }
            }
        }
        // Fallthrough: save as plain memory if parsing failed
    }

    crate::save_memory(memory_store, user_id, content)?;
    event_bus.emit(&crate::Event::MemorySaved(content.to_string()));
    Ok("Got it, I'll remember that.".to_string())
}

pub fn execute_update_memory(
    parameters: &Value,
    user_id: &str,
    memory_store: &crate::MemoryStore,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    permissions.check_permission(crate::Capability::MemoryWrite)?;

    let query = first_nonempty_string_param(parameters, &["query", "keyword", "target", "old_content"])
        .unwrap_or("class");

    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let matches = crate::memory_store::search_memories(&conn, user_id, query.to_string(), 5)?;
    let target = matches
        .first()
        .ok_or_else(|| format!("I couldn't find a memory to update for '{}'.", query))?;

    let new_content = if let Some(explicit) = first_nonempty_string_param(
        parameters,
        &["new_content", "content", "replacement", "value"],
    ) {
        explicit.to_string()
    } else if let Some(new_time) = first_nonempty_string_param(parameters, &["new_time", "time"]) {
        apply_time_correction(&target.content, new_time)
    } else {
        return Err("Update intent requires new_content or new_time".to_string());
    };

    crate::memory_store::update_memory_content(&conn, user_id, &target.id, &new_content)?;
    crate::memory_intelligence_service::link_related_memories(&conn, user_id, &target.id)?;
    crate::memory_intelligence_service::calculate_memory_importance(&conn, user_id, &target.id)?;

    event_bus.emit(&crate::Event::MemoryUpdated(target.id.clone()));
    Ok(format!("Updated memory: {}", new_content))
}

pub fn execute_delete_memory(
    parameters: &Value,
    user_id: &str,
    memory_store: &crate::MemoryStore,
    event_bus: &crate::EventBus,
    permissions: &crate::PermissionManager,
) -> Result<String, String> {
    permissions.check_permission(crate::Capability::MemoryWrite)?;

    let query = string_param(parameters, &["query", "keyword", "target", "memory"])?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let matches = crate::memory_store::search_memories(&conn, user_id, query.to_string(), 5)?;
    let target = matches
        .first()
        .ok_or_else(|| format!("I couldn't find a memory to forget for '{}'.", query))?;

    crate::memory_store::delete_memory(&conn, user_id, &target.id)?;
    event_bus.emit(&crate::Event::MemoryDeleted(target.id.clone()));
    Ok("Done. I forgot that memory.".to_string())
}

fn looks_like_schedule(text: &str) -> bool {
    let lower = text.to_lowercase();
    let has_weekday = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"]
        .iter().any(|d| lower.contains(d));
    let has_class_word = ["class", "lecture", "attend", "have ", "subject", "course", "lab"]
        .iter().any(|w| lower.contains(w));
    has_weekday && has_class_word
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
        Ok(format!("I couldn't find anything in your memories about {}.", query))
    } else {
        Ok(build_memory_search_answer(query, &results))
    }
}

fn build_memory_search_answer(query: &str, results: &[String]) -> String {
    let normalized_query = normalize_text(query);
    let asks_about_class_schedule =
        contains_any(&normalized_query, &["class", "timetable", "schedule", "week", "tomorrow"]);

    if asks_about_class_schedule {
        let temporal_focus = detect_temporal_focus(&normalized_query);
        let all_entries = collect_class_entries(results);
        let focused_entries = filter_entries_by_focus(&all_entries, temporal_focus);

        if !focused_entries.is_empty() {
            return format_schedule_answer(temporal_focus, &focused_entries);
        }

        if temporal_focus != TemporalFocus::Any {
            return match temporal_focus {
                TemporalFocus::WeekStartMonday => {
                    "I found class memories, but none clearly scheduled for Monday.".to_string()
                }
                TemporalFocus::Tomorrow => {
                    "I found class memories, but none clearly marked for tomorrow.".to_string()
                }
                TemporalFocus::Any => "I found class memories, but couldn't resolve the exact schedule.".to_string(),
            };
        }

        if let Some(first) = all_entries.first() {
            if let Some(time) = &first.time {
                return format!("You have {} at {}.", first.subject, time);
            }
            return format!("You have {}.", first.subject);
        }
    }

    if results.len() == 1 {
        return format!("From your memory: {}", results[0]);
    }

    format!(
        "Based on your memories: {}",
        results
            .iter()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(". ")
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TemporalFocus {
    Any,
    Tomorrow,
    WeekStartMonday,
}

#[derive(Clone)]
struct ClassEntry {
    subject: String,
    time: Option<String>,
    day: Option<String>,
}

fn detect_temporal_focus(normalized_query: &str) -> TemporalFocus {
    if normalized_query.contains("start of the week")
        || normalized_query.contains("start of week")
        || normalized_query.contains("beginning of the week")
        || normalized_query.contains("beginning of week")
        || normalized_query.contains("monday")
    {
        TemporalFocus::WeekStartMonday
    } else if normalized_query.contains("tomorrow") {
        TemporalFocus::Tomorrow
    } else {
        TemporalFocus::Any
    }
}

fn collect_class_entries(memories: &[String]) -> Vec<ClassEntry> {
    let mut entries = Vec::new();

    for memory in memories {
        let lower = memory.to_lowercase();
        let day = extract_weekday(&lower);
        let marker = "class of ";
        let mut start_idx = 0usize;

        while let Some(relative_idx) = lower[start_idx..].find(marker) {
            let subject_start = start_idx + relative_idx + marker.len();
            let subject_tail = &memory[subject_start..];
            let subject_tail_lower = &lower[subject_start..];

            let subject_end = subject_tail_lower
                .find(" at ")
                .or_else(|| subject_tail_lower.find(","))
                .or_else(|| subject_tail_lower.find("."))
                .unwrap_or(subject_tail.len());

            let subject = subject_tail[..subject_end]
                .trim()
                .trim_matches(|c: char| c == ',' || c == '.')
                .to_string();

            let time = subject_tail_lower
                .find(" at ")
                .and_then(|idx| extract_time_phrase(&subject_tail[idx + 4..]));

            if !subject.is_empty() {
                entries.push(ClassEntry {
                    subject,
                    time,
                    day: day.clone(),
                });
            }

            start_idx = subject_start;
        }
    }

    entries
}

fn filter_entries_by_focus(entries: &[ClassEntry], focus: TemporalFocus) -> Vec<ClassEntry> {
    match focus {
        TemporalFocus::Any => entries.to_vec(),
        TemporalFocus::Tomorrow => entries
            .iter()
            .filter(|entry| entry.day.is_none())
            .cloned()
            .collect(),
        TemporalFocus::WeekStartMonday => entries
            .iter()
            .filter(|entry| entry.day.as_deref() == Some("monday"))
            .cloned()
            .collect(),
    }
}

fn format_schedule_answer(focus: TemporalFocus, entries: &[ClassEntry]) -> String {
    let mut parts = Vec::new();
    for entry in entries.iter().take(3) {
        match &entry.time {
            Some(time) => parts.push(format!("{} at {}", entry.subject, time)),
            None => parts.push(entry.subject.clone()),
        }
    }

    if parts.is_empty() {
        return "I found your class memories, but couldn't build a clean schedule answer.".to_string();
    }

    let joined = if parts.len() == 1 {
        parts[0].clone()
    } else {
        format!("{}; then {}", parts[0], parts[1..].join("; then "))
    };

    match focus {
        TemporalFocus::WeekStartMonday => format!("At the start of the week, you have {}.", joined),
        TemporalFocus::Tomorrow => format!("Tomorrow you have {}.", joined),
        TemporalFocus::Any => format!("You have {}.", joined),
    }
}

fn extract_weekday(text: &str) -> Option<String> {
    [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ]
    .iter()
    .find(|day| text.contains(**day))
    .map(|day| (*day).to_string())
}

fn normalize_text(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
}

fn contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

fn extract_time_phrase(text: &str) -> Option<String> {
    let raw_tokens = text.split_whitespace().collect::<Vec<_>>();
    for (idx, token) in raw_tokens.iter().enumerate() {
        let cleaned = token
            .trim_matches(|c: char| !c.is_ascii_alphanumeric())
            .to_lowercase();
        if cleaned.ends_with("am") || cleaned.ends_with("pm") {
            let spaced = cleaned
                .replace("am", " AM")
                .replace("pm", " PM")
                .trim()
                .to_string();
            if !spaced.is_empty() {
                return Some(spaced);
            }
        }

        if idx + 1 < raw_tokens.len() {
            let next = raw_tokens[idx + 1]
                .trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_lowercase();
            if (next == "am" || next == "pm") && cleaned.chars().all(|c| c.is_ascii_digit()) {
                return Some(format!("{} {}", cleaned, next.to_uppercase()));
            }
        }
    }

    None
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
    user_id: &str,
    memory_store: &crate::MemoryStore,
) -> Result<String, String> {
    let query = parameters
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or(user_message);

    if query.trim().is_empty() {
        return Ok("I'm not sure what you're asking about.".to_string());
    }

    let runtime_context = super::context_builder::build_runtime_context(memory_store, user_id);
    let semantic_keywords = super::context_builder::extract_semantic_keywords(query).await;

    // Fetch relevant personal context to ground the answer.
    let mut seen = std::collections::HashSet::new();
    let mut memories = Vec::new();

    for keyword in semantic_keywords.iter().take(6) {
        for memory in crate::search_memories(memory_store, user_id, keyword).unwrap_or_default() {
            if seen.insert(memory.clone()) {
                memories.push(memory);
            }
            if memories.len() >= 8 {
                break;
            }
        }
        if memories.len() >= 8 {
            break;
        }
    }

    if memories.is_empty() {
        memories = crate::search_memories(memory_store, user_id, query).unwrap_or_default();
    }

    let prompt = if memories.is_empty() {
        super::prompt_templates::build_ai_assistant_query_prompt(query, &runtime_context)
    } else {
        super::prompt_templates::build_ai_assistant_query_with_context_prompt(
            query,
            &memories,
            &runtime_context,
        )
    };

    let answer = super::llm_client::generate_structured_response(prompt).await?;
    Ok(answer)
}

fn save_schedule_entries(
    memory_store: &crate::MemoryStore,
    user_id: &str,
    event_bus: &crate::EventBus,
    entries: &[super::schedule_parser::ScheduleEntry],
) -> Result<Vec<String>, String> {
    let mut labels = Vec::new();

    for entry in entries {
        let memory_text = format!("class of {} at {} on {}", entry.subject, entry.time, entry.day);
        crate::save_memory(memory_store, user_id, &memory_text)?;
        event_bus.emit(&crate::Event::MemorySaved(memory_text));
        labels.push(format!("{} on {} at {}", entry.subject, entry.day, entry.time));
    }

    Ok(labels)
}

fn apply_time_correction(existing: &str, new_time: &str) -> String {
    let lower = existing.to_lowercase();
    if let Some(idx) = lower.find(" at ") {
        let prefix = &existing[..idx + 4];
        let suffix = &existing[idx + 4..];
        let day_idx = suffix.to_lowercase().find(" on ");
        if let Some(on_idx) = day_idx {
            let after_day = &suffix[on_idx..];
            return format!("{}{}{}", prefix, new_time.trim(), after_day);
        }
        return format!("{}{}", prefix, new_time.trim());
    }

    format!("{} at {}", existing.trim(), new_time.trim())
}



fn first_nonempty_string_param<'a>(parameters: &'a Value, keys: &[&str]) -> Option<&'a str> {    keys.iter()
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

