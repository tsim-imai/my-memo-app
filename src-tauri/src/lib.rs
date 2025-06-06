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

pub struct ClipboardManager {
    history: Arc<Mutex<Vec<ClipboardItem>>>,
    last_clipboard_content: Arc<Mutex<Option<String>>>,
    is_monitoring: Arc<Mutex<bool>>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(Vec::new())),
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

        match self.history.lock() {
            Ok(mut history) => {
                // 重複チェック
                if let Some(last_item) = history.last() {
                    if last_item.content == content {
                        return Ok(()); // 重複なのでスキップ
                    }
                }

                // 50件制限
                if history.len() >= 50 {
                    history.remove(0);
                }
                
                history.push(item);
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
        
        let history = Arc::clone(&self.history);
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
                                if let Ok(mut hist) = history.lock() {
                                    // 重複チェック
                                    let should_add = hist.last()
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
                                        
                                        // 50件制限
                                        if hist.len() >= 50 {
                                            hist.remove(0);
                                        }
                                        
                                        hist.push(item);
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
    match state.history.lock() {
        Ok(history) => Ok(history.clone()),
        Err(_) => Err("Failed to access clipboard history".to_string()),
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
        stop_clipboard_monitoring,
        add_clipboard_item
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
