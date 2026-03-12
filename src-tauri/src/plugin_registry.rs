use crate::plugin_interface::{Plugin, PluginEvent};
use chrono::{self, Timelike};
use urlencoding;
use async_trait::async_trait;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRecord {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub config_json: Option<String>,
    pub description: String,
    pub provider: String,
    pub capabilities: Vec<String>,
}

#[derive(Clone)]
pub struct PluginRegistry {
    handlers: Arc<HashMap<String, Arc<dyn Plugin>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let mut handlers: HashMap<String, Arc<dyn Plugin>> = HashMap::new();
        register_builtin_plugin(&mut handlers, Arc::new(GoogleCalendarPlugin));
        register_builtin_plugin(&mut handlers, Arc::new(OutlookPlugin));
        register_builtin_plugin(&mut handlers, Arc::new(SystemPlugin));
        Self {
            handlers: Arc::new(handlers),
        }
    }

    pub fn handler(&self, plugin_id: &str) -> Option<Arc<dyn Plugin>> {
        self.handlers.get(plugin_id).cloned()
    }

    pub fn handlers(&self) -> Vec<Arc<dyn Plugin>> {
        self.handlers.values().cloned().collect()
    }
}

pub fn register_plugin(conn: &Connection, plugin: &dyn Plugin) -> Result<(), String> {
    conn.execute(
        "INSERT INTO plugins (id, name, enabled, config_json)
         VALUES (?1, ?2, COALESCE((SELECT enabled FROM plugins WHERE id = ?1), 0), COALESCE((SELECT config_json FROM plugins WHERE id = ?1), NULL))
         ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        params![plugin.id(), plugin.name()],
    )
    .map_err(|e| format!("Failed to register plugin: {}", e))?;

    Ok(())
}

pub fn seed_registered_plugins(conn: &Connection, registry: &PluginRegistry) -> Result<(), String> {
    for plugin in registry.handlers() {
        register_plugin(conn, plugin.as_ref())?;
    }
    Ok(())
}

pub fn set_plugin_enabled(
    conn: &Connection,
    plugin_id: &str,
    enabled: bool,
) -> Result<(), String> {
    let enabled_int = if enabled { 1 } else { 0 };
    conn.execute(
        "INSERT INTO plugins (id, name, enabled) VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET enabled=?3",
        params![plugin_id, plugin_id, enabled_int],
    )
    .map_err(|e| format!("Failed to set plugin enabled state: {}", e))?;
    Ok(())
}

pub async fn initialize_plugin(
    registry: &PluginRegistry,
    plugin_id: &str,
    config_json: Option<&str>,
) -> Result<(), String> {
    let handler = registry.handler(plugin_id).ok_or("Plugin handler not found")?;
    handler.initialize(config_json).await
}

pub fn disable_plugin(conn: &Connection, plugin_id: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE plugins SET enabled = 0 WHERE id = ?1",
        params![plugin_id],
    )
    .map_err(|e| format!("Failed to disable plugin: {}", e))?;

    Ok(())
}

pub fn get_active_plugins(conn: &Connection, registry: &PluginRegistry) -> Result<Vec<PluginRecord>, String> {
    let all_plugins = get_plugins(conn, registry)?;
    Ok(all_plugins.into_iter().filter(|plugin| plugin.enabled).collect())
}

pub fn get_plugins(conn: &Connection, registry: &PluginRegistry) -> Result<Vec<PluginRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, enabled, config_json
             FROM plugins
             ORDER BY name ASC",
        )
        .map_err(|e| format!("Failed to prepare plugin query: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? != 0,
                row.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| format!("Failed to query plugins: {}", e))?;

    let raw_plugins = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect plugins: {}", e))?;

    Ok(raw_plugins
        .into_iter()
        .map(|(id, name, enabled, config_json)| {
            if let Some(handler) = registry.handler(&id) {
                PluginRecord {
                    id,
                    name,
                    enabled,
                    config_json,
                    description: handler.description().to_string(),
                    provider: handler.provider().to_string(),
                    capabilities: handler.capabilities().iter().map(|item| (*item).to_string()).collect(),
                }
            } else {
                PluginRecord {
                    id,
                    name,
                    enabled,
                    config_json,
                    description: "Unknown plugin".to_string(),
                    provider: "custom".to_string(),
                    capabilities: Vec::new(),
                }
            }
        })
        .collect())
}

