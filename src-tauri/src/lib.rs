use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri_plugin_opener::OpenerExt;
use tauri::{Manager, Emitter};
use rusqlite::Connection;
use std::sync::Mutex;

// Database module for migrations and schema management
mod database;

// Service modules for CRUD operations
mod memory_store;
mod reminder_store;
mod history_store;
mod memory_graph_repository;
mod memory_intelligence_service;
mod auth_service;

// Command History architecture (repository + service layers)
mod command_history_repository;
mod command_history_service;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::enums::{HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER};

// ============================================================================
// Typed Intent Domain Model
// ============================================================================
// Represents structured user intents deserialized from Python Brain Layer.
// Removes stringly-typed action matching in favor of type-safe enum dispatch.
// ============================================================================

/// Strongly-typed Intent enum matching Python domain model.
/// Deserializes from JSON with "name" field as discriminator.
#[derive(Deserialize, Debug)]
#[serde(tag = "name", content = "payload")]
enum Intent {
    #[serde(rename = "remember")]
    Remember { content: String },
    
    #[serde(rename = "recall_memory")]
    RecallMemory,
    
    #[serde(rename = "search_memory")]
    SearchMemory { keyword: String },
    
    #[serde(rename = "set_reminder")]
    SetReminder { content: String, trigger_at: i64 },
    
    #[serde(rename = "search_web")]
    SearchWeb { url: String },
    
    #[serde(rename = "open_app")]
    OpenApp { target: String },
    
    #[serde(rename = "open_url")]
    OpenUrl { url: String },
    
    #[serde(rename = "kill_process")]
    KillProcess { process: String },
    
    #[serde(rename = "list_apps")]
    ListApps,
    
    #[serde(rename = "unknown")]
    Unknown { text: String },
}

// ============================================================================
// INFRASTRUCTURE LAYER 1: INTERNAL EVENT BUS
// ============================================================================
// Allows decoupled components to publish and subscribe to system events.
// Future: Can be extended for telemetry, sync, notifications.
// ============================================================================

#[derive(Debug, Clone)]
enum Event {
    IntentReceived(String),                          // intent_json
    IntentExecuted { intent_name: String, duration_ms: u128 }, // intent name and execution time
    MemorySaved(String),                             // content
    ReminderScheduled(String),                       // reminder_json
    ReminderTriggered(String),                       // reminder_content
    ErrorOccurred(String),                           // error_message
}

#[derive(Clone)]
struct EventBus {
    subscribers: std::sync::Arc<std::sync::Mutex<Vec<Box<dyn Fn(&Event) + Send + Sync>>>>,
}

impl EventBus {
    fn new() -> Self {
        EventBus {
            subscribers: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
    
    fn register<F: Fn(&Event) + Send + Sync + 'static>(&self, listener: F) {
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.push(Box::new(listener));
        }
    }
    
    fn emit(&self, event: &Event) {
        if let Ok(subs) = self.subscribers.lock() {
            for listener in subs.iter() {
                listener(event);
            }
        }
    }
}

// Structured telemetry subscriber: Converts events to JSON logs
fn create_telemetry_subscriber() -> impl Fn(&Event) + Send + Sync + 'static {
    move |event: &Event| {
        let telemetry = TelemetryEvent::from_event(event);
        // Log as structured JSON
        if let Ok(json) = serde_json::to_string(&serde_json::json!({
            "event_type": telemetry.event_type,
            "timestamp": telemetry.timestamp,
            "metadata": telemetry.metadata,
        })) {
            println!("[TELEMETRY] {}", json);
        }
    }
}

// ============================================================================
// INFRASTRUCTURE LAYER 2: CAPABILITY PERMISSION LAYER
// ============================================================================
// Defines what actions are allowed (future: safe mode, plugin restrictions).
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Capability {
    OpenApp,
    KillProcess,
    WebSearch,
    MemoryRead,
    MemoryWrite,
    ReminderSchedule,
}

#[derive(Clone)]
struct PermissionManager {
    allowed: std::collections::HashSet<Capability>,
}

impl PermissionManager {
    fn default_permissions() -> Self {
        let mut allowed = std::collections::HashSet::new();
        // Default: all capabilities allowed
        allowed.insert(Capability::OpenApp);
        allowed.insert(Capability::KillProcess);
        allowed.insert(Capability::WebSearch);
        allowed.insert(Capability::MemoryRead);
        allowed.insert(Capability::MemoryWrite);
        allowed.insert(Capability::ReminderSchedule);
        
        PermissionManager { allowed }
    }
    
    fn allows(&self, capability: Capability) -> bool {
        self.allowed.contains(&capability)
    }
    
    fn check_permission(&self, capability: Capability) -> Result<(), String> {
        if self.allows(capability) {
            Ok(())
        } else {
            Err(format!("Permission denied for capability: {:?}", capability))
        }
    }
}

// ============================================================================
// INFRASTRUCTURE LAYER 3: STRUCTURED TELEMETRY
// ============================================================================
// Structured event logging for observability and debugging
// ============================================================================

struct TelemetryEvent {
    event_type: String,
    timestamp: i64,
    metadata: HashMap<String, String>,
}

impl TelemetryEvent {
    fn from_event(event: &Event) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;
        
        let (event_type, metadata) = match event {
            Event::IntentReceived(json) => {
                let mut m = HashMap::new();
                m.insert("intent_json".to_string(), json.clone());
                ("IntentReceived".to_string(), m)
            }
            Event::IntentExecuted { intent_name, duration_ms } => {
                let mut m = HashMap::new();
                m.insert("intent_name".to_string(), intent_name.clone());
                m.insert("duration_ms".to_string(), duration_ms.to_string());
                m.insert("success".to_string(), "true".to_string());
                ("IntentExecuted".to_string(), m)
            }
            Event::MemorySaved(content) => {
                let mut m = HashMap::new();
                m.insert("size_bytes".to_string(), content.len().to_string());
                ("MemorySaved".to_string(), m)
            }
            Event::ReminderScheduled(json) => {
                let mut m = HashMap::new();
                m.insert("reminder_json".to_string(), json.clone());
                ("ReminderScheduled".to_string(), m)
            }
            Event::ReminderTriggered(content) => {
                let mut m = HashMap::new();
                m.insert("content_summary".to_string(), 
                    if content.len() > 100 { 
                        format!("{}...", &content[..100]) 
                    } else { 
                        content.clone() 
                    }
                );
                ("ReminderTriggered".to_string(), m)
            }
            Event::ErrorOccurred(msg) => {
                let mut m = HashMap::new();
                m.insert("error_message".to_string(), msg.clone());
                ("ErrorOccurred".to_string(), m)
            }
        };
        
        TelemetryEvent {
            event_type,
            timestamp,
            metadata,
        }
    }
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn require_user_from_access_token(access_token: &str, auth: &AuthConfig) -> Result<String, String> {
    auth_service::verify_access_token(access_token, &auth.jwt_secret)
}

#[tauri::command]
fn signup(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    email: String,
    password: String,
) -> Result<serde_json::Value, String> {
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let result = auth_service::signup(&conn, &email, &password, &auth_config.jwt_secret)?;

    Ok(serde_json::json!({
        "user": result.user,
        "tokens": result.tokens
    }))
}

