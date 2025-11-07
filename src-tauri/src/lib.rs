// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{AppHandle, Manager, Runtime, Window};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn set_always_on_top(window: Window, flag: bool) -> Result<(), String> {
    window.set_always_on_top(flag).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn set_mouse_passthrough(window: Window, flag: bool) -> Result<(), String> {
    window.set_ignore_cursor_events(flag).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn change_theme_color(color: &str) -> Result<String, String> {
    Ok(color.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            set_always_on_top,
            set_mouse_passthrough,
            change_theme_color
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}