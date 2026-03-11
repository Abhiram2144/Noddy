use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

// ============================================================================
// SETTINGS SCHEMA
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub first_run: bool,
    pub onboarding_completed: bool,
    pub theme: String, // "dark" | "light" | "system"
    pub suggestions_enabled: bool,
    pub auto_start: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            first_run: true,
            onboarding_completed: false,
            theme: "dark".to_string(),
            suggestions_enabled: true,
            auto_start: false,
        }
    }
}

// ============================================================================
// STATE CONTAINER
// ============================================================================

pub struct SettingsState {
    pub settings: Mutex<AppSettings>,
    pub path: PathBuf,
}

impl SettingsState {
    /// Load settings from `<config_dir>/settings.json`, falling back to defaults.
    pub fn load(config_dir: &PathBuf) -> Self {
        let path = config_dir.join("settings.json");
        let settings = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str::<AppSettings>(&content)
                    .unwrap_or_else(|e| {
                        eprintln!("⚠️  Failed to parse settings.json: {}. Using defaults.", e);
                        AppSettings::default()
                    }),
                Err(e) => {
                    eprintln!("⚠️  Failed to read settings.json: {}. Using defaults.", e);
                    AppSettings::default()
                }
            }
        } else {
            AppSettings::default()
        };

        SettingsState {
            settings: Mutex::new(settings),
            path,
        }
    }

    /// Persist current settings to disk.
    pub fn save(&self) -> Result<(), String> {
        let settings = self.settings.lock().map_err(|e| e.to_string())?;
        let content =
            serde_json::to_string_pretty(&*settings).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, content).map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub fn get_settings(state: tauri::State<SettingsState>) -> Result<AppSettings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn update_settings(
    state: tauri::State<SettingsState>,
    new_settings: AppSettings,
) -> Result<AppSettings, String> {
    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        *settings = new_settings;
    }
    state.save()?;
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn get_app_data_path(app: tauri::AppHandle) -> Result<String, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}