#[tauri::command]
fn login(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    email: String,
    password: String,
) -> Result<serde_json::Value, String> {
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let result = auth_service::login(&conn, &email, &password, &auth_config.jwt_secret)?;

    Ok(serde_json::json!({
        "user": result.user,
        "tokens": result.tokens
    }))
}

#[tauri::command]
fn refresh_token(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    refresh_token: String,
) -> Result<serde_json::Value, String> {
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let tokens = auth_service::refresh(&conn, &refresh_token, &auth_config.jwt_secret)?;

    Ok(serde_json::json!({ "tokens": tokens }))
}

#[tauri::command]
fn logout(
    memory_store: tauri::State<MemoryStore>,
    refresh_token: String,
) -> Result<String, String> {
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    auth_service::logout(&conn, &refresh_token)?;
    Ok("Logged out".to_string())
}

#[tauri::command]
fn get_current_user(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
) -> Result<serde_json::Value, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    auth_service::claim_orphaned_data_for_user(&conn, &user_id)?;
    let user = auth_service::get_user_by_id(&conn, &user_id)?;
    Ok(serde_json::json!({ "user": user }))
}

struct InstalledApp {
    name: String,
    display_name: String,
    path: String,
    #[allow(dead_code)]
    source: String,
}

struct AppRegistry {
    apps: HashMap<String, String>,
    display_names: Vec<String>,
}

struct MemoryStore {
    conn: Mutex<Connection>,
}

struct AuthConfig {
    jwt_secret: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ActionRequest {
    intent_json: String,  // JSON string deserializable into Intent enum
}

#[derive(Serialize)]
struct ActionResponse {
    success: bool,
    message: String,
    requires_confirmation: bool,
    fallback_action: Option<String>,
    fallback_value: Option<String>,
    data: Option<Vec<String>>,
}

#[cfg(target_os = "windows")]
fn discover_apps_from_path() -> Result<Vec<InstalledApp>, String> {
    let candidates = [
        "chrome",
        "code",
        "code.cmd",
        "notepad",
        "notepad.exe",
        "explorer",
        "explorer.exe",
        "powershell",
        "powershell.exe",
        "cmd",
        "cmd.exe",
        "python",
        "python.exe",
        "firefox",
        "firefox.exe",
        "spotify",
        "spotify.exe",
        "discord",
        "discord.exe",
        "slack",
        "slack.exe",
        "vlc",
        "vlc.exe",
    ];
    let mut apps = Vec::new();
    let mut failed_attempts = Vec::new();

    for candidate in candidates {
        let output = Command::new("where")
            .arg(candidate)
            .output()
            .map_err(|e| format!("Failed to run where for {}: {}", candidate, e))?;

        if !output.status.success() {
            failed_attempts.push(candidate);
            continue;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next().map(|line| line.trim());
        let path = match first_line {
            Some(line) if !line.is_empty() => line.to_string(),
            _ => {
                failed_attempts.push(candidate);
                continue;
            }
        };

        let normalized_name = candidate.to_lowercase().trim_end_matches(".exe").trim_end_matches(".cmd").to_string();
        apps.push(InstalledApp {
            name: normalized_name,
            display_name: candidate.to_string(),
            path,
            source: "path".to_string(),
        });
    }

    if apps.is_empty() {
        println!("⚠️  No apps discovered from PATH. Checked: {:?}", failed_attempts);
    } else {
        println!("✓ Discovered {} app(s)", apps.len());
    }

    Ok(apps)
}

#[cfg(not(target_os = "windows"))]
fn discover_apps_from_path() -> Result<Vec<InstalledApp>, String> {
    Ok(Vec::new())
}

#[cfg(target_os = "windows")]
fn clean_display_icon(icon_str: &str) -> Option<String> {
    // Handle format: "C:\Path\To\App.exe",0 or similar
    let cleaned = icon_str.trim_matches('"');
    
    // Remove trailing ,<number>
    let without_trailing = cleaned
        .split(',')
        .next()
        .unwrap_or(cleaned)
        .trim();

    // Validate it's a path-like string
    if without_trailing.ends_with(".exe") || without_trailing.ends_with(".EXE") {
        let path = std::path::Path::new(without_trailing);
        if path.exists() {
            return Some(without_trailing.to_string());
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn find_exe_in_directory(dir_path: &str, app_name_hint: &str) -> Option<String> {
    let path = std::path::Path::new(dir_path);
    if !path.is_dir() {
        return None;
    }

    // Utilities to skip
    let skip_keywords = vec![
        "uninstall", "setup", "install", "update", "helper", "tool",
        "crash", "report", "telemetry", "vcredist", "webview",
    ];

    let mut candidates = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("exe")) {
                if let Some(file_stem) = entry_path.file_stem().and_then(|s| s.to_str()) {
                    let file_stem_lower = file_stem.to_lowercase();
                    
                    // Skip utility executables
                    if skip_keywords.iter().any(|kw| file_stem_lower.contains(kw)) {
                        continue;
                    }

                    if let Some(exe_path) = entry_path.to_str() {
                        // Prioritize executables that match app name closely
                        let priority = if file_stem_lower == app_name_hint.to_lowercase() {
                            1000 // Exact match to app name
                        } else if app_name_hint.to_lowercase().contains(file_stem_lower.as_str()) {
                            500 // App name contains exe name
                        } else {
                            100 // Generic exe
                        };
                        
                        candidates.push((exe_path.to_string(), priority));
                    }
                }
            }
        }
    }

    // Sort by priority (highest first) and return best match
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    candidates.first().map(|(path, _)| path.clone())
}

#[cfg(target_os = "windows")]
fn extract_exe_from_display_icon(subkey: &winreg::RegKey) -> Option<String> {
    let display_icon: String = subkey.get_value("DisplayIcon").ok()?;
    clean_display_icon(&display_icon)
}

#[cfg(target_os = "windows")]
fn extract_exe_from_install_location(subkey: &winreg::RegKey, app_name_hint: &str) -> Option<String> {
    let install_location: String = subkey.get_value("InstallLocation").ok()?;
    if install_location.is_empty() {
        return None;
    }

    let path = std::path::Path::new(&install_location);
    
    // Check if the location itself is an exe
    if path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("exe")) {
        if path.exists() {
            return Some(install_location);
        }
    }

    // Try to find exe in the directory
    find_exe_in_directory(&install_location, app_name_hint)
}

#[cfg(target_os = "windows")]
fn discover_apps_from_single_registry_root(hive: &str, root_path: &str, seen: &mut std::collections::HashSet<String>) -> Result<Vec<InstalledApp>, String> {
    let mut apps = Vec::new();

    let reg_key = if hive == "HKLM" {
        RegKey::predef(HKEY_LOCAL_MACHINE)
    } else {
        RegKey::predef(HKEY_CURRENT_USER)
    };

    let uninstall = match reg_key.open_subkey(root_path) {
        Ok(key) => key,
        Err(_) => {
            println!("Could not open {}/{}", hive, root_path);
            return Ok(Vec::new());
        }
    };

    let mut found_count = 0;

    for entry in uninstall.enum_keys() {
        let entry_name = match entry {
            Ok(name) => name,
            Err(_) => continue,
        };

        let subkey = match uninstall.open_subkey(&entry_name) {
            Ok(key) => key,
            Err(_) => continue,
        };

        let display_name: String = match subkey.get_value("DisplayName") {
            Ok(name) => name,
            Err(_) => continue,
        };

        let display_name = display_name.trim();
        if display_name.is_empty() || display_name.len() > 120 {
            continue;
        }

        let normalized_name = display_name.to_lowercase();

        // Skip if already processed
        if seen.contains(&normalized_name) {
            continue;
        }

        // Try to extract executable
        let exe_path = extract_exe_from_display_icon(&subkey)
            .or_else(|| extract_exe_from_install_location(&subkey, display_name));

        if let Some(path) = exe_path {
            apps.push(InstalledApp {
                name: normalized_name.clone(),
                display_name: display_name.to_string(),
                path,
                source: "registry".to_string(),
            });
            seen.insert(normalized_name);
            found_count += 1;
        }
    }

    println!("  ✓ {}/{}: found {} apps", hive, root_path, found_count);
    Ok(apps)
}

#[cfg(target_os = "windows")]
fn discover_apps_from_registry() -> Result<Vec<InstalledApp>, String> {
    let mut all_apps = Vec::new();
    let mut seen = std::collections::HashSet::new();

    println!("Scanning Windows registry for installed apps...");

    // Define all registry roots to scan
    let registry_roots = vec![
        ("HKLM", "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        ("HKLM", "SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
        ("HKCU", "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall"),
    ];

    for (hive, path) in registry_roots {
        match discover_apps_from_single_registry_root(hive, path, &mut seen) {
            Ok(apps) => all_apps.extend(apps),
            Err(e) => println!("  ⚠️  Error scanning {}/{}: {}", hive, path, e),
        }
    }

    Ok(all_apps)
}

#[cfg(not(target_os = "windows"))]
fn discover_apps_from_registry() -> Result<Vec<InstalledApp>, String> {
    Ok(Vec::new())
}

#[cfg(target_os = "windows")]
fn discover_apps_from_start_menu() -> Result<Vec<InstalledApp>, String> {
    let mut apps = Vec::new();
    let mut found_count = 0;

    let roots = vec![
        std::env::var("ProgramData").unwrap_or_default() + "\\Microsoft\\Windows\\Start Menu\\Programs",
        std::env::var("AppData").unwrap_or_default() + "\\Microsoft\\Windows\\Start Menu\\Programs",
    ];

    for root_path in roots {
        scan_start_menu_apps_direct(&root_path, &mut apps, &mut found_count)?;
    }

    println!("✓ Start Menu: found {} apps", found_count);
    Ok(apps)
}

#[cfg(target_os = "windows")]
fn scan_start_menu_apps_direct(
    folder: &str,
    apps: &mut Vec<InstalledApp>,
    found_count: &mut usize,
) -> Result<(), String> {
    let path = std::path::Path::new(folder);
    if !path.is_dir() {
        return Ok(());
    }

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();

            if entry_path.is_dir() {
                // Recursively scan subdirectories
                scan_start_menu_apps_direct(
                    entry_path.to_str().unwrap_or(""),
                    apps,
                    found_count,
                )?;

                // Also check if this folder itself looks like an app (e.g., "Discord Inc")
                if let Some(folder_name) = entry_path.file_name().and_then(|s| s.to_str()) {
                    if let Some(exe) = find_primary_exe_in_folder(&entry_path) {
                        println!("Start Menu: {} -> {}", folder_name, exe);
                        apps.push(InstalledApp {
                            name: folder_name.to_lowercase(),
                            display_name: folder_name.to_string(),
                            path: exe,
                            source: "start_menu".to_string(),
                        });
                        *found_count += 1;
                    }
                }
                continue;
            }

            // Look for .lnk files and try to resolve them
            if entry_path
                .extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("lnk"))
            {
                if let Some(file_name) = entry_path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(Some(target)) = resolve_lnk_simple(entry_path.to_str().unwrap_or("")) {
                        println!("Start Menu: {} -> {}", file_name, target);
                        apps.push(InstalledApp {
                            name: file_name.to_lowercase(),
                            display_name: file_name.to_string(),
                            path: target,
                            source: "start_menu".to_string(),
                        });
                        *found_count += 1;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn find_primary_exe_in_folder(folder: &std::path::Path) -> Option<String> {
    let skip_keywords = vec![
        "uninstall", "setup", "install", "update", "helper", "tool",
        "crash", "report", "telemetry", "vcredist", "webview",
    ];

    // First try app-* subdirectories (for Squirrel/Electron apps like Discord)
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                    if dir_name.to_lowercase().starts_with("app-") {
                        if let Some(exe) = find_first_exe_in_dir(&path, &skip_keywords) {
                            return Some(exe);
                        }
                    }
                }
            }
        }
    }

