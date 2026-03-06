use crate::plugin_interface::{Plugin, PluginEvent};
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

pub fn enable_plugin(
    conn: &Connection,
    registry: &PluginRegistry,
    plugin_id: &str,
) -> Result<PluginRecord, String> {
    let plugin = registry
        .handler(plugin_id)
        .ok_or_else(|| format!("Unknown plugin: {}", plugin_id))?;

    let config_json = get_plugin_config(conn, plugin_id)?;
    plugin.initialize(config_json.as_deref())?;

    conn.execute(
        "UPDATE plugins SET enabled = 1 WHERE id = ?1",
        params![plugin_id],
    )
    .map_err(|e| format!("Failed to enable plugin: {}", e))?;

    get_plugin(conn, registry, plugin_id)
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

pub fn get_plugin(conn: &Connection, registry: &PluginRegistry, plugin_id: &str) -> Result<PluginRecord, String> {
    get_plugins(conn, registry)?
        .into_iter()
        .find(|plugin| plugin.id == plugin_id)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_id))
}

pub fn update_plugin_config(
    conn: &Connection,
    registry: &PluginRegistry,
    plugin_id: &str,
    config_json: String,
) -> Result<PluginRecord, String> {
    let plugin = registry
        .handler(plugin_id)
        .ok_or_else(|| format!("Unknown plugin: {}", plugin_id))?;

    let trimmed = config_json.trim();
    if !trimmed.is_empty() {
        serde_json::from_str::<Value>(trimmed)
            .map_err(|e| format!("Invalid plugin config JSON: {}", e))?;
        plugin.initialize(Some(trimmed))?;
    }

    conn.execute(
        "UPDATE plugins SET config_json = ?1 WHERE id = ?2",
        params![trimmed, plugin_id],
    )
    .map_err(|e| format!("Failed to update plugin config: {}", e))?;

    get_plugin(conn, registry, plugin_id)
}

pub fn dispatch_event(
    conn: &Connection,
    registry: &PluginRegistry,
    event: &PluginEvent,
) -> Result<(), String> {
    for plugin in get_active_plugins(conn, registry)? {
        if let Some(handler) = registry.handler(&plugin.id) {
            handler.handle_event(event, plugin.config_json.as_deref())?;
        }
    }

    Ok(())
}

pub fn execute_plugin_command(
    conn: &Connection,
    registry: &PluginRegistry,
    plugin_id: &str,
    command: &str,
) -> Result<Value, String> {
    let plugin = get_plugin(conn, registry, plugin_id)?;
    let handler = registry
        .handler(plugin_id)
        .ok_or_else(|| format!("Unknown plugin: {}", plugin_id))?;

    handler.execute_command(command, plugin.config_json.as_deref())
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

fn get_plugin_config(conn: &Connection, plugin_id: &str) -> Result<Option<String>, String> {
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
        &["sync reminders", "create calendar events"]
    }

    fn initialize(&self, config_json: Option<&str>) -> Result<(), String> {
        validate_config(config_json)
    }

    fn handle_event(&self, event: &PluginEvent, config_json: Option<&str>) -> Result<(), String> {
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

    fn execute_command(&self, command: &str, config_json: Option<&str>) -> Result<Value, String> {
        let config = read_config(config_json)?;
        Ok(json!({
            "plugin": self.id(),
            "command": command,
            "calendar_id": config.get("calendar_id").and_then(Value::as_str).unwrap_or("primary"),
            "status": "ready"
        }))
    }
}

struct OutlookPlugin;

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
        &["sync reminders", "create outlook tasks"]
    }

    fn initialize(&self, config_json: Option<&str>) -> Result<(), String> {
        validate_config(config_json)
    }

    fn handle_event(&self, event: &PluginEvent, config_json: Option<&str>) -> Result<(), String> {
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

    fn execute_command(&self, command: &str, config_json: Option<&str>) -> Result<Value, String> {
        let config = read_config(config_json)?;
        Ok(json!({
            "plugin": self.id(),
            "command": command,
            "task_list": config.get("task_list").and_then(Value::as_str).unwrap_or("Tasks"),
            "status": "ready"
        }))
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