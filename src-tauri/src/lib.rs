use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, State, Emitter};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use clipboard::{ClipboardProvider, ClipboardContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    pub content: String,
    pub content_type: String,
    pub timestamp: DateTime<Utc>,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkItem {
    pub id: String,
    pub name: String,
    pub content: String,
    pub content_type: String,
    pub timestamp: DateTime<Utc>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpHistoryItem {
    pub ip: String,
    pub timestamp: DateTime<Utc>,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub hotkey: String,
    pub history_limit: usize,
    pub ip_limit: usize,
    pub auto_start: bool,
    pub show_notifications: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey: "cmd+shift+v".to_string(),
            history_limit: 50,
            ip_limit: 10,
            auto_start: true,
            show_notifications: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppData {
    pub version: String,
    pub history: Vec<ClipboardItem>,
    pub bookmarks: Vec<BookmarkItem>,
    pub recent_ips: Vec<IpHistoryItem>,
    pub settings: AppSettings,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            history: Vec::new(),
            bookmarks: Vec::new(),
            recent_ips: Vec::new(),
            settings: AppSettings::default(),
        }
    }
}

pub struct ClipboardManager {
    app_data: Arc<Mutex<AppData>>,
    last_clipboard_content: Arc<Mutex<Option<String>>>,
    is_monitoring: Arc<Mutex<bool>>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self {
            app_data: Arc::new(Mutex::new(AppData::default())),
            last_clipboard_content: Arc::new(Mutex::new(None)),
            is_monitoring: Arc::new(Mutex::new(false)),
        }
    }

    pub fn add_item(&self, content: String, content_type: String) -> Result<(), String> {
        let item = ClipboardItem {
            id: Uuid::new_v4().to_string(),
            content: content.clone(),
            content_type,
            timestamp: Utc::now(),
            size: content.len(),
        };

        match self.app_data.lock() {
            Ok(mut data) => {
                // 重複チェック
                if let Some(last_item) = data.history.last() {
                    if last_item.content == content {
                        return Ok(()); // 重複なのでスキップ
                    }
                }

                // 設定で指定された件数制限
                let limit = data.settings.history_limit;
                if data.history.len() >= limit {
                    data.history.remove(0);
                }
                
                data.history.push(item);
                log::info!("クリップボード履歴に追加: {} chars", content.len());
                Ok(())
            }
            Err(_) => Err("Failed to access clipboard history".to_string()),
        }
    }

    pub fn start_monitoring(&self, app_handle: AppHandle) -> Result<(), String> {
        let mut is_monitoring = self.is_monitoring.lock().map_err(|_| "Failed to lock monitoring state")?;
        
        if *is_monitoring {
            return Ok(());
        }
        
        *is_monitoring = true;
        
        let app_data = Arc::clone(&self.app_data);
        let last_content = Arc::clone(&self.last_clipboard_content);
        let monitoring_flag = Arc::clone(&self.is_monitoring);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            
            loop {
                interval.tick().await;
                
                // 監視停止チェック
                if let Ok(is_running) = monitoring_flag.lock() {
                    if !*is_running {
                        break;
                    }
                }
                
                // クリップボード内容を取得
                if let Ok(mut ctx) = ClipboardContext::new() {
                    if let Ok(text) = ctx.get_contents() {
                        // 前回の内容と比較
                        if let Ok(mut last) = last_content.lock() {
                            if last.as_ref() != Some(&text) && !text.trim().is_empty() {
                                *last = Some(text.clone());
                                
                                // 履歴に追加
                                if let Ok(mut data) = app_data.lock() {
                                    // 重複チェック
                                    let should_add = data.history.last()
                                        .map(|item| item.content != text)
                                        .unwrap_or(true);
                                        
                                    if should_add {
                                        let item = ClipboardItem {
                                            id: Uuid::new_v4().to_string(),
                                            content: text.clone(),
                                            content_type: "text".to_string(),
                                            timestamp: Utc::now(),
                                            size: text.len(),
                                        };
                                        
                                        // 設定で指定された件数制限
                                        let limit = data.settings.history_limit;
                                        if data.history.len() >= limit {
                                            data.history.remove(0);
                                        }
                                        
                                        data.history.push(item);
                                        log::info!("クリップボード変更検出: {} chars", text.len());
                                        
                                        // フロントエンドに通知
                                        let _ = app_handle.emit("clipboard-updated", &text);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
        
        Ok(())
    }

    pub fn stop_monitoring(&self) -> Result<(), String> {
        match self.is_monitoring.lock() {
            Ok(mut is_monitoring) => {
                *is_monitoring = false;
                log::info!("クリップボード監視を停止しました");
                Ok(())
            }
            Err(_) => Err("Failed to stop monitoring".to_string()),
        }
    }
}

#[tauri::command]
async fn init_clipboard_manager(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    log::info!("Clipboard manager initializing...");
    
    // クリップボード監視を開始
    state.start_monitoring(app_handle)?;
    
    log::info!("Clipboard manager initialized and monitoring started");
    Ok("Clipboard manager started".to_string())
}

#[tauri::command]
fn get_clipboard_history(state: State<'_, ClipboardManager>) -> Result<Vec<ClipboardItem>, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.history.clone()),
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
fn get_app_data(state: State<'_, ClipboardManager>) -> Result<AppData, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.clone()),
        Err(_) => Err("Failed to access app data".to_string()),
    }
}

#[tauri::command]
fn get_bookmarks(state: State<'_, ClipboardManager>) -> Result<Vec<BookmarkItem>, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.bookmarks.clone()),
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
fn add_bookmark(
    state: State<'_, ClipboardManager>,
    name: String,
    content: String,
    content_type: String,
    tags: Vec<String>,
) -> Result<String, String> {
    let bookmark = BookmarkItem {
        id: Uuid::new_v4().to_string(),
        name,
        content,
        content_type,
        timestamp: Utc::now(),
        tags,
    };

    match state.app_data.lock() {
        Ok(mut data) => {
            data.bookmarks.push(bookmark);
            log::info!("ブックマークを追加しました");
            Ok("Bookmark added successfully".to_string())
        }
        Err(_) => Err("Failed to add bookmark".to_string()),
    }
}

#[tauri::command]
fn delete_bookmark(
    state: State<'_, ClipboardManager>,
    bookmark_id: String,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            data.bookmarks.retain(|b| b.id != bookmark_id);
            log::info!("ブックマークを削除しました: {}", bookmark_id);
            Ok("Bookmark deleted successfully".to_string())
        }
        Err(_) => Err("Failed to delete bookmark".to_string()),
    }
}

#[tauri::command]
fn get_recent_ips(state: State<'_, ClipboardManager>) -> Result<Vec<IpHistoryItem>, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.recent_ips.clone()),
        Err(_) => Err("Failed to access recent IPs".to_string()),
    }
}

#[tauri::command]
fn get_settings(state: State<'_, ClipboardManager>) -> Result<AppSettings, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.settings.clone()),
        Err(_) => Err("Failed to access settings".to_string()),
    }
}

#[tauri::command]
fn update_settings(
    state: State<'_, ClipboardManager>,
    settings: AppSettings,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            data.settings = settings;
            log::info!("設定を更新しました");
            Ok("Settings updated successfully".to_string())
        }
        Err(_) => Err("Failed to update settings".to_string()),
    }
}

#[tauri::command]
fn stop_clipboard_monitoring(state: State<'_, ClipboardManager>) -> Result<String, String> {
    state.stop_monitoring()?;
    Ok("Clipboard monitoring stopped".to_string())
}

#[tauri::command]
fn add_clipboard_item(
    state: State<'_, ClipboardManager>,
    content: String,
    content_type: String,
) -> Result<String, String> {
    state.add_item(content, content_type)?;
    Ok("Item added to clipboard history".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(ClipboardManager::new())
    .setup(|_app| {
      log::info!("App setup completed");
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
        init_clipboard_manager,
        get_clipboard_history,
        get_app_data,
        get_bookmarks,
        add_bookmark,
        delete_bookmark,
        get_recent_ips,
        get_settings,
        update_settings,
        stop_clipboard_monitoring,
        add_clipboard_item
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