    // Then try the folder itself
    find_first_exe_in_dir(folder, &skip_keywords)
}

#[cfg(target_os = "windows")]
fn find_first_exe_in_dir(folder: &std::path::Path, skip_keywords: &[&str]) -> Option<String> {
    if let Ok(entries) = std::fs::read_dir(folder) {
        let mut exes = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("exe") {
                    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let stem_lower = file_stem.to_lowercase();
                        if !skip_keywords.iter().any(|kw| stem_lower.contains(kw)) {
                            exes.push(path.to_str().unwrap_or("").to_string());
                        }
                    }
                }
            }
        }
        if !exes.is_empty() {
            return Some(exes[0].clone());
        }
    }
    None
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn resolve_lnk_simple(_lnk_path: &str) -> Result<Option<String>, String> {
    // Skip LNK resolution for now - too unreliable with PowerShell COM
    // Rely on folder-based discovery instead
    Ok(None)
}


#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn find_exe_in_app_subdir(root: &std::path::Path, exe_name: &str) -> Option<String> {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                    if dir_name.to_lowercase().starts_with("app-") {
                        let candidate = path.join(exe_name);
                        if candidate.exists() {
                            if let Some(candidate_str) = candidate.to_str() {
                                return Some(candidate_str.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
fn find_exe_in_local_app_data(app_name: &str, exe_name: &str) -> Option<String> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let base = std::path::Path::new(&local);

    let app_name_trimmed = app_name.trim();
    if app_name_trimmed.is_empty() {
        return None;
    }

    let mut candidates = Vec::new();
    candidates.push(app_name_trimmed.to_string());
    candidates.push(app_name_trimmed.replace(' ', ""));

    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let dir_name = match path.file_name().and_then(|s| s.to_str()) {
                Some(name) => name,
                None => continue,
            };

            for cand in &candidates {
                if dir_name.eq_ignore_ascii_case(cand) {
                    if let Some(found) = find_exe_in_app_subdir(&path, exe_name) {
                        return Some(found);
                    }
                }
            }
        }
    }

    None
}

#[cfg(not(target_os = "windows"))]
fn discover_apps_from_start_menu() -> Result<Vec<InstalledApp>, String> {
    Ok(Vec::new())
}

