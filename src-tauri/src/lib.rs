// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{AppHandle, Manager, Runtime, Window, Emitter};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
#[macro_use]
extern crate lazy_static;
use futures_util::StreamExt;
use reqwest::Client;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SongInfo {
    title: String,
    artist: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct CurrentSongResponse {
    title: Option<String>,
    artist: Option<String>,
}

// 全局变量存储当前歌曲信息
lazy_static! {
    static ref CURRENT_SONG: Arc<Mutex<SongInfo>> = Arc::new(Mutex::new(SongInfo {
        title: "Loading...".to_string(),
        artist: "".to_string(),
    }));
}

// 初始化SSE监听任务
fn start_sse_task(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            match listen_to_sse(app_handle.clone()).await {
                Ok(_) => {
                    eprintln!("SSE连接已断开，正在尝试重新连接...");
                    // 等待一段时间后重新连接
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
                Err(e) => {
                    eprintln!("SSE连接错误: {}，正在尝试重新连接...", e);
                    // 等待一段时间后重新连接
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });
}

// 监听SSE流
async fn listen_to_sse(app_handle: AppHandle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    let response = client
        .get("https://startend.xyz/current/stream")
        .send()
        .await?;
        
    let mut stream = response.bytes_stream();
    
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let data = String::from_utf8(chunk.to_vec())?;
        
        // 解析SSE数据 (格式: data: {...}\n\n)
        if data.starts_with("data: ") {
            let json_str = data.trim_start_matches("data: ").trim();
            if let Ok(song_response) = serde_json::from_str::<CurrentSongResponse>(json_str) {
                let song_info = SongInfo {
                    title: song_response.title.unwrap_or_else(|| "Unknown Title".to_string()),
                    artist: song_response.artist.unwrap_or_else(|| "Unknown Artist".to_string()),
                };
                
                // 更新全局变量
                let current_song = CURRENT_SONG.clone();
                let mut song = current_song.lock().unwrap();
                *song = song_info.clone();
                
                // 向前端发送事件
                app_handle.emit("song-info-update", song_info)?;
            }
        }
    }
    
    Ok(())
}

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

#[tauri::command]
fn get_current_song() -> SongInfo {
    let current_song = CURRENT_SONG.clone();
    let song = current_song.lock().unwrap();
    song.clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 启动SSE监听任务
            start_sse_task(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            set_always_on_top,
            set_mouse_passthrough,
            change_theme_color,
            get_current_song
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}