use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri_plugin_opener::OpenerExt;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
use winreg::RegKey;
#[cfg(target_os = "windows")]
use winreg::enums::{HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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

#[derive(Deserialize)]
#[allow(dead_code)]
struct ActionRequest {
    action: String,
    value: String,
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

fn log_action(action: &str, value: &str, success: bool) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!(
        "[{}] action={} value={} success={}",
        timestamp, action, value, success
    );
}

// Main command dispatcher
#[tauri::command]
fn execute_action(
    action: String,
    value: String,
    app_handle: tauri::AppHandle,
    registry: tauri::State<AppRegistry>,
) -> Result<ActionResponse, String> {
    let action_name = action.as_str();
    let value_ref = value.as_str();

    let response = match action_name {
        "list_apps" => {
            let names = registry.display_names.clone();
            ActionResponse {
                success: true,
                message: "Installed apps listed".to_string(),
                requires_confirmation: false,
                fallback_action: None,
                fallback_value: None,
                data: Some(names),
            }
        }
        "open_app" => match open_app_internal(value_ref, &registry) {
            Ok(()) => ActionResponse {
                success: true,
                message: "Application opened".to_string(),
                requires_confirmation: false,
                fallback_action: None,
                fallback_value: None,
                data: None,
            },
            Err(err) => {
                let fallback_url = build_fallback_url(value_ref);
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
        },
        "open_url" => {
            if !is_valid_url(value_ref) {
                ActionResponse {
                    success: false,
                    message: "Invalid URL".to_string(),
                    requires_confirmation: false,
                    fallback_action: None,
                    fallback_value: None,
                    data: None,
                }
            } else {
                match open_url_internal(value_ref, &app_handle) {
                    Ok(()) => ActionResponse {
                        success: true,
                        message: "URL opened".to_string(),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    },
                    Err(err) => ActionResponse {
                        success: false,
                        message: format!("Failed to open URL: {}", err),
                        requires_confirmation: false,
                        fallback_action: None,
                        fallback_value: None,
                        data: None,
                    },
                }
            }
        }
        "kill_process" => match kill_process_internal(value.clone()) {
            Ok(()) => ActionResponse {
                success: true,
                message: "Process terminated".to_string(),
                requires_confirmation: false,
                fallback_action: None,
                fallback_value: None,
                data: None,
            },
            Err(err) => ActionResponse {
                success: false,
                message: format!("Failed to terminate process: {}", err),
                requires_confirmation: false,
                fallback_action: None,
                fallback_value: None,
                data: None,
            },
        },
        _ => ActionResponse {
            success: false,
            message: format!("Unknown action: {}", action_name),
            requires_confirmation: false,
            fallback_action: None,
            fallback_value: None,
            data: None,
        },
    };

    log_action(action_name, value_ref, response.success);
    Ok(response)
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(registry)
        .invoke_handler(tauri::generate_handler![greet, execute_action])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
