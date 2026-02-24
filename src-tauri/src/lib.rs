use tauri_plugin_opener::OpenerExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn open_app(app: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    app_handle
        .opener()
        .open_url(app, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_url(url: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    app_handle
        .opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, open_app, open_url])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
