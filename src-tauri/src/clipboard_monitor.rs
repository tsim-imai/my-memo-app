use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::fs;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tauri::{AppHandle, Emitter};
use chrono::Utc;
use uuid::Uuid;
use clipboard::{ClipboardProvider, ClipboardContext};
use regex::Regex;
use serde_json;
use crate::models::{AppData, ClipboardItem};
use crate::file_manager::FileManager;

pub struct ClipboardMonitor {
    app_data: Arc<Mutex<AppData>>,
    last_clipboard_content: Arc<Mutex<Option<String>>>,
    is_monitoring: Arc<Mutex<bool>>,
}

impl ClipboardMonitor {
    pub fn new(app_data: Arc<Mutex<AppData>>) -> Self {
        Self {
            app_data,
            last_clipboard_content: Arc::new(Mutex::new(None)),
            is_monitoring: Arc::new(Mutex::new(false)),
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
                    
                    let file_path = match FileManager::get_data_file_path(&app_handle_clone) {
                        Ok(path) => path,
                        Err(e) => {
                            log::warn!("自動保存: ファイルパス取得エラー: {}", e);
                            continue;
                        }
                    };
                    
                    // メモリ効率的なシリアライゼーション
                    if let Ok(json_content) = serde_json::to_string(&data_clone) {
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
                                                // 完全重複アイテムを検索・削除
                                                let mut removed_count = 0;
                                                data.history.retain(|item| {
                                                    if item.content == text {
                                                        removed_count += 1;
                                                        false // 削除
                                                    } else {
                                                        true // 保持
                                                    }
                                                });
                                                
                                                if removed_count > 0 {
                                                    log::info!("重複アイテム{}件を自動削除しました", removed_count);
                                                }
                                                
                                                // 新しいアイテムを追加
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
                                            
                                            // IP検出処理
                                            if let Ok(_data) = app_data.lock() {
                                                let detected_ips = Self::extract_ip_addresses(&text);
                                                drop(_data);
                                                
                                                for ip in detected_ips {
                                                    if let Err(e) = Self::add_ip_to_history(&app_data, ip.clone()) {
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
            Err(_) => Err("Failed to lock monitoring state".to_string()),
        }
    }

    fn extract_ip_addresses(text: &str) -> Vec<String> {
        // IPv4アドレスのパターン: xxx.xxx.xxx.xxx
        let ip_regex = Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap();
        
        let mut ips = Vec::new();
        for cap in ip_regex.find_iter(text) {
            let ip = cap.as_str().to_string();
            
            // IPv4アドレスの妥当性を簡単にチェック
            let parts: Vec<&str> = ip.split('.').collect();
            if parts.len() == 4 {
                let mut valid = true;
                for part in parts {
                    if let Ok(num) = part.parse::<u32>() {
                        if num > 255 {
                            valid = false;
                            break;
                        }
                    } else {
                        valid = false;
                        break;
                    }
                }
                
                if valid {
                    ips.push(ip);
                }
            }
        }
        
        ips
    }

    fn add_ip_to_history(app_data: &Arc<Mutex<AppData>>, ip: String) -> Result<(), String> {
        use crate::models::IpHistoryItem;
        
        let mut data = app_data.lock().map_err(|_| "Failed to lock app data")?;
        
        // 既存のIPを検索
        if let Some(existing_ip) = data.recent_ips.iter_mut().find(|item| item.ip == ip) {
            existing_ip.count += 1;
            existing_ip.timestamp = Utc::now();
            log::info!("IP履歴を更新: {} (count: {})", ip, existing_ip.count);
        } else {
            // 新しいIPを追加
            let ip_item = IpHistoryItem {
                ip: ip.clone(),
                timestamp: Utc::now(),
                count: 1,
            };
            
            // 制限を超えている場合は古いものを削除
            let limit = data.settings.ip_limit;
            if data.recent_ips.len() >= limit {
                data.recent_ips.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                data.recent_ips.remove(0);
            }
            
            data.recent_ips.push(ip_item);
            log::info!("新しいIPを履歴に追加: {}", ip);
        }
        
        // IPを最新順にソート
        data.recent_ips.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(())
    }

    pub fn add_item(&self, content: String, content_type: String) -> Result<(), String> {
        let mut data = self.app_data.lock().map_err(|_| "Failed to lock app data")?;
        
        let item = ClipboardItem {
            id: Uuid::new_v4().to_string(),
            content,
            content_type,
            timestamp: Utc::now(),
            size: 0, // サイズは後で計算
            access_count: 0,
            last_accessed: None,
        };
        
        // 設定で指定された件数制限
        let limit = data.settings.history_limit;
        if data.history.len() >= limit {
            data.history.remove(0);
        }
        
        data.history.push(item);
        log::info!("クリップボード履歴に追加: {} chars", data.history.last().unwrap().size);
        
        Ok(())
    }
}