#[cfg(target_os = "windows")]
fn discover_apps_from_localappdata() -> Result<Vec<InstalledApp>, String> {
    let mut apps = Vec::new();
    let mut found_count = 0;

    let local = match std::env::var("LOCALAPPDATA") {
        Ok(path) => path,
        Err(_) => return Ok(Vec::new()),
    };

    let base = std::path::Path::new(&local);
    if !base.is_dir() {
        return Ok(Vec::new());
    }

    // List of common Electron/Squirrel apps to look for
    let common_apps = vec![
        "Discord",
        "Slack",
        "Teams",
        "Spotify",
        "WhatsApp",
        "Signal",
        "Obsidian",
    ];

    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            if let Some(dir_name) = path.file_name().and_then(|s| s.to_str()) {
                for app_name in &common_apps {
                    if dir_name.eq_ignore_ascii_case(app_name) {
                        if let Some(exe) = find_primary_exe_in_folder(&path) {
                            println!("LocalAppData: {} -> {}", dir_name, exe);
                            apps.push(InstalledApp {
                                name: dir_name.to_lowercase(),
                                display_name: dir_name.to_string(),
                                path: exe,
                                source: "localappdata".to_string(),
                            });
                            found_count += 1;
                            break;
                        }
                    }
                }
            }
        }
    }

    println!("✓ LocalAppData: found {} apps", found_count);
    Ok(apps)
}

#[cfg(not(target_os = "windows"))]
fn discover_apps_from_localappdata() -> Result<Vec<InstalledApp>, String> {
    Ok(Vec::new())
}

fn build_app_registry(mut apps: Vec<InstalledApp>) -> AppRegistry {
    // Add fallback system apps if not already discovered
    let system_fallbacks = vec![
        ("notepad", "notepad.exe"),
        ("explorer", "explorer.exe"),
        ("cmd", "cmd.exe"),
    ];

    for (name, path) in system_fallbacks {
        if !apps.iter().any(|app| app.name == name) {
            apps.push(InstalledApp {
                name: name.to_string(),
                display_name: name.to_string(),
                path: path.to_string(),
                source: "fallback".to_string(),
            });
            println!("Added fallback app: {} -> {}", name, path);
        }
    }

    // Build registry with normalized names (special chars removed, alphanumeric only)
    let mut display_names: Vec<String> = apps.iter().map(|app| app.display_name.clone()).collect();
    display_names.sort();
    display_names.dedup();

    let apps_map = apps
        .into_iter()
        .map(|app| (normalize_app_name(&app.name), app.path))
        .collect();
    AppRegistry {
        apps: apps_map,
        display_names,
    }
}

// Internal function to open an executable
fn open_app_internal(name: &str, registry: &AppRegistry) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let normalized_query = normalize_app_name(name);

        // Priority 1: Exact match
        if let Some(path) = registry.apps.get(&normalized_query) {
            println!("Opening app '{}' (exact match): {}", name, path);
            return launch_app(path).map_err(|e| {
                println!("Failed to launch {}: {}", path, e);
                e
            });
        }

        // Priority 2: Find best fuzzy match
        let mut best_match: Option<(String, String, usize)> = None;

        for (key, value) in &registry.apps {
            let mut match_quality = 0;
            let mut is_match = false;

            // Prefix match: app name starts with query (highest priority)
            if key.starts_with(&normalized_query) {
                is_match = true;
                match_quality = 2000 + key.len();
            }
            // Substring match only if query is long enough
            else if normalized_query.len() >= 4 && key.contains(&normalized_query) {
                is_match = true;
                match_quality = 1000 + key.len();
            }

            if is_match {
                if best_match.is_none() 
                    || match_quality > best_match.as_ref().unwrap().2 {
                    best_match = Some((key.clone(), value.clone(), match_quality));
                }
            }
        }

        if let Some((matched_name, path, _)) = best_match {
            println!("Opening app '{}' (fuzzy match '{}'): {}", name, matched_name, path);
            return launch_app(&path).map_err(|e| {
                println!("Failed to launch {}: {}", path, e);
                e
            });
        }

        match attempt_windows_native_resolution(name) {
            Ok(()) => {
                println!("Launched via Windows native resolution");
                return Ok(());
            }
            Err(_) => {
                println!("App '{}' not found in registry", name);
                return Err(format!("Unknown app: {}", name));
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        return Err(format!("open_app is only supported on Windows: {}", name));
    }
}

#[cfg(target_os = "windows")]
fn attempt_windows_native_resolution(app_name: &str) -> Result<(), String> {
    let sanitized = sanitize_windows_app_name(app_name)?;

    Command::new("cmd")
        .args(["/C", "start", "", sanitized.as_str()])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| format!("Windows native resolution failed: {}", e))
        .map(|_| ())
}

#[cfg(target_os = "windows")]
fn sanitize_windows_app_name(app_name: &str) -> Result<String, String> {
    let trimmed = app_name.trim();
    if trimmed.is_empty() {
        return Err("Empty app name".to_string());
    }

    if trimmed.contains('&')
        || trimmed.contains('|')
        || trimmed.contains('>')
        || trimmed.contains('<')
        || trimmed.contains(';')
        || trimmed.contains('"')
        || trimmed.contains('\'')
    {
        return Err("Invalid characters in app name".to_string());
    }

    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ') {
        return Err("Invalid characters in app name".to_string());
    }

    Ok(trimmed.to_string())
}

