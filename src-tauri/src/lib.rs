use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::path::PathBuf;
use tauri::{AppHandle, State, Emitter, Manager};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use clipboard::{ClipboardProvider, ClipboardContext};
use std::fs;
use regex::Regex;

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

    fn get_data_file_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
        let app_data_dir = app_handle.path().app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;
        
        // ディレクトリが存在しない場合は作成
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir)
                .map_err(|e| format!("Failed to create app data directory: {}", e))?;
        }
        
        Ok(app_data_dir.join("clipboard_data.json"))
    }

    pub fn load_from_file(&self, app_handle: &AppHandle) -> Result<(), String> {
        let file_path = Self::get_data_file_path(app_handle)?;
        
        if !file_path.exists() {
            log::info!("データファイルが存在しないため、デフォルト設定を使用します");
            return Ok(());
        }

        let file_content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read data file: {}", e))?;

        let loaded_data: AppData = serde_json::from_str(&file_content)
            .map_err(|e| format!("Failed to parse JSON data: {}", e))?;

        match self.app_data.lock() {
            Ok(mut data) => {
                *data = loaded_data;
                log::info!("データファイルから読み込み完了: {:?}", file_path);
                Ok(())
            }
            Err(_) => Err("Failed to lock app data for loading".to_string()),
        }
    }

    pub fn save_to_file(&self, app_handle: &AppHandle) -> Result<(), String> {
        let file_path = Self::get_data_file_path(app_handle)?;

        let data_to_save = match self.app_data.lock() {
            Ok(data) => data.clone(),
            Err(_) => return Err("Failed to lock app data for saving".to_string()),
        };

        let json_content = serde_json::to_string_pretty(&data_to_save)
            .map_err(|e| format!("Failed to serialize data: {}", e))?;

        fs::write(&file_path, json_content)
            .map_err(|e| format!("Failed to write data file: {}", e))?;

        log::info!("データファイルに保存完了: {:?}", file_path);
        Ok(())
    }

    fn extract_ip_addresses(&self, text: &str) -> Vec<String> {
        // IPv4アドレスのパターン: xxx.xxx.xxx.xxx
        let ip_regex = Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap();
        
        let mut ips = Vec::new();
        for cap in ip_regex.find_iter(text) {
            let ip = cap.as_str().to_string();
            
            // 有効なIPアドレスかチェック（各オクテットが0-255の範囲内）
            if self.is_valid_ip(&ip) {
                ips.push(ip);
            }
        }
        
        ips
    }

    fn is_valid_ip(&self, ip: &str) -> bool {
        let parts: Vec<&str> = ip.split('.').collect();
        if parts.len() != 4 {
            return false;
        }
        
        for part in parts {
            if let Ok(_num) = part.parse::<u8>() {
                // 0-255の範囲内であることを確認（u8なので自動的に範囲内）
                continue;
            } else {
                return false;
            }
        }
        
        true
    }

    fn add_ip_to_history(&self, ip: String) -> Result<(), String> {
        match self.app_data.lock() {
            Ok(mut data) => {
                // 既存のIPがあるかチェック
                if let Some(existing_ip) = data.recent_ips.iter_mut().find(|item| item.ip == ip) {
                    // 既存の場合はカウントを増やして最新のタイムスタンプに更新
                    existing_ip.count += 1;
                    existing_ip.timestamp = Utc::now();
                    log::info!("IP履歴を更新: {} (count: {})", ip, existing_ip.count);
                } else {
                    // 新しいIPの場合は追加
                    let ip_item = IpHistoryItem {
                        ip: ip.clone(),
                        timestamp: Utc::now(),
                        count: 1,
                    };
                    
                    // 設定で指定された件数制限
                    let limit = data.settings.ip_limit;
                    if data.recent_ips.len() >= limit {
                        // 最も古いものを削除（最初の要素）
                        data.recent_ips.remove(0);
                    }
                    
                    data.recent_ips.push(ip_item);
                    log::info!("新しいIPを履歴に追加: {}", ip);
                }
                
                // タイムスタンプでソート（新しい順）
                data.recent_ips.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                
                Ok(())
            }
            Err(_) => Err("Failed to access IP history".to_string()),
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

    pub fn start_auto_save(&self, app_handle: AppHandle) {
        let app_data = Arc::clone(&self.app_data);
        let app_handle_clone = app_handle.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30)); // 30秒ごとに自動保存
            
            loop {
                interval.tick().await;
                
                if let Ok(data) = app_data.lock() {
                    let data_clone = data.clone();
                    drop(data); // Mutexのロックを解放
                    
                    let file_path = match Self::get_data_file_path(&app_handle_clone) {
                        Ok(path) => path,
                        Err(e) => {
                            log::warn!("自動保存: ファイルパス取得エラー: {}", e);
                            continue;
                        }
                    };
                    
                    if let Ok(json_content) = serde_json::to_string_pretty(&data_clone) {
                        if let Err(e) = fs::write(&file_path, json_content) {
                            log::warn!("自動保存エラー: {}", e);
                        } else {
                            log::debug!("自動保存完了: {:?}", file_path);
                        }
                    }
                }
            }
        });
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
                                        
                                        // データ変更時の通知のみ（自動保存は別タスクで実行）
                                        
                                        // フロントエンドに通知
                                        let _ = app_handle.emit("clipboard-updated", &text);
                                    }
                                }
                            }
                        }
                        
                        // IP検出処理（クリップボード変更があった場合）
                        if let Ok(last) = last_content.lock() {
                            if last.as_ref() != Some(&text) && !text.trim().is_empty() {
                                // IP検出を実行
                                if let Ok(_data) = app_data.lock() {
                                    let manager_clone = ClipboardManager {
                                        app_data: Arc::clone(&app_data),
                                        last_clipboard_content: Arc::new(Mutex::new(None)),
                                        is_monitoring: Arc::new(Mutex::new(false)),
                                    };
                                    
                                    let detected_ips = manager_clone.extract_ip_addresses(&text);
                                    drop(_data);
                                    
                                    for ip in detected_ips {
                                        if let Err(e) = manager_clone.add_ip_to_history(ip.clone()) {
                                            log::warn!("IP履歴追加エラー: {}", e);
                                        } else {
                                            log::info!("IP検出・追加: {}", ip);
                                            let _ = app_handle.emit("ip-detected", &ip);
                                        }
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
    
    // データファイルから読み込み
    if let Err(e) = state.load_from_file(&app_handle) {
        log::warn!("データファイル読み込みエラー: {}", e);
    }
    
    // 自動保存を開始
    state.start_auto_save(app_handle.clone());
    
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
    app_handle: AppHandle,
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
            
            // データを自動保存
            drop(data); // Mutexのロックを解放
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
            Ok("Bookmark added successfully".to_string())
        }
        Err(_) => Err("Failed to add bookmark".to_string()),
    }
}

#[tauri::command]
fn delete_bookmark(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    bookmark_id: String,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            data.bookmarks.retain(|b| b.id != bookmark_id);
            log::info!("ブックマークを削除しました: {}", bookmark_id);
            
            // データを自動保存
            drop(data); // Mutexのロックを解放
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
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
    app_handle: AppHandle,
    settings: AppSettings,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            data.settings = settings;
            log::info!("設定を更新しました");
            
            // データを自動保存
            drop(data); // Mutexのロックを解放
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
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

#[tauri::command]
fn save_data_to_file(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    state.save_to_file(&app_handle)?;
    Ok("Data saved successfully".to_string())
}

#[tauri::command]
fn load_data_from_file(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    state.load_from_file(&app_handle)?;
    Ok("Data loaded successfully".to_string())
}

#[tauri::command]
fn add_ip_to_recent(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    ip: String,
) -> Result<String, String> {
    if !state.is_valid_ip(&ip) {
        return Err("Invalid IP address format".to_string());
    }
    
    state.add_ip_to_history(ip)?;
    
    // データを自動保存
    if let Err(e) = state.save_to_file(&app_handle) {
        log::warn!("自動保存エラー: {}", e);
    }
    
    Ok("IP added to recent history".to_string())
}

#[tauri::command]
fn remove_ip_from_recent(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    ip: String,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            data.recent_ips.retain(|item| item.ip != ip);
            log::info!("IP履歴から削除: {}", ip);
            
            // データを自動保存
            drop(data);
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
            Ok("IP removed from recent history".to_string())
        }
        Err(_) => Err("Failed to access recent IPs".to_string()),
    }
}

#[tauri::command]
fn detect_ips_in_text(
    state: State<'_, ClipboardManager>,
    text: String,
) -> Result<Vec<String>, String> {
    let ips = state.extract_ip_addresses(&text);
    Ok(ips)
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
        add_clipboard_item,
        save_data_to_file,
        load_data_from_file,
        add_ip_to_recent,
        remove_ip_from_recent,
        detect_ips_in_text
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