pub fn update_plugin_config(
    conn: &Connection,
    registry: &PluginRegistry,
    plugin_id: &str,
    config: &Value,
) -> Result<PluginRecord, String> {
    let trimmed = serde_json::to_string(config)
        .map_err(|e| format!("Invalid plugin config JSON: {}", e))?;
    
    conn.execute(
        "UPDATE plugins SET config_json = ?1 WHERE id = ?2",
        params![trimmed, plugin_id],
    )
    .map_err(|e| format!("Failed to update plugin config: {}", e))?;

    get_plugin(conn, registry, plugin_id)
}

pub fn get_plugin(conn: &Connection, registry: &PluginRegistry, plugin_id: &str) -> Result<PluginRecord, String> {
    get_plugins(conn, registry)?
        .into_iter()
        .find(|plugin| plugin.id == plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))
}

pub async fn dispatch_event(
    registry: &PluginRegistry,
    active_plugins: Vec<PluginRecord>,
    event: &PluginEvent,
) -> Result<(), String> {
    for plugin in active_plugins {
        if let Some(handler) = registry.handler(&plugin.id) {
            if let Err(e) = handler.handle_event(event, plugin.config_json.as_deref()).await {
                eprintln!("⚠️  Plugin '{}' failed to handle event: {}", plugin.id, e);
            }
        }
    }
    Ok(())
}

pub async fn execute_plugin_command(
    registry: &PluginRegistry,
    plugin_id: &str,
    command: &str,
    config_json: Option<&str>,
) -> Result<Value, String> {
    let handler = registry
        .handler(plugin_id)
        .ok_or_else(|| format!("Unknown plugin: {}", plugin_id))?;

    handler.execute_command(command, config_json).await
}

pub fn plugin_event_from_core_event(event: &crate::Event) -> Option<PluginEvent> {
    match event {
        crate::Event::ReminderScheduled(content) => Some(PluginEvent::ReminderScheduled {
            content: content.clone(),
        }),
        crate::Event::ReminderTriggered(content) => Some(PluginEvent::ReminderFired {
            content: content.clone(),
        }),
        crate::Event::TaskCompleted { task_id, task_type } => Some(PluginEvent::TaskCompleted {
            task_id: task_id.clone(),
            task_type: task_type.clone(),
        }),
        _ => None,
    }
}

pub fn get_plugin_config(conn: &Connection, plugin_id: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT config_json FROM plugins WHERE id = ?1",
        params![plugin_id],
        |row| row.get(0),
    )
    .map_err(|e| format!("Failed to load plugin config: {}", e))
}

fn register_builtin_plugin(handlers: &mut HashMap<String, Arc<dyn Plugin>>, plugin: Arc<dyn Plugin>) {
    handlers.insert(plugin.id().to_string(), plugin);
}

struct GoogleCalendarPlugin;