#[cfg(target_os = "windows")]
fn launch_app(exe_path: &str) -> Result<(), String> {
    // Validate path exists
    let path = std::path::Path::new(exe_path);
    if !path.exists() {
        return Err(format!("Executable path does not exist: {}", exe_path));
    }

    // Get parent directory for working directory
    let work_dir = path.parent().ok_or_else(|| {
        "Could not determine working directory".to_string()
    })?;

    // Spawn with working directory context
    match Command::new(exe_path)
        .current_dir(work_dir)
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
    {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("740") || error_msg.contains("elevation") {
                Err(format!("App requires admin rights: {}", exe_path))
            } else {
                Err(format!("Spawn failed: {}", e))
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn launch_app(exe_path: &str) -> Result<(), String> {
    Command::new(exe_path)
        .spawn()
        .map_err(|e| format!("Spawn failed: {}", e))
        .map(|_| ())
}

// Internal function to open a URL
fn open_url_internal(url: &str, app_handle: &tauri::AppHandle) -> Result<(), String> {
    app_handle
        .opener()
    .open_url(url.to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

// Internal function to kill a process
fn kill_process_internal(process_name: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("taskkill")
            .args(["/F", "/IM", &process_name])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("pkill")
            .args(["-f", &process_name])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn is_valid_url(url: &str) -> bool {
    let trimmed = url.trim();
    trimmed.starts_with("https://") || trimmed.starts_with("http://")
}

fn build_fallback_url(value: &str) -> String {
    let normalized = value.trim().to_lowercase();
    if normalized == "youtube" {
        "https://www.youtube.com".to_string()
    } else {
        format!("https://www.{}.com", normalized)
    }
}

fn normalize_app_name(name: &str) -> String {
    // Convert to lowercase and keep only ASCII alphanumeric characters
    // This handles special unicode characters like µ in µTorrent
    name.to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

// Memory operations
fn save_memory(memory_store: &MemoryStore, user_id: &str, content: &str) -> Result<(), String> {
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    // Use memory_store service to create memory
    let content_string = content.to_string();
    let memory_id = memory_store::create_memory(&conn, user_id, content_string, None)?;
    memory_intelligence_service::link_related_memories(&conn, user_id, &memory_id)?;
    memory_intelligence_service::calculate_memory_importance(&conn, user_id, &memory_id)?;
    
    Ok(())
}

fn search_memories(memory_store: &MemoryStore, user_id: &str, keyword: &str) -> Result<Vec<String>, String> {
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    // Use memory_store service to search
    let search_results = memory_store::search_memories(&conn, user_id, keyword.to_string(), 10)?;
    
    // Extract content from Memory structs
    let memories = search_results.iter().map(|m| m.content.clone()).collect();
    
    Ok(memories)
}

fn set_reminder(memory_store: &MemoryStore, user_id: &str, json_value: &str) -> Result<(), String> {
    // Parse JSON
    let parsed: serde_json::Value = serde_json::from_str(json_value)
        .map_err(|e| format!("Invalid JSON: {}", e))?;
    
    let content = parsed["content"]
        .as_str()
        .ok_or("Missing 'content' field")?
        .to_string();
    
    let trigger_at = parsed["trigger_at"]
        .as_i64()
        .ok_or("Missing or invalid 'trigger_at' field")?;
    
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    println!("📝 Storing reminder: '{}' at timestamp {}", content, trigger_at);
    
    // Use reminder_store service to create reminder
    let reminder_id = reminder_store::create_reminder(&conn, user_id, content.clone(), trigger_at, None)?;
    
    println!("✓ Reminder stored successfully with ID: {}", reminder_id);
    
    Ok(())
}

fn check_expired_reminders(memory_store: &MemoryStore, event_bus: &EventBus, app_handle: Option<&tauri::AppHandle>) -> Result<(), String> {
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    // Use reminder_store service to get due reminders
    let due_reminders = reminder_store::get_due_reminders_global(&conn, 0)?;
    
    // Process each due reminder
    for reminder in due_reminders {
        println!("🔔 REMINDER: {}", reminder.content);
        
        // Update reminder status to triggered
        let _ = reminder_store::update_reminder_status(
            &conn,
            &reminder.user_id,
            &reminder.id,
            reminder_store::status::TRIGGERED,
        );
        
        // Emit reminder event to frontend. Frontend shows clickable desktop notification.
        if let Some(app) = app_handle {
            let _ = app.emit("reminder_fired", serde_json::json!({
                "id": reminder.id,
                "content": reminder.content,
                "user_id": reminder.user_id
            }));
            println!("✓ Reminder event emitted to frontend");
        }
        
        // Emit ReminderTriggered event for telemetry and observability
        event_bus.emit(&Event::ReminderTriggered(reminder.content));
    }
    
    Ok(())
}

#[tauri::command]
fn delete_memory(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    memory_id: String,
) -> Result<String, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    memory_store::delete_memory(&conn, &user_id, &memory_id)?;
    Ok("Memory deleted".to_string())
}

#[tauri::command]
fn finish_reminder(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    reminder_id: String,
) -> Result<String, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    // Delete reminder using service layer
    reminder_store::delete_reminder(&conn, &user_id, &reminder_id)?;
    Ok("Reminder finished".to_string())
}

#[tauri::command]
fn snooze_reminder(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    reminder_id: String,
    snooze_minutes: i64,
) -> Result<String, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    // Snooze reminder using service layer
    reminder_store::snooze_reminder(&conn, &user_id, &reminder_id, snooze_minutes)?;
    Ok(format!("Reminder snoozed for {} minutes", snooze_minutes.max(1)))
}

fn recall_memories(memory_store: &MemoryStore, user_id: &str) -> Result<Vec<String>, String> {
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    // Use memory_store service to retrieve memories
    let retrieved_memories = memory_store::get_memories(&conn, user_id, 10, 0)?;
    
    // Extract content from Memory structs
    let memories = retrieved_memories.iter().map(|m| m.content.clone()).collect();
    
    Ok(memories)
}

// Main command dispatcher with event emission and permission enforcement
// Accepts typed Intent JSON, deserializes to enum, and dispatches safely.
#[tauri::command]
fn execute_action(
    intent_json: String,
    access_token: String,
    app_handle: tauri::AppHandle,
    registry: tauri::State<AppRegistry>,
    memory_store: tauri::State<MemoryStore>,
    event_bus: tauri::State<EventBus>,
    permissions: tauri::State<PermissionManager>,
    auth_config: tauri::State<AuthConfig>,
) -> Result<ActionResponse, String> {
    let start_time = std::time::Instant::now();
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    
    // Deserialize JSON string into typed Intent enum
    let intent: Intent = serde_json::from_str(&intent_json)
        .map_err(|e| {
            let error_msg = format!("Invalid intent JSON: {}", e);
            event_bus.emit(&Event::ErrorOccurred(error_msg.clone()));
            error_msg
        })?;
    
    // Emit IntentReceived event
    event_bus.emit(&Event::IntentReceived(intent_json.clone()));
    
    // Extract intent name for telemetry
    let intent_name = match &intent {
        Intent::ListApps => "list_apps",
        Intent::OpenApp { .. } => "open_app",
        Intent::OpenUrl { .. } => "open_url",
        Intent::SearchWeb { .. } => "search_web",
        Intent::KillProcess { .. } => "kill_process",
        Intent::Remember { .. } => "remember",
        Intent::RecallMemory => "recall_memory",
        Intent::SearchMemory { .. } => "search_memory",
        Intent::SetReminder { .. } => "set_reminder",
        Intent::Unknown { .. } => "unknown",
    };
    
    // Type-safe dispatch using enum matching (no string comparisons)
    let response = match intent {
        Intent::ListApps => {
            let names = registry.display_names.clone();
            let duration_ms = start_time.elapsed().as_millis();
            event_bus.emit(&Event::IntentExecuted {
                intent_name: "list_apps".to_string(),
                duration_ms,
            });
            ActionResponse {
                success: true,
                message: "Installed apps listed".to_string(),
                requires_confirmation: false,
                fallback_action: None,
                fallback_value: None,
                data: Some(names),
            }
        }
        
        Intent::OpenApp { target } => {
            // Check permission before executing
            if let Err(perm_err) = permissions.check_permission(Capability::OpenApp) {
                event_bus.emit(&Event::ErrorOccurred(perm_err.clone()));
                return Ok(ActionResponse {
                    success: false,
                    message: perm_err,
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                });
            }
            
            match open_app_internal(&target, &registry) {
                Ok(()) => {
                    let duration_ms = start_time.elapsed().as_millis();
                    event_bus.emit(&Event::IntentExecuted {
                        intent_name: "open_app".to_string(),
                        duration_ms,
                    });
                    ActionResponse {
                        success: true,
                        message: "Application opened".to_string(),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    }
                },
                Err(err) => {
                    event_bus.emit(&Event::ErrorOccurred(err.clone()));
                    let fallback_url = build_fallback_url(&target);
                    ActionResponse {
                        success: false,
                        message: format!(
                            "App not found. Should I open this in Chrome? ({})",
                            err
                        ),
                        requires_confirmation: true,
                        fallback_action: Some("open_url".to_string()),
                        fallback_value: Some(fallback_url),
                        data: None,
                    }
                }
            }
        },
        
        Intent::OpenUrl { url } => {
            // Check permission before executing
            if let Err(perm_err) = permissions.check_permission(Capability::WebSearch) {
                event_bus.emit(&Event::ErrorOccurred(perm_err.clone()));
                return Ok(ActionResponse {
                    success: false,
                    message: perm_err,
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                });
            }
            
            if !is_valid_url(&url) {
                event_bus.emit(&Event::ErrorOccurred("Invalid URL".to_string()));
                ActionResponse {
                    success: false,
                    message: "Invalid URL".to_string(),
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                }
            } else {
                match open_url_internal(&url, &app_handle) {
                    Ok(()) => {
                        let duration_ms = start_time.elapsed().as_millis();
                        event_bus.emit(&Event::IntentExecuted {
                            intent_name: "open_url".to_string(),
                            duration_ms,
                        });
                        ActionResponse {
                            success: true,
                            message: "URL opened".to_string(),
                            requires_confirmation: false,
                            fallback_action: None,
                            fallback_value: None,
                            data: None,
                        }
                    },
                    Err(err) => {
                        event_bus.emit(&Event::ErrorOccurred(err.clone()));
                        ActionResponse {
                            success: false,
                            message: format!("Failed to open URL: {}", err),
                            requires_confirmation: false,
                            fallback_action: None,
                            fallback_value: None,
                            data: None,
                        }
                    },
                }
            }
        }
        
        Intent::SearchWeb { url } => {
            // Check permission before executing
            if let Err(perm_err) = permissions.check_permission(Capability::WebSearch) {
                event_bus.emit(&Event::ErrorOccurred(perm_err.clone()));
                return Ok(ActionResponse {
                    success: false,
                    message: perm_err,
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                });
            }
            
            // Search web (Google) - URL is already constructed by Brain Layer
            // In future, this will be delegated to LLM instead
            if !is_valid_url(&url) {
                event_bus.emit(&Event::ErrorOccurred("Invalid search URL".to_string()));
                ActionResponse {
                    success: false,
                    message: "Invalid search URL".to_string(),
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                }
            } else {
                match open_url_internal(&url, &app_handle) {
                    Ok(()) => {
                        let duration_ms = start_time.elapsed().as_millis();
                        event_bus.emit(&Event::IntentExecuted {
                            intent_name: "search_web".to_string(),
                            duration_ms,
                        });
                        ActionResponse {
                            success: true,
                            message: "Searching Google...".to_string(),
                            requires_confirmation: false,
                            fallback_action: None,
                            fallback_value: None,
                            data: None,
                        }
                    },
                    Err(err) => {
                        event_bus.emit(&Event::ErrorOccurred(err.clone()));
                        ActionResponse {
                            success: false,
                            message: format!("Failed to open search: {}", err),
                            requires_confirmation: false,
                            fallback_action: None,
                            fallback_value: None,
                            data: None,
                        }
                    },
                }
            }
        }
        
        Intent::KillProcess { process } => {
            // Check permission before executing
            if let Err(perm_err) = permissions.check_permission(Capability::KillProcess) {
                event_bus.emit(&Event::ErrorOccurred(perm_err.clone()));
                return Ok(ActionResponse {
                    success: false,
                    message: perm_err,
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                });
            }
            
            match kill_process_internal(process) {
                Ok(()) => {
                    let duration_ms = start_time.elapsed().as_millis();
                    event_bus.emit(&Event::IntentExecuted {
                        intent_name: "kill_process".to_string(),
                        duration_ms,
                    });
                    ActionResponse {
                        success: true,
                        message: "Process terminated".to_string(),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    }
                },
                Err(err) => {
                    event_bus.emit(&Event::ErrorOccurred(err.clone()));
                    ActionResponse {
                        success: false,
                        message: format!("Failed to terminate process: {}", err),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    }
                },
            }
        },
        
        Intent::Remember { content } => {
            // Check permission before executing
            if let Err(perm_err) = permissions.check_permission(Capability::MemoryWrite) {
                event_bus.emit(&Event::ErrorOccurred(perm_err.clone()));
                return Ok(ActionResponse {
                    success: false,
                    message: perm_err,
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                });
            }
            
            match save_memory(&memory_store, &user_id, &content) {
                Ok(()) => {
                    let duration_ms = start_time.elapsed().as_millis();
                    event_bus.emit(&Event::MemorySaved(content));
                    let _ = app_handle.emit("memory_saved", serde_json::json!({
                        "user_id": user_id,
                        "intent": "remember"
                    }));
                    event_bus.emit(&Event::IntentExecuted {
                        intent_name: "remember".to_string(),
                        duration_ms,
                    });
                    ActionResponse {
                        success: true,
                        message: "Memory saved.".to_string(),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    }
                },
                Err(err) => {
                    event_bus.emit(&Event::ErrorOccurred(err.clone()));
                    ActionResponse {
                        success: false,
                        message: format!("Failed to save memory: {}", err),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    }
                },
            }
        },
        
        Intent::RecallMemory => {
            // Check permission before executing
            if let Err(perm_err) = permissions.check_permission(Capability::MemoryRead) {
                event_bus.emit(&Event::ErrorOccurred(perm_err.clone()));
                return Ok(ActionResponse {
                    success: false,
                    message: perm_err,
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                });
            }
            
            match recall_memories(&memory_store, &user_id) {
                Ok(memories) => {
                    let duration_ms = start_time.elapsed().as_millis();
                    event_bus.emit(&Event::IntentExecuted {
                        intent_name: "recall_memory".to_string(),
                        duration_ms,
                    });
                    ActionResponse {
                        success: true,
                        message: "Here is what I remember.".to_string(),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: Some(memories),
                    }
                },
                Err(err) => {
                    event_bus.emit(&Event::ErrorOccurred(err.clone()));
                    ActionResponse {
                        success: false,
                        message: format!("Failed to recall memories: {}", err),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    }
                },
            }
        },
        
        Intent::SearchMemory { keyword } => {
            // Check permission before executing
            if let Err(perm_err) = permissions.check_permission(Capability::MemoryRead) {
                event_bus.emit(&Event::ErrorOccurred(perm_err.clone()));
                return Ok(ActionResponse {
                    success: false,
                    message: perm_err,
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                });
            }
            
            match search_memories(&memory_store, &user_id, &keyword) {
                Ok(memories) => {
                    let duration_ms = start_time.elapsed().as_millis();
                    event_bus.emit(&Event::IntentExecuted {
                        intent_name: "search_memory".to_string(),
                        duration_ms,
                    });
                    ActionResponse {
                        success: true,
                        message: format!("Found {} matching memories.", memories.len()),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: Some(memories),
                    }
                },
                Err(err) => {
                    event_bus.emit(&Event::ErrorOccurred(err.clone()));
                    ActionResponse {
                        success: false,
                        message: format!("Failed to search memories: {}", err),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    }
                },
            }
        },
        
        Intent::SetReminder { content, trigger_at } => {
            // Check permission before executing
            if let Err(perm_err) = permissions.check_permission(Capability::ReminderSchedule) {
                event_bus.emit(&Event::ErrorOccurred(perm_err.clone()));
                return Ok(ActionResponse {
                    success: false,
                    message: perm_err,
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                });
            }
            
            let reminder_json = serde_json::json!({
                "content": content,
                "trigger_at": trigger_at
            }).to_string();
            
            match set_reminder(&memory_store, &user_id, &reminder_json) {
                Ok(()) => {
                    let duration_ms = start_time.elapsed().as_millis();
                    event_bus.emit(&Event::ReminderScheduled(content));
                    event_bus.emit(&Event::IntentExecuted {
                        intent_name: "set_reminder".to_string(),
                        duration_ms,
                    });
                    ActionResponse {
                        success: true,
                        message: "Reminder set successfully.".to_string(),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    }
                },
                Err(err) => {
                    event_bus.emit(&Event::ErrorOccurred(err.clone()));
                    ActionResponse {
                        success: false,
                        message: format!("Failed to set reminder: {}", err),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    }
                },
            }
        },
        
        Intent::Unknown { text } => {
            let error_msg = format!("Unknown action: {}", text);
            event_bus.emit(&Event::ErrorOccurred(error_msg.clone()));
            ActionResponse {
                success: false,
                message: error_msg,
                requires_confirmation: false,
                fallback_action: None,
                fallback_value: None,
                data: None,
            }
        },
    };

    // Record command execution via duration and response outcome.
    let duration_ms = start_time.elapsed().as_millis();

    // Record command execution via service → repository → DB.
    // Uses if-let to avoid failing the response when history write fails.
    if let Ok(conn) = memory_store.conn.lock() {
        let _ = command_history_service::record_command_execution(
            &conn,
            &user_id,
            intent_name,
            &intent_json,
            response.success,
            duration_ms,
        );
    }
    
    // Log the dispatched intent for debugging
    log_intent_dispatch(&intent_json, response.success);
    Ok(response)
}

// ============================================================================
// DATA RETRIEVAL COMMANDS
// ============================================================================

#[tauri::command]
fn get_memories(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    limit: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let limit = limit.unwrap_or(10) as i32;
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    // Use memory_store service to retrieve memories
    let memories_list = memory_store::get_memories(&conn, &user_id, limit, 0)?;
    
    // Convert Memory structs to JSON response
    let json_memories = memories_list
        .iter()
        .map(|memory| {
            serde_json::json!({
                "id": memory.id,
                "content": memory.content,
                "timestamp": format_timestamp(memory.created_at),
                "importance": memory.importance,
                "source": memory.source
            })
        })
        .collect();
    
    Ok(json_memories)
}

#[tauri::command]
fn get_reminders(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    limit: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let _limit = limit.unwrap_or(10);
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    // Use reminder_store service to retrieve pending reminders
    let reminders_list = reminder_store::get_pending_reminders(&conn, &user_id)?;
    
    // Convert Reminder structs to JSON response
    let json_reminders = reminders_list
        .iter()
        .map(|reminder| {
            serde_json::json!({
                "id": reminder.id,
                "content": reminder.content,
                "trigger_at": reminder.trigger_at,
                "time": format_timestamp(reminder.trigger_at),
                "status": reminder.status,
                "source": reminder.source,
                "memory_id": reminder.memory_id
            })
        })
        .collect();
    
    Ok(json_reminders)
}

#[tauri::command]
fn get_command_history(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    limit: Option<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    let limit = limit.unwrap_or(50) as i32;
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let records = command_history_service::fetch_recent_history(&conn, &user_id, limit)?;

    Ok(records
        .iter()
        .map(|record| {
            serde_json::json!({
                "id": record.id,
                "command": record.command_text,
                "intent": record.intent_name,
                "timestamp": format_timestamp(record.created_at),
                "success": record.success,
                "duration": record.duration_ms,
                "status": record.status,
                "error_message": record.error_message
            })
        })
        .collect())
}

// Debug command to manually trigger reminder check
#[tauri::command]
fn check_reminders_now(
    memory_store: tauri::State<MemoryStore>,
    event_bus: tauri::State<EventBus>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    app: tauri::AppHandle
) -> Result<String, String> {
    let _ = require_user_from_access_token(&access_token, &auth_config)?;
    println!("🔍 Manual reminder check triggered");
    check_expired_reminders(&memory_store, &event_bus, Some(&app))?;
    Ok("Reminder check completed".to_string())
}

// Graph command to rebuild memory relationship graph
#[tauri::command]
fn rebuild_memory_graph(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
) -> Result<serde_json::Value, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    println!("🔗 Rebuilding memory relationship graph...");
        let (cleared_edges, created_edges) = memory_intelligence_service::rebuild_memory_links(&conn, &user_id)?;
    
    println!("✓ Graph rebuild complete: cleared {} existing edges, created {} new edges", 
             cleared_edges, created_edges);
    
    Ok(serde_json::json!({
        "cleared_edges": cleared_edges,
        "created_edges": created_edges,
        "status": "completed"
    }))
}

// Get related memories for a specific memory
#[tauri::command]
fn get_related_memories(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    memory_id: String,
    min_weight: Option<f64>
) -> Result<Vec<serde_json::Value>, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let min_weight = min_weight.unwrap_or(0.0);
    
    let related = memory_intelligence_service::get_related_memories(&conn, &user_id, &memory_id, min_weight)?;
    
    let json_related: Vec<serde_json::Value> = related
        .iter()
        .map(|entry| {
            serde_json::json!({
                "memory_id": entry.memory_id,
                "weight": entry.weight,
                "relationship": entry.relationship
            })
        })
        .collect();
    
    Ok(json_related)
}

// Get graph statistics
#[tauri::command]
fn get_graph_stats(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
) -> Result<serde_json::Value, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    
    let stats = memory_intelligence_service::get_graph_stats(&conn, &user_id)?;
    
    Ok(serde_json::json!({
        "total_edges": stats.total_edges,
        "shared_tag_edges": stats.keyword_edges,
        "keyword_edges": stats.keyword_edges,
        "average_weight": stats.average_weight,
        "memories_with_edges": stats.memories_with_edges,
        "clusters": stats.clusters
    }))
}

#[tauri::command]
fn track_memory_access(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    memory_id: String,
) -> Result<serde_json::Value, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let updated_importance = memory_intelligence_service::record_access_and_refresh(&conn, &user_id, &memory_id)?;

    Ok(serde_json::json!({
        "memory_id": memory_id,
        "importance": updated_importance,
        "status": "tracked"
    }))
}

