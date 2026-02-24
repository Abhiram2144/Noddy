use tauri_plugin_opener::OpenerExt;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// Internal function to open an executable
fn open_app_internal(app: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", &app])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW flag
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new(&app)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// Internal function to open a URL
fn open_url_internal(url: String, app_handle: &tauri::AppHandle) -> Result<(), String> {
    app_handle
        .opener()
        .open_url(url, None::<&str>)
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

// Main command dispatcher
#[tauri::command]
fn execute_action(
    action: String,
    value: String,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    match action.as_str() {
        "open_app" => {
            open_app_internal(value)?;
            Ok("Application opened successfully".to_string())
        }
        "open_url" => {
            open_url_internal(value, &app_handle)?;
            Ok("URL opened successfully".to_string())
        }
        "kill_process" => {
            kill_process_internal(value)?;
            Ok("Process terminated successfully".to_string())
        }
        _ => Err(format!("Unknown action: {}", action)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, execute_action])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
