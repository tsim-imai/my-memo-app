use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    id: String,
    content: String,
    content_type: String,
    timestamp: String,
    size: usize,
}

pub struct ClipboardManager {
    history: Mutex<Vec<ClipboardItem>>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self {
            history: Mutex::new(Vec::new()),
        }
    }
}

#[tauri::command]
fn init_clipboard_manager() -> Result<String, String> {
    log::info!("Clipboard manager initialized");
    Ok("Clipboard manager started".to_string())
}

#[tauri::command]
fn get_clipboard_history(state: State<ClipboardManager>) -> Result<Vec<ClipboardItem>, String> {
    match state.history.lock() {
        Ok(history) => Ok(history.clone()),
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(ClipboardManager::new())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
        init_clipboard_manager,
        get_clipboard_history
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