#[tauri::command]
fn get_graph_data(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    limit: Option<i64>,
) -> Result<serde_json::Value, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let limit = (limit.unwrap_or(100) as i32).max(1).min(1000);

    let graph_data = memory_intelligence_service::get_graph_data(&conn, &user_id, limit)?;

    Ok(serde_json::json!({
        "nodes": graph_data.nodes,
        "edges": graph_data.edges
    }))
}

// Get memory graph for visualization (react-force-graph compatible)
#[tauri::command]
fn get_memory_graph(
    memory_store: tauri::State<MemoryStore>,
    auth_config: tauri::State<AuthConfig>,
    access_token: String,
    limit: Option<i64>
) -> Result<serde_json::Value, String> {
    let user_id = require_user_from_access_token(&access_token, &auth_config)?;
    let conn = memory_store.conn.lock().map_err(|e| format!("Lock error: {}", e))?;
    let limit = (limit.unwrap_or(100) as i32).max(1).min(1000);
    
    let graph_data = memory_intelligence_service::get_graph_data(&conn, &user_id, limit)?;
    
    Ok(serde_json::json!({
        "nodes": graph_data.nodes,
        "edges": graph_data.edges
    }))
}

/// Helper function to format Unix timestamp to readable string
fn format_timestamp(timestamp: i64) -> String {
    // Format as absolute date and time. e.g. "Mar 5, 2026 3:45 PM"
    
    // Days since epoch (Jan 1, 1970)
    let days_since_epoch = timestamp / 86400;
    let seconds_in_day = timestamp % 86400;
    
    // Simple year/month/day calculation (approximation for display)
    let mut year = 1970;
    let mut days_left = days_since_epoch;
    
    loop {
        let days_in_year = if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }
    
    let months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1;
    let mut day_of_month = days_left + 1;
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    
    for (i, &days_in_month) in months.iter().enumerate() {
        let actual_days = if i == 1 && is_leap { 29 } else { days_in_month };
        if day_of_month <= actual_days as i64 {
            month = i + 1;
            break;
        }
        day_of_month -= actual_days as i64;
    }
    
    // Time calculation
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;
    
    // Convert 24-hour to 12-hour format
    let am_pm = if hours >= 12 { "PM" } else { "AM" };
    let hour_12 = if hours % 12 == 0 { 12 } else { hours % 12 };
    
    let month_names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", 
                       "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let month_name = if month >= 1 && month <= 12 { 
        month_names[(month - 1) as usize] 
    } else { 
        "Jan" 
    };
    
    format!("{} {}, {} {}:{:02} {}", month_name, day_of_month, year, hour_12, minutes, am_pm)
}