#[async_trait]
impl Plugin for GoogleCalendarPlugin {
    fn id(&self) -> &'static str {
        "google_calendar_plugin"
    }

    fn name(&self) -> &'static str {
        "Google Calendar"
    }

    fn description(&self) -> &'static str {
        "Sync reminders with Google Calendar and create calendar events automatically."
    }

    fn provider(&self) -> &'static str {
        "google"
    }

    fn capabilities(&self) -> &'static [&'static str] {
        &[
            "sync reminders",
            "fetch_events",
            "create calendar events",
            "delete calendar events",
            "update calendar events",
        ]
    }

    async fn initialize(&self, config_json: Option<&str>) -> Result<(), String> {
        validate_config(config_json)
    }

    async fn authenticate(&self, _app_handle: &tauri::AppHandle) -> Result<String, String> {
        let client_id = std::env::var("GOOGLE_CLIENT_ID").map_err(|_| "Missing GOOGLE_CLIENT_ID".to_string())?;
        let redirect_uri = "http://localhost:1421";
        let scope = "https://www.googleapis.com/auth/calendar.events";
        
        let auth_url = format!(
            "https://accounts.google.com/o/oauth2/v2/auth?client_id={}&response_type=code&redirect_uri={}&scope={}&access_type=offline&prompt=consent",
            urlencoding::encode(&client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(scope)
        );
        
        Ok(auth_url)
    }

    async fn handle_event(&self, event: &PluginEvent, config_json: Option<&str>) -> Result<(), String> {
        let config = read_config(config_json)?;
        if let PluginEvent::ReminderFired { content } | PluginEvent::ReminderScheduled { content } = event {
            let calendar_id = config
                .get("calendar_id")
                .and_then(Value::as_str)
                .unwrap_or("primary");
            println!(
                "[PLUGIN][Google Calendar] syncing reminder '{}' to calendar '{}'",
                content, calendar_id
            );
        }
        Ok(())
    }

    async fn execute_command(&self, command: &str, config_json: Option<&str>) -> Result<Value, String> {
        let config = read_config(config_json)?;
        
        if command == "fetch_events" {
            let tokens_json = config.get("tokens").ok_or_else(|| "Not authenticated".to_string())?;
            let tokens: crate::oauth::OAuthTokens = serde_json::from_value(tokens_json.clone())
                .map_err(|e| format!("Invalid tokens: {}", e))?;

            // Check if expired
            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            if tokens.expires_at < now {
                return Err("Authentication expired. Please sign in again.".to_string());
            }

            let client = reqwest::Client::new();
            let calendar_id = config.get("calendar_id").and_then(Value::as_str).unwrap_or("primary");
            let time_min = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            
            let url = format!("https://www.googleapis.com/calendar/v3/calendars/{}/events?maxResults=10&timeMin={}", 
                urlencoding::encode(calendar_id), 
                urlencoding::encode(&time_min)
            );

            let response = client.get(url)
                .header("Authorization", format!("Bearer {}", tokens.access_token))
                .send()
                .await
                .map_err(|e| format!("Google Calendar API request failed: {}", e))?;

            if !response.status().is_success() {
                let err = response.text().await.unwrap_or_default();
                return Err(format!("Google Calendar API error: {}", err));
            }

            let data: Value = response.json().await.map_err(|e| format!("Failed to parse Google response: {}", e))?;
            
            let events = data["items"].as_array().unwrap_or(&vec![]).iter().map(|item| {
                let start = item["start"]["dateTime"].as_str().or(item["start"]["date"].as_str()).unwrap_or("");
                let end = item["end"]["dateTime"].as_str().or(item["end"]["date"].as_str()).unwrap_or("");
                let is_all_day = item["start"]["date"].is_string();

                json!({
                    "id": item["id"],
                    "subject": item["summary"],
                    "start": start,
                    "end": end,
                    "location": item["location"],
                    "is_all_day": is_all_day
                })
            }).collect::<Vec<_>>();

            Ok(json!({
                "events": events,
                "metadata": {
                    "is_demo": false
                },
                "status": "success",
                "provider": "google"
            }))
        } else if command.starts_with("create_event") {
            let tokens_json = config.get("tokens").ok_or_else(|| "Not authenticated".to_string())?;
            let tokens: crate::oauth::OAuthTokens = serde_json::from_value(tokens_json.clone())
                .map_err(|e| format!("Invalid tokens: {}", e))?;

            let params_json = command.splitn(2, ':').nth(1).unwrap_or("{}");
            let params: Value = serde_json::from_str(params_json).unwrap_or(json!({}));
            
            let title = params.get("title").and_then(Value::as_str).unwrap_or("New Meeting");
            let start_time_raw = params.get("start_time").and_then(Value::as_str).unwrap_or("next_hour");
            
            let mut start_dt = chrono::Utc::now();
            if start_time_raw == "next_hour" {
                 start_dt = start_dt.date_naive()
                    .and_hms_opt(start_dt.hour(), 0, 0)
                    .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc) + chrono::Duration::hours(1))
                    .unwrap_or(start_dt);
            } else if start_time_raw == "tomorrow" {
                start_dt = (start_dt + chrono::Duration::days(1)).date_naive()
                    .and_hms_opt(9, 0, 0) // Default to 9 AM tomorrow
                    .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc))
                    .unwrap_or(start_dt);
            } else {
                start_dt = chrono::DateTime::parse_from_rfc3339(start_time_raw)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or(start_dt);
            }

            let duration_mins = params.get("duration_minutes").and_then(Value::as_i64).unwrap_or(60);
            let end_dt = start_dt + chrono::Duration::minutes(duration_mins);

            let client = reqwest::Client::new();
            let calendar_id = config.get("calendar_id").and_then(Value::as_str).unwrap_or("primary");
            let url = format!("https://www.googleapis.com/calendar/v3/calendars/{}/events", urlencoding::encode(calendar_id));

            let event_body = json!({
                "summary": title,
                "start": { "dateTime": start_dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true) },
                "end": { "dateTime": end_dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true) }
            });

            let response = client.post(url).header("Authorization", format!("Bearer {}", tokens.access_token)).json(&event_body).send().await
                .map_err(|e| format!("Google Calendar API create event failed: {}", e))?;

            if !response.status().is_success() {
                return Err(format!("Google Calendar API error: {}", response.text().await.unwrap_or_default()));
            }

            let data: Value = response.json().await.map_err(|e| format!("Failed to parse response: {}", e))?;
            Ok(json!({ "status": "success", "event_id": data["id"], "message": format!("Successfully created event: {}", title) }))

        } else if command.starts_with("delete_event") {
            let tokens_json = config.get("tokens").ok_or_else(|| "Not authenticated".to_string())?;
            let tokens: crate::oauth::OAuthTokens = serde_json::from_value(tokens_json.clone())
                .map_err(|e| format!("Invalid tokens: {}", e))?;

            let params_json = command.splitn(2, ':').nth(1).unwrap_or("{}");
            let params: Value = serde_json::from_str(params_json).unwrap_or(json!({}));
            let query = params.get("query").and_then(Value::as_str).unwrap_or("");

            if query.is_empty() { return Err("No event query provided.".to_string()); }

            let client = reqwest::Client::new();
            let calendar_id = config.get("calendar_id").and_then(Value::as_str).unwrap_or("primary");

            // Broaden search: search from 1 day ago to 30 days in the future
            let now = chrono::Utc::now();
            let time_min = (now - chrono::Duration::days(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            let time_max = (now + chrono::Duration::days(30)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

            let search_url = format!("https://www.googleapis.com/calendar/v3/calendars/{}/events?q={}&timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime&maxResults=5", 
                urlencoding::encode(calendar_id), 
                urlencoding::encode(query),
                urlencoding::encode(&time_min),
                urlencoding::encode(&time_max)
            );

            let search_resp = client.get(search_url).header("Authorization", format!("Bearer {}", tokens.access_token)).send().await
                .map_err(|e| format!("Search failed: {}", e))?;

            let search_data: Value = search_resp.json().await.map_err(|e| format!("Parse failed: {}", e))?;
            let items = search_data["items"].as_array().ok_or("No matching events found.")?;
            
            if items.is_empty() {
                return Err(format!("Could not find any event matching '{}' in the next 30 days.", query));
            }

            // Find the best match: case-insensitive title match or first item if generic
            let event = items.iter().find(|item| {
                item["summary"].as_str().map(|s| s.to_lowercase() == query.to_lowercase()).unwrap_or(false)
            }).unwrap_or(&items[0]);

            let event_id = event["id"].as_str().ok_or("Invalid event ID.")?;
            let title = event["summary"].as_str().unwrap_or("Untitled");

            let delete_url = format!("https://www.googleapis.com/calendar/v3/calendars/{}/events/{}", 
                urlencoding::encode(calendar_id), 
                urlencoding::encode(event_id)
            );

            let resp = client.delete(delete_url).header("Authorization", format!("Bearer {}", tokens.access_token)).send().await
                .map_err(|e| format!("Delete failed: {}", e))?;

            if !resp.status().is_success() {
                return Err(format!("Deletion failed: {}", resp.status()));
            }

            Ok(json!({ "status": "success", "message": format!("Deleted event: {}", title) }))
        } else if command.starts_with("update_event") {
            // Simplified update: for now we just find by title and we could extend this
            let params_json = command.splitn(2, ':').nth(1).unwrap_or("{}");
            let params: Value = serde_json::from_str(params_json).unwrap_or(json!({}));
            let original_text = params.get("original_text").and_then(Value::as_str).unwrap_or("");
            
            Ok(json!({ "status": "info", "message": "Update functionality is being refined. Please use 'delete' and 'add' for now or provide specific details.", "original_text": original_text }))
        } else {
            Ok(json!({
                "plugin": self.id(),
                "command": command,
                "calendar_id": config.get("calendar_id").and_then(Value::as_str).unwrap_or("primary"),
                "status": "ready"
            }))
        }
    }
}

struct OutlookPlugin;

#[async_trait]
impl Plugin for OutlookPlugin {
    fn id(&self) -> &'static str {
        "outlook_plugin"
    }

    fn name(&self) -> &'static str {
        "Outlook"
    }

    fn description(&self) -> &'static str {
        "Sync reminders with Outlook tasks and route follow-up actions into Microsoft workflows."
    }

    fn provider(&self) -> &'static str {
        "outlook"
    }

    fn capabilities(&self) -> &'static [&'static str] {
        &[
            "sync reminders",
            "create outlook tasks",
            "fetch_events",
            "create calendar events",
            "delete calendar events",
            "update calendar events",
        ]
    }

    async fn initialize(&self, config_json: Option<&str>) -> Result<(), String> {
        validate_config(config_json)
    }

    async fn authenticate(&self, _app_handle: &tauri::AppHandle) -> Result<String, String> {
        let client_id = std::env::var("OUTLOOK_CLIENT_ID").map_err(|_| "Missing OUTLOOK_CLIENT_ID".to_string())?;
        let redirect_uri = "http://localhost:1421";
        let scope = "offline_access Calendars.ReadWrite";
        
        let auth_url = format!(
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={}&response_type=code&redirect_uri={}&response_mode=query&scope={}",
            urlencoding::encode(&client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(scope)
        );
        
        Ok(auth_url)
    }

    async fn handle_event(&self, event: &PluginEvent, config_json: Option<&str>) -> Result<(), String> {
        let config = read_config(config_json)?;
        match event {
            PluginEvent::ReminderFired { content } => {
                let task_list = config
                    .get("task_list")
                    .and_then(Value::as_str)
                    .unwrap_or("Tasks");
                println!(
                    "[PLUGIN][Outlook] syncing reminder '{}' to task list '{}'",
                    content, task_list
                );
            }
            PluginEvent::TaskCompleted { task_id, task_type } => {
                println!(
                    "[PLUGIN][Outlook] observed completed task '{}' of type '{}'",
                    task_id, task_type
                );
            }
            _ => {}
        }
        Ok(())
    }

    async fn execute_command(&self, command: &str, config_json: Option<&str>) -> Result<Value, String> {
        let config = read_config(config_json)?;
        match command {
            "fetch_events" => {
                let tokens_json = config.get("tokens").ok_or_else(|| "Not authenticated".to_string())?;
                let tokens: crate::oauth::OAuthTokens = serde_json::from_value(tokens_json.clone())
                    .map_err(|e| format!("Invalid tokens: {}", e))?;

                // Check if expired
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                if tokens.expires_at < now {
                    return Err("Authentication expired. Please sign in again.".to_string());
                }

                let client = reqwest::Client::new();
                let response = client.get("https://graph.microsoft.com/v1.0/me/events")
                    .header("Authorization", format!("Bearer {}", tokens.access_token))
                    .send()
                    .await
                    .map_err(|e| format!("Outlook Calendar API request failed: {}", e))?;

                if !response.status().is_success() {
                    let err = response.text().await.unwrap_or_default();
                    return Err(format!("Outlook Calendar API error: {}", err));
                }

                let data: Value = response.json().await.map_err(|e| format!("Failed to parse Graph response: {}", e))?;
                
                // Map Graph API format to Noddy format
                let events = data["value"].as_array().unwrap_or(&vec![]).iter().map(|item| {
                    json!({
                        "id": item["id"],
                        "subject": item["subject"],
                        "start": item["start"]["dateTime"].as_str().unwrap_or(""),
                        "end": item["end"]["dateTime"].as_str().unwrap_or(""),
                        "location": item["location"]["displayName"].as_str().unwrap_or(""),
                        "is_all_day": item["isAllDay"].as_bool().unwrap_or(false)
                    })
                }).collect::<Vec<_>>();

                Ok(json!({
                    "events": events,
                    "metadata": {
                        "is_demo": false
                    },
                    "status": "success",
                    "provider": "outlook"
                }))
            },
            command if command.starts_with("create_event") => {
                let tokens_json = config.get("tokens").ok_or_else(|| "Not authenticated".to_string())?;
                let tokens: crate::oauth::OAuthTokens = serde_json::from_value(tokens_json.clone())
                    .map_err(|e| format!("Invalid tokens: {}", e))?;

                let params_json = command.splitn(2, ':').nth(1).unwrap_or("{}");
                let params: Value = serde_json::from_str(params_json).unwrap_or(json!({}));
                
                let title = params.get("title").and_then(Value::as_str).unwrap_or("New Meeting");
                let start_time_raw = params.get("start_time").and_then(Value::as_str).unwrap_or("next_hour");
                
                let mut start_dt = chrono::Utc::now();
                if start_time_raw == "next_hour" {
                     start_dt = start_dt.date_naive()
                        .and_hms_opt(start_dt.hour(), 0, 0)
                        .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc) + chrono::Duration::hours(1))
                        .unwrap_or(start_dt);
                } else if start_time_raw == "tomorrow" {
                    start_dt = (start_dt + chrono::Duration::days(1)).date_naive()
                        .and_hms_opt(9, 0, 0)
                        .map(|dt| chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc))
                        .unwrap_or(start_dt);
                } else {
                    start_dt = chrono::DateTime::parse_from_rfc3339(start_time_raw)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or(start_dt);
                }

                let duration_mins = params.get("duration_minutes").and_then(Value::as_i64).unwrap_or(60);
                let end_dt = start_dt + chrono::Duration::minutes(duration_mins);

                let client = reqwest::Client::new();
                let url = "https://graph.microsoft.com/v1.0/me/events";

                let event_body = json!({
                    "subject": title,
                    "start": {
                        "dateTime": start_dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
                        "timeZone": "UTC"
                    },
                    "end": {
                        "dateTime": end_dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
                        "timeZone": "UTC"
                    }
                });

                let response = client.post(url)
                    .header("Authorization", format!("Bearer {}", tokens.access_token))
                    .json(&event_body)
                    .send()
                    .await
                    .map_err(|e| format!("Outlook Calendar API create event failed: {}", e))?;

                if !response.status().is_success() {
                    let err = response.text().await.unwrap_or_default();
                    return Err(format!("Outlook Calendar API error: {}", err));
                }

                let data: Value = response.json().await.map_err(|e| format!("Failed to parse Outlook response: {}", e))?;
                
                Ok(json!({
                    "status": "success",
                    "event_id": data["id"],
                    "message": format!("Successfully created Outlook event: {}", title)
                }))
            },
            command if command.starts_with("delete_event") => {
                let tokens_json = config.get("tokens").ok_or_else(|| "Not authenticated".to_string())?;
                let tokens: crate::oauth::OAuthTokens = serde_json::from_value(tokens_json.clone())
                    .map_err(|e| format!("Invalid tokens: {}", e))?;

                let params_json = command.splitn(2, ':').nth(1).unwrap_or("{}");
                let params: Value = serde_json::from_str(params_json).unwrap_or(json!({}));
                let query = params.get("query").and_then(Value::as_str).unwrap_or("");

                if query.is_empty() { return Err("No event query provided.".to_string()); }

                let client = reqwest::Client::new();
                
                // 1. Search for the event by subject
                let search_url = format!("https://graph.microsoft.com/v1.0/me/events?$search=\"subject:{}\"&$select=id,subject", 
                    urlencoding::encode(query)
                );

                let search_resp = client.get(search_url)
                    .header("Authorization", format!("Bearer {}", tokens.access_token))
                    .send()
                    .await
                    .map_err(|e| format!("Outlook search failed: {}", e))?;

                let search_data: Value = search_resp.json().await.map_err(|e| format!("Parse failed: {}", e))?;
                let items = search_data["value"].as_array().ok_or("No matching events found in Outlook.")?;
                
                if items.is_empty() {
                    return Err(format!("Could not find any Outlook event matching '{}'.", query));
                }

                // Pick best match or first
                let event = items.iter().find(|item| {
                    item["subject"].as_str().map(|s| s.to_lowercase() == query.to_lowercase()).unwrap_or(false)
                }).unwrap_or(&items[0]);

                let event_id = event["id"].as_str().ok_or("Invalid event ID.")?;
                let title = event["subject"].as_str().unwrap_or("Untitled");

                // 2. Delete it
                let delete_url = format!("https://graph.microsoft.com/v1.0/me/events/{}", event_id);

                let resp = client.delete(delete_url)
                    .header("Authorization", format!("Bearer {}", tokens.access_token))
                    .send()
                    .await
                    .map_err(|e| format!("Outlook delete failed: {}", e))?;

                if !resp.status().is_success() {
                    return Err(format!("Outlook deletion failed: {}", resp.status()));
                }

                Ok(json!({ "status": "success", "message": format!("Deleted Outlook event: {}", title) }))
            },
            command if command.starts_with("update_event") => {
                Ok(json!({ "status": "info", "message": "Outlook update functionality is being refined. Please use delete/add for now." }))
            },
            _ => {
                let config = read_config(config_json)?;
                Ok(json!({
                    "plugin": self.id(),
                    "command": command,
                    "task_list": config.get("task_list").and_then(Value::as_str).unwrap_or("Tasks"),
                    "status": "ready"
                }))
            }
        }
    }
}

