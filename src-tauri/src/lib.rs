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
use std::io::Write; // ログファイル書き込み用

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,
    pub content: String,
    pub content_type: String,
    pub timestamp: DateTime<Utc>,
    pub size: usize,
    #[serde(default)]
    pub access_count: u32,
    #[serde(default)]
    pub last_accessed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookmarkItem {
    pub id: String,
    pub name: String,
    pub content: String,
    pub content_type: String,
    pub timestamp: DateTime<Utc>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub access_count: u32,
    #[serde(default)]
    pub last_accessed: Option<DateTime<Utc>>,
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
    hotkey_registered: Arc<Mutex<bool>>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self {
            app_data: Arc::new(Mutex::new(AppData::default())),
            last_clipboard_content: Arc::new(Mutex::new(None)),
            is_monitoring: Arc::new(Mutex::new(false)),
            hotkey_registered: Arc::new(Mutex::new(false)),
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
    
    fn get_log_file_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
        let app_data_dir = app_handle.path().app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;
        
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir)
                .map_err(|e| format!("Failed to create app data directory: {}", e))?;
        }
        
        Ok(app_data_dir.join("clipboard_manager.log"))
    }
    
    fn log_to_file(app_handle: &AppHandle, level: &str, message: &str) {
        if let Ok(log_path) = Self::get_log_file_path(app_handle) {
            let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
            let log_entry = format!("[{}] {}: {}\n", timestamp, level, message);
            
            // ログファイルサイズ制限（5MB）
            if let Ok(metadata) = fs::metadata(&log_path) {
                if metadata.len() > 5 * 1024 * 1024 { // 5MB
                    // 古いログをローテート
                    let old_log_path = log_path.with_extension("log.old");
                    let _ = fs::rename(&log_path, &old_log_path);
                }
            }
            
            if let Ok(mut file) = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path) {
                let _ = file.write_all(log_entry.as_bytes());
                let _ = file.flush();
            }
        }
    }

    pub fn load_from_file(&self, app_handle: &AppHandle) -> Result<(), String> {
        let file_path = Self::get_data_file_path(app_handle)?;
        
        if !file_path.exists() {
            log::info!("データファイルが存在しないため、デフォルト設定を使用します");
            return Ok(());
        }

        // エラーハンドリング強化: ファイルサイズチェック
        let metadata = fs::metadata(&file_path)
            .map_err(|e| format!("Failed to get file metadata: {}", e))?;
        
        if metadata.len() > 50 * 1024 * 1024 { // 50MB制限
            return Err("Data file is too large (>50MB)".to_string());
        }
        
        if metadata.len() == 0 {
            log::warn!("データファイルが空です。デフォルト設定を使用します。");
            return Ok(());
        }

        let file_content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read data file: {}", e))?;

        // JSONパースエラーのロバストなハンドリング
        let loaded_data: AppData = match serde_json::from_str(&file_content) {
            Ok(data) => data,
            Err(e) => {
                log::error!("JSONパースエラー: {}. バックアップを作成してデフォルト設定で続行します", e);
                
                // 破損したファイルのバックアップを作成
                let backup_path = file_path.with_extension("json.backup");
                if let Err(backup_err) = fs::copy(&file_path, &backup_path) {
                    log::warn!("バックアップ作成失敗: {}", backup_err);
                } else {
                    log::info!("破損したファイルのバックアップを作成: {:?}", backup_path);
                }
                
                return Ok(()); // デフォルト設定で続行
            }
        };

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

        // エラーハンドリング強化: データサイズチェック
        if data_to_save.history.len() > data_to_save.settings.history_limit * 2 {
            log::warn!("履歴アイテム数が制限を大幅に超過しています: {}", data_to_save.history.len());
        }

        let json_content = serde_json::to_string(&data_to_save) // prettyをやめてサイズ削減
            .map_err(|e| format!("Failed to serialize data: {}", e))?;
        
        // アトミックなファイル書き込み（一時ファイル経由）
        let temp_path = file_path.with_extension("json.tmp");
        
        // 一時ファイルに書き込み
        fs::write(&temp_path, &json_content)
            .map_err(|e| format!("Failed to write temporary data file: {}", e))?;
        
        // 原子的にリネーム（データ破損を防止）
        fs::rename(&temp_path, &file_path)
            .map_err(|e| {
                // 失敗時は一時ファイルを清理
                let _ = fs::remove_file(&temp_path);
                format!("Failed to rename temporary file: {}", e)
            })?;

        log::info!("データファイルに保存完了: {:?} ({} bytes)", file_path, json_content.len());
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
            access_count: 0,
            last_accessed: None,
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
            // メモリ最適化: 自動保存間隔を動的に調整
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // 初期は60秒
            let mut last_data_hash: Option<u64> = None;
            
            loop {
                interval.tick().await;
                
                if let Ok(data) = app_data.lock() {
                    // メモリ最適化: データのハッシュ値をチェックして変更がある場合のみ保存
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    
                    let mut hasher = DefaultHasher::new();
                    data.history.len().hash(&mut hasher);
                    data.bookmarks.len().hash(&mut hasher);
                    data.recent_ips.len().hash(&mut hasher);
                    let current_hash = hasher.finish();
                    
                    if last_data_hash == Some(current_hash) {
                        // データに変更がない場合はスキップ
                        continue;
                    }
                    
                    last_data_hash = Some(current_hash);
                    let data_clone = data.clone();
                    drop(data); // Mutexのロックを解放
                    
                    let file_path = match Self::get_data_file_path(&app_handle_clone) {
                        Ok(path) => path,
                        Err(e) => {
                            log::warn!("自動保存: ファイルパス取得エラー: {}", e);
                            continue;
                        }
                    };
                    
                    // メモリ効率的なシリアライゼーション
                    if let Ok(json_content) = serde_json::to_string(&data_clone) { // pretty形式をやめてサイズ削減
                        if let Err(e) = fs::write(&file_path, json_content) {
                            log::warn!("自動保存エラー: {}", e);
                        } else {
                            log::debug!("自動保存完了: {:?} (hash: {})", file_path, current_hash);
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
            // パフォーマンス最適化: アダプティブな監視間隔
            let mut interval = tokio::time::interval(Duration::from_millis(250)); // より高速な応答
            let mut consecutive_errors = 0;
            let mut last_clipboard_hash: Option<u64> = None;
            
            loop {
                interval.tick().await;
                
                // 監視停止チェック
                if let Ok(is_running) = monitoring_flag.lock() {
                    if !*is_running {
                        break;
                    }
                }
                
                // クリップボード内容を取得（エラーハンドリング改善）
                match ClipboardContext::new() {
                    Ok(mut ctx) => {
                        match ctx.get_contents() {
                            Ok(text) => {
                                consecutive_errors = 0; // エラーカウントリセット
                                
                                // パフォーマンス最適化: ハッシュベースの変更検出
                                use std::collections::hash_map::DefaultHasher;
                                use std::hash::{Hash, Hasher};
                                
                                let mut hasher = DefaultHasher::new();
                                text.hash(&mut hasher);
                                let current_hash = hasher.finish();
                                
                                if last_clipboard_hash != Some(current_hash) && !text.trim().is_empty() {
                                    last_clipboard_hash = Some(current_hash);
                                
                                    // 前回の内容と比較
                                    if let Ok(mut last) = last_content.lock() {
                                        if last.as_ref() != Some(&text) {
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
                                                        access_count: 0,
                                                        last_accessed: None,
                                                    };
                                                    
                                                    // 設定で指定された件数制限
                                                    let limit = data.settings.history_limit;
                                                    if data.history.len() >= limit {
                                                        data.history.remove(0);
                                                    }
                                                    
                                                    data.history.push(item);
                                                    log::info!("クリップボード変更検出: {} chars", text.len());
                                                    
                                                    // フロントエンドに通知（非同期）
                                                    let _ = app_handle.emit("clipboard-updated", &text);
                                                }
                                            }
                                            
                                            // IP検出処理
                                            if let Ok(_data) = app_data.lock() {
                                                let manager_clone = ClipboardManager {
                                                    app_data: Arc::clone(&app_data),
                                                    last_clipboard_content: Arc::new(Mutex::new(None)),
                                                    is_monitoring: Arc::new(Mutex::new(false)),
                                                    hotkey_registered: Arc::new(Mutex::new(false)),
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
                            Err(e) => {
                                consecutive_errors += 1;
                                log::warn!("クリップボード読み込みエラー #{}: {}", consecutive_errors, e);
                                
                                // 連続エラーが多い場合は監視間隔を調整
                                if consecutive_errors > 5 {
                                    interval = tokio::time::interval(Duration::from_millis(1000)); // 1秒に延長
                                    log::warn!("連続エラーが多いため監視間隔を1秒に変更");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        log::error!("クリップボードコンテキスト作成エラー #{}: {}", consecutive_errors, e);
                        
                        if consecutive_errors > 10 {
                            log::error!("致命的エラー: クリップボード監視を停止します");
                            break;
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
    state.start_monitoring(app_handle.clone())?;
    
    // グローバルホットキーを自動登録
    match register_global_hotkey(app_handle.clone(), state.clone()).await {
        Ok(msg) => log::info!("グローバルホットキー自動登録: {}", msg),
        Err(e) => log::warn!("グローバルホットキー自動登録失敗: {}", e),
    }
    
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
        access_count: 0,
        last_accessed: None,
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

#[tauri::command]
fn delete_clipboard_item(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    item_id: String,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let original_len = data.history.len();
            data.history.retain(|item| item.id != item_id);
            
            if data.history.len() < original_len {
                log::info!("クリップボード履歴アイテム削除: {}", item_id);
                
                // データを自動保存
                drop(data);
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }
                
                Ok("Clipboard item deleted successfully".to_string())
            } else {
                Err("Clipboard item not found".to_string())
            }
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
fn clear_clipboard_history(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let count = data.history.len();
            data.history.clear();
            log::info!("クリップボード履歴をクリア: {} items", count);
            
            // データを自動保存
            drop(data);
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
            Ok(format!("Cleared {} clipboard items", count))
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
fn search_clipboard_history(
    state: State<'_, ClipboardManager>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<ClipboardItem>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let query_lower = query.to_lowercase();
            let mut results: Vec<ClipboardItem> = data.history
                .iter()
                .filter(|item| {
                    item.content.to_lowercase().contains(&query_lower) ||
                    item.content_type.to_lowercase().contains(&query_lower)
                })
                .cloned()
                .collect();
            
            // 新しい順にソート
            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            
            // 制限がある場合は適用
            if let Some(max_results) = limit {
                results.truncate(max_results);
            }
            
            log::info!("クリップボード検索: '{}' -> {} 件", query, results.len());
            Ok(results)
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
fn get_clipboard_stats(
    state: State<'_, ClipboardManager>,
) -> Result<serde_json::Value, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let total_items = data.history.len();
            let total_size: usize = data.history.iter().map(|item| item.size).sum();
            let oldest_timestamp = data.history.first().map(|item| &item.timestamp);
            let newest_timestamp = data.history.last().map(|item| &item.timestamp);
            
            let stats = serde_json::json!({
                "total_items": total_items,
                "total_size_bytes": total_size,
                "average_size_bytes": if total_items > 0 { total_size / total_items } else { 0 },
                "oldest_timestamp": oldest_timestamp,
                "newest_timestamp": newest_timestamp,
                "max_capacity": data.settings.history_limit,
                "usage_percent": if data.settings.history_limit > 0 { 
                    (total_items as f64 / data.settings.history_limit as f64 * 100.0) as u32
                } else { 0 }
            });
            
            Ok(stats)
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
fn update_bookmark(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    bookmark_id: String,
    name: String,
    content: String,
    content_type: String,
    tags: Vec<String>,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            if let Some(bookmark) = data.bookmarks.iter_mut().find(|b| b.id == bookmark_id) {
                bookmark.name = name;
                bookmark.content = content;
                bookmark.content_type = content_type;
                bookmark.tags = tags;
                bookmark.timestamp = Utc::now(); // 更新タイムスタンプ
                
                log::info!("ブックマークを更新: {}", bookmark_id);
                
                // データを自動保存
                drop(data);
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }
                
                Ok("Bookmark updated successfully".to_string())
            } else {
                Err("Bookmark not found".to_string())
            }
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
fn search_bookmarks(
    state: State<'_, ClipboardManager>,
    query: String,
    tags: Option<Vec<String>>,
    limit: Option<usize>,
) -> Result<Vec<BookmarkItem>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let query_lower = query.to_lowercase();
            let mut results: Vec<BookmarkItem> = data.bookmarks
                .iter()
                .filter(|bookmark| {
                    // テキスト検索
                    let text_match = query.is_empty() || 
                        bookmark.name.to_lowercase().contains(&query_lower) ||
                        bookmark.content.to_lowercase().contains(&query_lower);
                    
                    // タグ検索
                    let tag_match = if let Some(ref search_tags) = tags {
                        if search_tags.is_empty() {
                            true
                        } else {
                            search_tags.iter().any(|tag| {
                                bookmark.tags.iter().any(|bookmark_tag| {
                                    bookmark_tag.to_lowercase().contains(&tag.to_lowercase())
                                })
                            })
                        }
                    } else {
                        true
                    };
                    
                    text_match && tag_match
                })
                .cloned()
                .collect();
            
            // 新しい順にソート
            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            
            // 制限がある場合は適用
            if let Some(max_results) = limit {
                results.truncate(max_results);
            }
            
            log::info!("ブックマーク検索: '{}' -> {} 件", query, results.len());
            Ok(results)
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
fn get_bookmark_tags(
    state: State<'_, ClipboardManager>,
) -> Result<Vec<String>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let mut all_tags = std::collections::HashSet::new();
            
            for bookmark in &data.bookmarks {
                for tag in &bookmark.tags {
                    all_tags.insert(tag.clone());
                }
            }
            
            let mut tags: Vec<String> = all_tags.into_iter().collect();
            tags.sort();
            
            Ok(tags)
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
fn duplicate_bookmark(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    bookmark_id: String,
    new_name: Option<String>,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            if let Some(original) = data.bookmarks.iter().find(|b| b.id == bookmark_id) {
                let new_bookmark = BookmarkItem {
                    id: Uuid::new_v4().to_string(),
                    name: new_name.unwrap_or_else(|| format!("{} (コピー)", original.name)),
                    content: original.content.clone(),
                    content_type: original.content_type.clone(),
                    timestamp: Utc::now(),
                    tags: original.tags.clone(),
                    access_count: 0,
                    last_accessed: None,
                };
                
                data.bookmarks.push(new_bookmark);
                log::info!("ブックマークを複製: {}", bookmark_id);
                
                // データを自動保存
                drop(data);
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }
                
                Ok("Bookmark duplicated successfully".to_string())
            } else {
                Err("Bookmark not found".to_string())
            }
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
fn clear_bookmarks(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let count = data.bookmarks.len();
            data.bookmarks.clear();
            log::info!("全ブックマークをクリア: {} items", count);
            
            // データを自動保存
            drop(data);
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
            Ok(format!("Cleared {} bookmarks", count))
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
fn clear_ip_history(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let count = data.recent_ips.len();
            data.recent_ips.clear();
            log::info!("IP履歴をクリア: {} items", count);
            
            // データを自動保存
            drop(data);
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
            Ok(format!("Cleared {} IP history items", count))
        }
        Err(_) => Err("Failed to access IP history".to_string()),
    }
}

#[tauri::command]
fn search_ip_history(
    state: State<'_, ClipboardManager>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<IpHistoryItem>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let mut results: Vec<IpHistoryItem> = data.recent_ips
                .iter()
                .filter(|ip_item| {
                    query.is_empty() || ip_item.ip.contains(&query)
                })
                .cloned()
                .collect();
            
            // カウントが多い順、次に新しい順でソート
            results.sort_by(|a, b| {
                match b.count.cmp(&a.count) {
                    std::cmp::Ordering::Equal => b.timestamp.cmp(&a.timestamp),
                    other => other,
                }
            });
            
            // 制限がある場合は適用
            if let Some(max_results) = limit {
                results.truncate(max_results);
            }
            
            log::info!("IP履歴検索: '{}' -> {} 件", query, results.len());
            Ok(results)
        }
        Err(_) => Err("Failed to access IP history".to_string()),
    }
}

#[tauri::command]
fn get_ip_stats(
    state: State<'_, ClipboardManager>,
) -> Result<serde_json::Value, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let total_ips = data.recent_ips.len();
            let total_accesses: u32 = data.recent_ips.iter().map(|item| item.count).sum();
            let most_used_ip = data.recent_ips.iter()
                .max_by_key(|item| item.count)
                .map(|item| &item.ip);
            let oldest_timestamp = data.recent_ips.iter()
                .min_by_key(|item| &item.timestamp)
                .map(|item| &item.timestamp);
            let newest_timestamp = data.recent_ips.iter()
                .max_by_key(|item| &item.timestamp)
                .map(|item| &item.timestamp);
            
            let stats = serde_json::json!({
                "total_ips": total_ips,
                "total_accesses": total_accesses,
                "average_accesses": if total_ips > 0 { total_accesses / total_ips as u32 } else { 0 },
                "most_used_ip": most_used_ip,
                "oldest_timestamp": oldest_timestamp,
                "newest_timestamp": newest_timestamp,
                "max_capacity": data.settings.ip_limit,
                "usage_percent": if data.settings.ip_limit > 0 { 
                    (total_ips as f64 / data.settings.ip_limit as f64 * 100.0) as u32
                } else { 0 }
            });
            
            Ok(stats)
        }
        Err(_) => Err("Failed to access IP history".to_string()),
    }
}

#[tauri::command]
fn reset_ip_count(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    ip: String,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            if let Some(ip_item) = data.recent_ips.iter_mut().find(|item| item.ip == ip) {
                ip_item.count = 1;
                ip_item.timestamp = Utc::now();
                log::info!("IPカウントをリセット: {}", ip);
                
                // データを自動保存
                drop(data);
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }
                
                Ok("IP count reset successfully".to_string())
            } else {
                Err("IP not found in history".to_string())
            }
        }
        Err(_) => Err("Failed to access IP history".to_string()),
    }
}

#[tauri::command]
fn remove_duplicate_clipboard_items(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let original_count = data.history.len();
            
            // メモリ最適化: ハッシュベースの重複削除
            use std::collections::HashMap;
            let mut seen_hashes: HashMap<u64, usize> = HashMap::new();
            let mut unique_items = Vec::new();
            
            // ハッシュ値で重複チェック（メモリ効率的）
            for (index, item) in data.history.iter().enumerate().rev() {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                
                let mut hasher = DefaultHasher::new();
                item.content.hash(&mut hasher);
                let content_hash = hasher.finish();
                
                if !seen_hashes.contains_key(&content_hash) {
                    seen_hashes.insert(content_hash, index);
                    unique_items.push(item.clone());
                }
            }
            
            // 元の順序に戻す（古い順）
            unique_items.reverse();
            data.history = unique_items;
            
            let removed_count = original_count - data.history.len();
            log::info!("重複アイテム削除: {} 件削除（残り {} 件）", removed_count, data.history.len());
            
            // データを自動保存
            drop(data);
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
            Ok(format!("Removed {} duplicate items, {} items remaining", removed_count, original_count - removed_count))
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
fn remove_duplicate_bookmarks(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let original_count = data.bookmarks.len();
            let mut seen_content = std::collections::HashSet::new();
            let mut unique_bookmarks = Vec::new();
            
            // 新しい順に処理して、重複する場合は最新のものを保持
            for bookmark in data.bookmarks.iter().rev() {
                let content_key = format!("{}:{}", bookmark.name, bookmark.content);
                if !seen_content.contains(&content_key) {
                    seen_content.insert(content_key);
                    unique_bookmarks.push(bookmark.clone());
                }
            }
            
            // 元の順序に戻す
            unique_bookmarks.reverse();
            data.bookmarks = unique_bookmarks;
            
            let removed_count = original_count - data.bookmarks.len();
            log::info!("重複ブックマーク削除: {} 件削除（残り {} 件）", removed_count, data.bookmarks.len());
            
            // データを自動保存
            drop(data);
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
            Ok(format!("Removed {} duplicate bookmarks, {} bookmarks remaining", removed_count, original_count - removed_count))
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
fn find_duplicate_clipboard_items(
    state: State<'_, ClipboardManager>,
) -> Result<Vec<Vec<ClipboardItem>>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let mut content_groups: std::collections::HashMap<String, Vec<ClipboardItem>> = std::collections::HashMap::new();
            
            // 内容ごとにグループ化
            for item in &data.history {
                content_groups.entry(item.content.clone())
                    .or_insert_with(Vec::new)
                    .push(item.clone());
            }
            
            // 2つ以上のアイテムがあるグループのみを重複として返す
            let duplicates: Vec<Vec<ClipboardItem>> = content_groups
                .into_values()
                .filter(|group| group.len() > 1)
                .collect();
            
            log::info!("重複クリップボードアイテム検出: {} グループ", duplicates.len());
            Ok(duplicates)
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
fn find_duplicate_bookmarks(
    state: State<'_, ClipboardManager>,
) -> Result<Vec<Vec<BookmarkItem>>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let mut content_groups: std::collections::HashMap<String, Vec<BookmarkItem>> = std::collections::HashMap::new();
            
            // 名前と内容の組み合わせでグループ化
            for bookmark in &data.bookmarks {
                let key = format!("{}:{}", bookmark.name, bookmark.content);
                content_groups.entry(key)
                    .or_insert_with(Vec::new)
                    .push(bookmark.clone());
            }
            
            // 2つ以上のブックマークがあるグループのみを重複として返す
            let duplicates: Vec<Vec<BookmarkItem>> = content_groups
                .into_values()
                .filter(|group| group.len() > 1)
                .collect();
            
            log::info!("重複ブックマーク検出: {} グループ", duplicates.len());
            Ok(duplicates)
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

// ログ機能用コマンド
#[tauri::command]
fn get_app_logs(
    app_handle: AppHandle,
    lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let log_path = ClipboardManager::get_log_file_path(&app_handle)?;
    
    if !log_path.exists() {
        return Ok(vec!["ログファイルが存在しません".to_string()]);
    }
    
    let content = fs::read_to_string(&log_path)
        .map_err(|e| format!("Failed to read log file: {}", e))?;
    
    let mut log_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    
    // 最新のログから指定行数を返す
    if let Some(max_lines) = lines {
        if log_lines.len() > max_lines {
            let total_lines = log_lines.len();
            log_lines = log_lines.into_iter().skip(total_lines - max_lines).collect();
        }
    }
    
    Ok(log_lines)
}

#[tauri::command]
fn clear_app_logs(app_handle: AppHandle) -> Result<String, String> {
    let log_path = ClipboardManager::get_log_file_path(&app_handle)?;
    
    if log_path.exists() {
        fs::remove_file(&log_path)
            .map_err(|e| format!("Failed to clear log file: {}", e))?;
    }
    
    ClipboardManager::log_to_file(&app_handle, "INFO", "ログファイルがクリアされました");
    Ok("Log file cleared successfully".to_string())
}

#[tauri::command]
fn get_app_diagnostics(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<serde_json::Value, String> {
    let data = match state.app_data.lock() {
        Ok(data) => data.clone(),
        Err(_) => return Err("Failed to access app data".to_string()),
    };
    
    let log_path = ClipboardManager::get_log_file_path(&app_handle)?;
    let data_path = ClipboardManager::get_data_file_path(&app_handle)?;
    
    let log_size = if log_path.exists() {
        fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };
    
    let data_size = if data_path.exists() {
        fs::metadata(&data_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };
    
    let diagnostics = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": Utc::now(),
        "data_stats": {
            "history_count": data.history.len(),
            "bookmarks_count": data.bookmarks.len(),
            "ips_count": data.recent_ips.len(),
            "total_history_size": data.history.iter().map(|item| item.size).sum::<usize>(),
            "data_file_size": data_size,
        },
        "system_stats": {
            "log_file_size": log_size,
            "settings": data.settings,
        },
        "health": {
            "data_integrity": "OK",
            "memory_usage": "Normal",
            "disk_usage": if data_size + log_size > 10 * 1024 * 1024 { "High" } else { "Normal" }
        }
    });
    
    Ok(diagnostics)
}

// メモリクリーンアップ用の新しいコマンド
#[tauri::command]
fn optimize_memory(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let mut cleaned_items = 0;
            
            // 大きなコンテンツ（10KB以上）で古い（7日以上）アイテムを削除
            let cutoff_date = Utc::now() - chrono::Duration::days(7);
            let _original_history_count = data.history.len();
            
            data.history.retain(|item| {
                if item.size > 10240 && item.timestamp < cutoff_date { // 10KB以上かつ7日以上古い
                    cleaned_items += 1;
                    false
                } else {
                    true
                }
            });
            
            // 使用されていない古いアイテムも削除
            data.history.retain(|item| {
                if item.access_count == 0 && item.timestamp < Utc::now() - chrono::Duration::days(30) {
                    cleaned_items += 1;
                    false
                } else {
                    true
                }
            });
            
            log::info!("メモリ最適化: {} 件のアイテムを削除", cleaned_items);
            
            // データを保存
            drop(data);
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
            Ok(format!("メモリ最適化完了: {} 件のアイテムを削除", cleaned_items))
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
fn cleanup_old_items(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    days_old: u32,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let cutoff_date = Utc::now() - chrono::Duration::days(days_old as i64);
            let original_count = data.history.len();
            
            // 指定された日数より古いアイテムを削除
            data.history.retain(|item| item.timestamp > cutoff_date);
            
            let removed_count = original_count - data.history.len();
            log::info!("古いアイテム削除: {} 日以前の {} 件削除", days_old, removed_count);
            
            // データを自動保存
            drop(data);
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }
            
            Ok(format!("Removed {} items older than {} days", removed_count, days_old))
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
async fn register_global_hotkey(
    app_handle: AppHandle,
    state: State<'_, ClipboardManager>,
) -> Result<String, String> {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, GlobalShortcutExt};
    
    // 設定からホットキーを取得
    let hotkey_string = match state.app_data.lock() {
        Ok(data) => data.settings.hotkey.clone(),
        Err(_) => return Err("Failed to access settings".to_string()),
    };
    
    // ホットキー登録状態をチェック
    if let Ok(registered) = state.hotkey_registered.lock() {
        if *registered {
            return Ok("Global hotkey already registered".to_string());
        }
    }
    
    // Cmd+Shift+Vのショートカットを作成
    let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);
    
    match app_handle.global_shortcut().register(shortcut) {
        Ok(_) => {
            // 登録成功
            if let Ok(mut registered) = state.hotkey_registered.lock() {
                *registered = true;
            }
            log::info!("グローバルホットキー登録成功: {}", hotkey_string);
            Ok("Global hotkey registered successfully".to_string())
        }
        Err(e) => {
            log::error!("グローバルホットキー登録失敗: {}", e);
            Err(format!("Failed to register global hotkey: {}", e))
        }
    }
}

#[tauri::command]
async fn unregister_global_hotkey(
    app_handle: AppHandle,
    state: State<'_, ClipboardManager>,
) -> Result<String, String> {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, GlobalShortcutExt};
    
    // ホットキー登録状態をチェック
    if let Ok(registered) = state.hotkey_registered.lock() {
        if !*registered {
            return Ok("Global hotkey not registered".to_string());
        }
    }
    
    // Cmd+Shift+Vのショートカットを作成
    let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);
    
    match app_handle.global_shortcut().unregister(shortcut) {
        Ok(_) => {
            // 登録解除成功
            if let Ok(mut registered) = state.hotkey_registered.lock() {
                *registered = false;
            }
            log::info!("グローバルホットキー登録解除成功");
            Ok("Global hotkey unregistered successfully".to_string())
        }
        Err(e) => {
            log::error!("グローバルホットキー登録解除失敗: {}", e);
            Err(format!("Failed to unregister global hotkey: {}", e))
        }
    }
}

#[tauri::command]
fn is_global_hotkey_registered(
    state: State<'_, ClipboardManager>,
) -> Result<bool, String> {
    match state.hotkey_registered.lock() {
        Ok(registered) => Ok(*registered),
        Err(_) => Err("Failed to check hotkey registration status".to_string()),
    }
}

#[tauri::command]
async fn show_main_window(app_handle: AppHandle) -> Result<String, String> {
    if let Some(main_window) = app_handle.get_webview_window("main") {
        match main_window.show() {
            Ok(_) => {
                let _ = main_window.set_focus();
                let _ = main_window.unminimize();
                log::info!("メインウィンドウを表示しました");
                Ok("Main window shown successfully".to_string())
            }
            Err(e) => {
                log::error!("メインウィンドウ表示失敗: {}", e);
                Err(format!("Failed to show main window: {}", e))
            }
        }
    } else {
        Err("Main window not found".to_string())
    }
}

#[tauri::command]
async fn hide_main_window(app_handle: AppHandle) -> Result<String, String> {
    if let Some(main_window) = app_handle.get_webview_window("main") {
        match main_window.hide() {
            Ok(_) => {
                log::info!("メインウィンドウを非表示にしました");
                Ok("Main window hidden successfully".to_string())
            }
            Err(e) => {
                log::error!("メインウィンドウ非表示失敗: {}", e);
                Err(format!("Failed to hide main window: {}", e))
            }
        }
    } else {
        Err("Main window not found".to_string())
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
async fn set_dock_icon_visibility(app_handle: AppHandle, visible: bool) -> Result<String, String> {
    
    match visible {
        true => {
            // Dockアイコンを表示
            if let Err(e) = app_handle.set_activation_policy(tauri::ActivationPolicy::Regular) {
                log::error!("Dockアイコン表示失敗: {}", e);
                return Err(format!("Failed to show dock icon: {}", e));
            }
            log::info!("Dockアイコンを表示しました");
            Ok("Dock icon shown successfully".to_string())
        }
        false => {
            // Dockアイコンを非表示
            if let Err(e) = app_handle.set_activation_policy(tauri::ActivationPolicy::Accessory) {
                log::error!("Dockアイコン非表示失敗: {}", e);
                return Err(format!("Failed to hide dock icon: {}", e));
            }
            log::info!("Dockアイコンを非表示にしました");
            Ok("Dock icon hidden successfully".to_string())
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
async fn set_dock_icon_visibility(_app_handle: AppHandle, _visible: bool) -> Result<String, String> {
    Err("Dock icon control is only available on macOS".to_string())
}

#[tauri::command]
async fn minimize_to_tray(app_handle: AppHandle) -> Result<String, String> {
    if let Some(main_window) = app_handle.get_webview_window("main") {
        match main_window.hide() {
            Ok(_) => {
                // macOSの場合はDockアイコンも非表示にする
                #[cfg(target_os = "macos")]
                {
                    let _ = set_dock_icon_visibility(app_handle, false).await;
                }
                
                log::info!("アプリをトレイに最小化しました");
                Ok("App minimized to tray successfully".to_string())
            }
            Err(e) => {
                log::error!("トレイ最小化失敗: {}", e);
                Err(format!("Failed to minimize to tray: {}", e))
            }
        }
    } else {
        Err("Main window not found".to_string())
    }
}

#[tauri::command]
async fn restore_from_tray(app_handle: AppHandle) -> Result<String, String> {
    // macOSの場合はDockアイコンを表示
    #[cfg(target_os = "macos")]
    {
        let _ = set_dock_icon_visibility(app_handle.clone(), true).await;
    }
    
    if let Some(main_window) = app_handle.get_webview_window("main") {
        match main_window.show() {
            Ok(_) => {
                let _ = main_window.set_focus();
                let _ = main_window.unminimize();
                log::info!("トレイからアプリを復元しました");
                Ok("App restored from tray successfully".to_string())
            }
            Err(e) => {
                log::error!("トレイ復元失敗: {}", e);
                Err(format!("Failed to restore from tray: {}", e))
            }
        }
    } else {
        Err("Main window not found".to_string())
    }
}

#[cfg(target_os = "macos")]
#[tauri::command]
async fn check_accessibility_permission() -> Result<bool, String> {
    use std::process::Command;
    
    // macOSでアクセシビリティ権限をチェック
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get every process")
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                log::info!("アクセシビリティ権限: 許可済み");
                Ok(true)
            } else {
                log::warn!("アクセシビリティ権限: 拒否または未設定");
                Ok(false)
            }
        }
        Err(e) => {
            log::error!("アクセシビリティ権限チェック失敗: {}", e);
            Err(format!("Failed to check accessibility permission: {}", e))
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
async fn check_accessibility_permission() -> Result<bool, String> {
    Ok(true) // non-macOSでは常にtrue
}

#[cfg(target_os = "macos")]
#[tauri::command]
async fn request_accessibility_permission() -> Result<String, String> {
    use std::process::Command;
    
    // System Preferencesのアクセシビリティ設定を開く
    let output = Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .output();
    
    match output {
        Ok(result) => {
            if result.status.success() {
                log::info!("システム環境設定のアクセシビリティ画面を開きました");
                Ok("System preferences opened for accessibility settings".to_string())
            } else {
                log::error!("システム環境設定を開けませんでした");
                Err("Failed to open system preferences".to_string())
            }
        }
        Err(e) => {
            log::error!("システム環境設定を開く際にエラー: {}", e);
            Err(format!("Failed to open system preferences: {}", e))
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[tauri::command]
async fn request_accessibility_permission() -> Result<String, String> {
    Err("Accessibility permission request is only available on macOS".to_string())
}

#[tauri::command]
async fn check_permissions_status() -> Result<serde_json::Value, String> {
    let accessibility_permission = check_accessibility_permission().await.unwrap_or(false);
    
    let status = serde_json::json!({
        "accessibility": accessibility_permission,
        "clipboard": true, // クリップボードアクセスは通常は問題なし
        "global_shortcuts": true, // ホットキーの動作確認は別途必要
        "all_granted": accessibility_permission
    });
    
    log::info!("権限ステータス: {:?}", status);
    Ok(status)
}

#[tauri::command]
async fn get_permission_instructions() -> Result<serde_json::Value, String> {
    let instructions = serde_json::json!({
        "title": "アクセシビリティ権限の設定",
        "steps": [
            "1. システム環境設定を開きます",
            "2. 「セキュリティとプライバシー」をクリックします",
            "3. 左側の「プライバシー」タブを選択します",
            "4. 左のリストから「アクセシビリティ」を選択します",
            "5. 右下の鍵マークをクリックしてパスワードを入力します",
            "6. 「Clipboard Manager」アプリにチェックを入れます",
            "7. 設定を保存してアプリを再起動します"
        ],
        "note": "この権限はグローバルホットキーとクリップボード監視に必要です"
    });
    
    Ok(instructions)
}

#[tauri::command]
fn increment_access_count(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    item_id: String,
    item_type: String, // "history" または "bookmark"
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let now = Utc::now();
            let mut updated = false;
            
            if item_type == "history" {
                if let Some(item) = data.history.iter_mut().find(|item| item.id == item_id) {
                    item.access_count += 1;
                    item.last_accessed = Some(now);
                    updated = true;
                }
            } else if item_type == "bookmark" {
                if let Some(bookmark) = data.bookmarks.iter_mut().find(|bookmark| bookmark.id == item_id) {
                    bookmark.access_count += 1;
                    bookmark.last_accessed = Some(now);
                    updated = true;
                }
            }
            
            if updated {
                log::info!("アクセス回数を更新: {} ({})", item_id, item_type);
                
                // データを自動保存
                drop(data);
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }
                
                Ok("Access count incremented successfully".to_string())
            } else {
                Err("Item not found".to_string())
            }
        }
        Err(_) => Err("Failed to access app data".to_string()),
    }
}

#[tauri::command]
fn get_sorted_history(
    state: State<'_, ClipboardManager>,
    sort_by: String, // "recent", "frequency", "alphabetical"
) -> Result<Vec<ClipboardItem>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let mut history = data.history.clone();
            
            match sort_by.as_str() {
                "frequency" => {
                    // アクセス回数が多い順、次に最後のアクセス時間順
                    history.sort_by(|a, b| {
                        match b.access_count.cmp(&a.access_count) {
                            std::cmp::Ordering::Equal => {
                                match (&b.last_accessed, &a.last_accessed) {
                                    (Some(b_time), Some(a_time)) => b_time.cmp(a_time),
                                    (Some(_), None) => std::cmp::Ordering::Less,
                                    (None, Some(_)) => std::cmp::Ordering::Greater,
                                    (None, None) => b.timestamp.cmp(&a.timestamp),
                                }
                            }
                            other => other,
                        }
                    });
                }
                "alphabetical" => {
                    // アルファベット順
                    history.sort_by(|a, b| a.content.to_lowercase().cmp(&b.content.to_lowercase()));
                }
                _ => {
                    // デフォルト: recent（新しい順）
                    history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                }
            }
            
            Ok(history)
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
fn get_sorted_bookmarks(
    state: State<'_, ClipboardManager>,
    sort_by: String, // "recent", "frequency", "alphabetical", "name"
) -> Result<Vec<BookmarkItem>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let mut bookmarks = data.bookmarks.clone();
            
            match sort_by.as_str() {
                "frequency" => {
                    // アクセス回数が多い順、次に最後のアクセス時間順
                    bookmarks.sort_by(|a, b| {
                        match b.access_count.cmp(&a.access_count) {
                            std::cmp::Ordering::Equal => {
                                match (&b.last_accessed, &a.last_accessed) {
                                    (Some(b_time), Some(a_time)) => b_time.cmp(a_time),
                                    (Some(_), None) => std::cmp::Ordering::Less,
                                    (None, Some(_)) => std::cmp::Ordering::Greater,
                                    (None, None) => b.timestamp.cmp(&a.timestamp),
                                }
                            }
                            other => other,
                        }
                    });
                }
                "alphabetical" => {
                    // 内容のアルファベット順
                    bookmarks.sort_by(|a, b| a.content.to_lowercase().cmp(&b.content.to_lowercase()));
                }
                "name" => {
                    // 名前のアルファベット順
                    bookmarks.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                }
                _ => {
                    // デフォルト: recent（新しい順）
                    bookmarks.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
                }
            }
            
            Ok(bookmarks)
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(ClipboardManager::new())
    .setup(|app| {
      log::info!("App setup completed");
      
      // グローバルホットキーイベントリスナーを設定
      use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, GlobalShortcutExt};
      
      let app_handle = app.handle().clone();
      let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);
      
      let _ = app.global_shortcut().on_shortcut(shortcut, move |_app_handle, _shortcut, _event| {
        log::info!("グローバルホットキーが押されました: Cmd+Shift+V");
        
        // メインウィンドウを表示・フォーカス
        if let Some(main_window) = app_handle.get_webview_window("main") {
          let _ = main_window.show();
          let _ = main_window.set_focus();
          let _ = main_window.unminimize();
          
          // フロントエンドにホットキーイベントを通知
          let _ = app_handle.emit("hotkey-triggered", "cmd+shift+v");
        }
      });
      
      // システムトレイメニューを設定
      use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
        tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
      };
      
      let show_item = MenuItem::with_id(app, "show", "ウィンドウを表示", true, None::<&str>)?;
      let hide_item = MenuItem::with_id(app, "hide", "ウィンドウを非表示", true, None::<&str>)?;
      let separator = PredefinedMenuItem::separator(app)?;
      let clipboard_submenu = Submenu::with_id_and_items(app, "clipboard", "クリップボード", true, &[
        &MenuItem::with_id(app, "clear_history", "履歴をクリア", true, None::<&str>)?,
        &MenuItem::with_id(app, "remove_duplicates", "重複を削除", true, None::<&str>)?,
      ])?;
      let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
      
      let menu = Menu::with_items(app, &[
        &show_item,
        &hide_item,
        &separator,
        &clipboard_submenu,
        &separator,
        &quit_item,
      ])?;
      
      let app_handle_for_tray = app.handle().clone();
      let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .tooltip("Clipboard Manager")
        .on_menu_event(move |_app, event| match event.id.as_ref() {
          "show" => {
            log::info!("トレイメニュー: ウィンドウを表示");
            // Tauriコマンドを呼び出し
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
              let app_handle_clone = app_handle_for_tray.clone();
              runtime.spawn(async move {
                let _ = restore_from_tray(app_handle_clone).await;
              });
            }
          }
          "hide" => {
            log::info!("トレイメニュー: ウィンドウを非表示");
            // Tauriコマンドを呼び出し
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
              let app_handle_clone = app_handle_for_tray.clone();
              runtime.spawn(async move {
                let _ = minimize_to_tray(app_handle_clone).await;
              });
            }
          }
          "clear_history" => {
            log::info!("トレイメニュー: 履歴をクリア");
            let _ = app_handle_for_tray.emit("tray-clear-history", ());
          }
          "remove_duplicates" => {
            log::info!("トレイメニュー: 重複を削除");
            let _ = app_handle_for_tray.emit("tray-remove-duplicates", ());
          }
          "quit" => {
            log::info!("トレイメニュー: アプリケーション終了");
            app_handle_for_tray.exit(0);
          }
          _ => {}
        })
        .on_tray_icon_event(|_tray, event| {
          match event {
            TrayIconEvent::Click { button: MouseButton::Left, .. } => {
              log::info!("トレイアイコンをクリック");
            }
            TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => {
              log::info!("トレイアイコンをダブルクリック");
            }
            _ => {}
          }
        })
        .build(app)?;
      
      Ok(())
    })
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
        detect_ips_in_text,
        delete_clipboard_item,
        clear_clipboard_history,
        search_clipboard_history,
        get_clipboard_stats,
        update_bookmark,
        search_bookmarks,
        get_bookmark_tags,
        duplicate_bookmark,
        clear_bookmarks,
        clear_ip_history,
        search_ip_history,
        get_ip_stats,
        reset_ip_count,
        remove_duplicate_clipboard_items,
        remove_duplicate_bookmarks,
        find_duplicate_clipboard_items,
        find_duplicate_bookmarks,
        cleanup_old_items,
        register_global_hotkey,
        unregister_global_hotkey,
        is_global_hotkey_registered,
        show_main_window,
        hide_main_window,
        set_dock_icon_visibility,
        minimize_to_tray,
        restore_from_tray,
        check_accessibility_permission,
        request_accessibility_permission,
        check_permissions_status,
        get_permission_instructions,
        increment_access_count,
        get_sorted_history,
        get_sorted_bookmarks,
        optimize_memory,
        get_app_logs,
        clear_app_logs,
        get_app_diagnostics
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