/// Helper function to log intent dispatch (for debugging)
fn log_intent_dispatch(intent_json: &str, success: bool) {
    let status = if success { "✓" } else { "✗" };
    println!("{} Intent dispatched: {}", status, intent_json);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Discover from PATH
    let mut all_apps = discover_apps_from_path().unwrap_or_else(|e| {
        println!("⚠️  App discovery from PATH failed: {}", e);
        Vec::new()
    });

    // Discover from Start Menu shortcuts (Windows only)
    #[cfg(target_os = "windows")]
    {
        let start_menu_apps = discover_apps_from_start_menu().unwrap_or_else(|e| {
            println!("⚠️  App discovery from Start Menu failed: {}", e);
            Vec::new()
        });
        all_apps.extend(start_menu_apps);
    }

    // Discover from LocalAppData (Windows only)
    #[cfg(target_os = "windows")]
    {
        let localappdata_apps = discover_apps_from_localappdata().unwrap_or_else(|e| {
            println!("⚠️  App discovery from LocalAppData failed: {}", e);
            Vec::new()
        });
        all_apps.extend(localappdata_apps);
    }

    // Discover from Registry (Windows only)
    #[cfg(target_os = "windows")]
    {
        let registry_apps = discover_apps_from_registry().unwrap_or_else(|e| {
            println!("⚠️  App discovery from Registry failed: {}", e);
            Vec::new()
        });
        all_apps.extend(registry_apps);
    }

    // Deduplicate by name
    let mut seen = std::collections::HashSet::new();
    all_apps.retain(|app| seen.insert(app.name.clone()));
    all_apps.sort_by(|a, b| a.name.cmp(&b.name));

    println!("✓ Discovered {} total app(s)", all_apps.len());
    for app in &all_apps {
        println!("  • {} -> {}", app.display_name, app.path);
    }

    let registry = build_app_registry(all_apps);

    // Initialize infrastructure layers
    let event_bus = EventBus::new();
    let permissions = PermissionManager::default_permissions();
    
    // Register telemetry subscriber
    let telemetry_fn = create_telemetry_subscriber();
    event_bus.register(telemetry_fn);
    
    println!("✓ Phase 2 Infrastructure Activated:");
    println!("  • EventBus initialized");
    println!("  • PermissionManager initialized (default: all capabilities allowed)");
    println!("  • TelemetryEvent subscriber registered");
    println!("  • Execution lifecycle instrumentation enabled");

    // Clone event_bus for use in setup closure
    let event_bus_for_setup = event_bus.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(registry)
        .manage(event_bus)
        .manage(permissions)
        .setup(move |app| {
            // Initialize memory database in app data directory
            let app_data_dir = app.path().app_data_dir().unwrap_or_else(|_| {
                // Fallback to temp directory if app_data_dir fails
                std::env::temp_dir()
            });
            
            // Create app data directory if it doesn't exist
            std::fs::create_dir_all(&app_data_dir).ok();
            
            let db_path = app_data_dir.join("noddy.db");
            
            let conn = Connection::open(&db_path)
                .expect("Failed to open database");
            
            // Initialize database schema with migrations
            database::initialize_database(&conn)
                .expect("Failed to initialize database schema");
            
            // Verify database integrity
            database::verify_database(&conn)
                .expect("Failed to verify database integrity");
            
            println!("✓ Memory database initialized: {}", db_path.display());
            
            let memory_store = MemoryStore {
                conn: Mutex::new(conn),
            };
            
            // Clone memory store for background thread
            let db_path_clone = db_path.clone();
            let memory_store_clone = MemoryStore {
                conn: Mutex::new(Connection::open(&db_path_clone).expect("Failed to open database for reminder thread")),
            };
            
            // Spawn background reminder checker thread
            // Clone event_bus for background thread to emit ReminderTriggered events
            let event_bus_clone = event_bus_for_setup.clone();
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    
                    if let Err(e) = check_expired_reminders(&memory_store_clone, &event_bus_clone, Some(&app_handle)) {
                        eprintln!("⚠️  Reminder check failed: {}", e);
                    }
                }
            });
            
            println!("✓ Background reminder checker started (checks every 30 seconds)");
            
            let jwt_secret = std::env::var("NODDY_JWT_SECRET")
                .unwrap_or_else(|_| "noddy-local-dev-secret-change-me".to_string());

            app.manage(memory_store);
            app.manage(AuthConfig { jwt_secret });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            signup,
            login,
            refresh_token,
            logout,
            get_current_user,
            execute_action,
            get_memories,
            get_reminders,
            get_command_history,
            check_reminders_now,
            delete_memory,
            finish_reminder,
            snooze_reminder,
            rebuild_memory_graph,
            get_related_memories,
            get_graph_stats,
            track_memory_access,
            get_graph_data,
            get_memory_graph
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
