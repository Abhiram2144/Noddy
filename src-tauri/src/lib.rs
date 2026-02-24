use std::process::Command;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn open_app(app: String) {
    Command::new("cmd")
        .args(["/C", "start", "", &app])
        .spawn()
        .expect("Failed to open app");
}

#[tauri::command]
fn open_url(url: String) {
    Command::new("cmd")
        .args(["/C", "start", "", &url])
        .spawn()
        .expect("Failed to open url");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, open_app, open_url])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
