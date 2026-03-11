use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

// ============================================================================
// PERMISSIONS SCHEMA
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPermissions {
    /// Allow Noddy to read the list of running applications.
    pub access_running_apps: bool,
    /// Allow Noddy to launch applications on your behalf.
    pub launch_apps: bool,
    /// Allow plugins to run and access their configured integrations.
    pub plugin_access: bool,
    /// Allow Noddy to make network requests (web search, AI calls).
    pub network_access: bool,
    /// Allow Noddy to generate proactive background suggestions.
    pub background_suggestions: bool,
}

impl Default for UserPermissions {
    fn default() -> Self {
        UserPermissions {
            access_running_apps: false,
            launch_apps: false,
            plugin_access: true,
            network_access: true,
            background_suggestions: true,
        }
    }
}

// ============================================================================
// STATE CONTAINER
// ============================================================================

pub struct PermissionsState {
    pub permissions: Mutex<UserPermissions>,
    pub path: PathBuf,
}

impl PermissionsState {
    /// Load permissions from `<config_dir>/permissions.json`, falling back to defaults.
    pub fn load(config_dir: &PathBuf) -> Self {
        let path = config_dir.join("permissions.json");
        let permissions = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str::<UserPermissions>(&content)
                    .unwrap_or_else(|e| {
                        eprintln!("⚠️  Failed to parse permissions.json: {}. Using defaults.", e);
                        UserPermissions::default()
                    }),
                Err(e) => {
                    eprintln!(
                        "⚠️  Failed to read permissions.json: {}. Using defaults.",
                        e
                    );
                    UserPermissions::default()
                }
            }
        } else {
            UserPermissions::default()
        };

        PermissionsState {
            permissions: Mutex::new(permissions),
            path,
        }
    }

    /// Persist current permissions to disk.
    pub fn save(&self) -> Result<(), String> {
        let permissions = self.permissions.lock().map_err(|e| e.to_string())?;
        let content =
            serde_json::to_string_pretty(&*permissions).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_user_permissions(
    state: tauri::State<PermissionsState>,
) -> Result<UserPermissions, String> {
    let permissions = state.permissions.lock().map_err(|e| e.to_string())?;
    Ok(permissions.clone())
}

#[tauri::command]
pub fn update_user_permission(
    state: tauri::State<PermissionsState>,
    permission: String,
    value: bool,
) -> Result<UserPermissions, String> {
    {
        let mut permissions = state.permissions.lock().map_err(|e| e.to_string())?;
        match permission.as_str() {
            "access_running_apps" => permissions.access_running_apps = value,
            "launch_apps" => permissions.launch_apps = value,
            "plugin_access" => permissions.plugin_access = value,
            "network_access" => permissions.network_access = value,
            "background_suggestions" => permissions.background_suggestions = value,
            _ => return Err(format!("Unknown permission key: {}", permission)),
        }
    }
    state.save()?;
    let permissions = state.permissions.lock().map_err(|e| e.to_string())?;
    Ok(permissions.clone())
}