pub struct SystemPlugin;

#[async_trait]
impl Plugin for SystemPlugin {
    fn id(&self) -> &'static str { "system_plugin" }
    fn name(&self) -> &'static str { "System Control" }
    fn description(&self) -> &'static str { "Global control for Windows volume, brightness, and power." }
    fn provider(&self) -> &'static str { "windows" }
    fn capabilities(&self) -> &'static [&'static str] {
        &["set_volume", "get_volume", "set_brightness", "get_brightness", "system_control"]
    }

    async fn initialize(&self, _config_json: Option<&str>) -> Result<(), String> { Ok(()) }

    async fn authenticate(&self, _app_handle: &tauri::AppHandle) -> Result<String, String> {
        Err("Authentication not required for System Control.".to_string())
    }

    async fn handle_event(&self, _event: &PluginEvent, _config_json: Option<&str>) -> Result<(), String> { Ok(()) }

    async fn execute_command(&self, command: &str, _config_json: Option<&str>) -> Result<Value, String> {
        if command.starts_with("set_volume") {
            let params_json = command.splitn(2, ':').nth(1).unwrap_or("{}");
            let params: Value = serde_json::from_str(params_json).unwrap_or(json!({}));
            
            let mut ps_cmd = String::new();
            
            if let Some(level) = params.get("level").and_then(Value::as_i64) {
                // Approximate level (0-100) by sending volume up/down keys
                // Character 173: Mute, 174: Volume Down, 175: Volume Up
                ps_cmd = format!(
                    "$w = New-Object -ComObject WScript.Shell; for($i=0; $i -lt 50; $i++) {{ $w.SendKeys([char]174) }}; for($i=0; $i -lt {}; $i+=2) {{ $w.SendKeys([char]175) }}",
                    level
                );
            } else if let Some(action) = params.get("action").and_then(Value::as_str) {
                match action {
                    "increase" => ps_cmd = "(New-Object -ComObject WScript.Shell).SendKeys([char]175)".to_string(),
                    "decrease" => ps_cmd = "(New-Object -ComObject WScript.Shell).SendKeys([char]174)".to_string(),
                    "mute" | "unmute" => ps_cmd = "(New-Object -ComObject WScript.Shell).SendKeys([char]173)".to_string(),
                    _ => {}
                }
            }

            if !ps_cmd.is_empty() {
                std::process::Command::new("powershell")
                    .args(["-Command", &ps_cmd])
                    .spawn()
                    .map_err(|e| format!("Failed to execute volume command: {}", e))?;
            }
            
            Ok(json!({ "status": "success", "message": "Volume adjusted" }))

        } else if command == "get_volume" {
            let client = reqwest::Client::new();
            let res = client.get("http://127.0.0.1:8000/system/volume")
                .send()
                .await
                .map_err(|e| format!("Failed to reach Brain for volume: {}", e))?;
                
            let data: Value = res.json().await.map_err(|e| format!("Invalid JSON from Brain: {}", e))?;
            
            if data["status"] == "success" {
                let level = data["level"].as_i64().unwrap_or(0);
                Ok(json!({ "status": "success", "level": level, "message": format!("Your current system volume is {}%.", level) }))
            } else {
                let err_msg = data["message"].as_str().unwrap_or("Unknown error");
                Err(format!("Brain failed to read volume: {}", err_msg))
            }
            
        } else if command == "get_brightness" {
            let output = std::process::Command::new("powershell")
                .args(["-Command", "(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness).CurrentBrightness"])
                .output()
                .map_err(|e| format!("Failed to read brightness: {}", e))?;
                
            if output.status.success() {
                let level = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Ok(json!({ "status": "success", "level": level, "message": format!("Your current screen brightness is {}%.", level) }))
            } else {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                Err(format!("PowerShell execution failed: {}", error_msg))
            }

        } else if command.starts_with("set_brightness") {
            let params_json = command.splitn(2, ':').nth(1).unwrap_or("{}");
            let params: Value = serde_json::from_str(params_json).unwrap_or(json!({}));
            let level = params.get("level").and_then(Value::as_i64).unwrap_or(50);
            
            let ps_cmd = format!("(Get-WmiObject -Namespace root/WMI -Class WmiMonitorBrightnessMethods).WmiSetBrightness(1, {})", level);
            
            std::process::Command::new("powershell")
                .args(["-Command", &ps_cmd])
                .spawn()
                .map_err(|e| format!("Failed to set brightness: {}", e))?;
                
            Ok(json!({ "status": "success", "message": format!("Brightness set to {}%", level) }))

        } else if command.starts_with("system_control") {
            let params_json = command.splitn(2, ':').nth(1).unwrap_or("{}");
            let params: Value = serde_json::from_str(params_json).unwrap_or(json!({}));
            let sub_cmd = params.get("command").and_then(Value::as_str).unwrap_or("");
            
            match sub_cmd {
                "lock" => {
                    std::process::Command::new("rundll32.exe")
                        .args(["user32.dll,LockWorkStation"])
                        .spawn()
                        .map_err(|e| format!("Failed to lock screen: {}", e))?;
                },
                "shutdown" => {
                    std::process::Command::new("shutdown")
                        .args(["/s", "/t", "60"]) 
                        .spawn()
                        .map_err(|e| format!("Failed to initiate shutdown: {}", e))?;
                },
                "restart" => {
                    std::process::Command::new("shutdown")
                        .args(["/r", "/t", "60"])
                        .spawn()
                        .map_err(|e| format!("Failed to initiate restart: {}", e))?;
                },
                "sleep" => {
                    std::process::Command::new("rundll32.exe")
                        .args(["powrprof.dll,SetSuspendState", "0,1,0"])
                        .spawn()
                        .map_err(|e| format!("Failed to enter sleep: {}", e))?;
                },
                _ => return Err(format!("Unknown system command: {}", sub_cmd)),
            }
            
            Ok(json!({ "status": "success", "message": format!("Executed system command: {}", sub_cmd) }))
        } else {
            Err("Unknown system plugin command".to_string())
        }
    }
}

fn validate_config(config_json: Option<&str>) -> Result<(), String> {
    if let Some(config_json) = config_json {
        let trimmed = config_json.trim();
        if !trimmed.is_empty() {
            serde_json::from_str::<Value>(trimmed)
                .map_err(|e| format!("Invalid plugin config JSON: {}", e))?;
        }
    }

    Ok(())
}

fn read_config(config_json: Option<&str>) -> Result<Value, String> {
    match config_json {
        Some(config_json) if !config_json.trim().is_empty() => {
            serde_json::from_str(config_json)
                .map_err(|e| format!("Invalid plugin config JSON: {}", e))
        }
        _ => Ok(json!({})),
    }
}