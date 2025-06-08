use std::fs;
use std::path::PathBuf;
use std::io::Write;
use tauri::{AppHandle, Manager};
use chrono::Utc;
use serde_json;
use crate::models::AppData;

pub struct FileManager;

impl FileManager {
    pub fn get_data_file_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
        let app_data_dir = app_handle.path().app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;
        
        // ディレクトリが存在しない場合は作成
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir)
                .map_err(|e| format!("Failed to create app data directory: {}", e))?;
        }
        
        Ok(app_data_dir.join("clipboard_data.json"))
    }
    
    pub fn get_log_file_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
        let app_data_dir = app_handle.path().app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {}", e))?;
        
        if !app_data_dir.exists() {
            fs::create_dir_all(&app_data_dir)
                .map_err(|e| format!("Failed to create app data directory: {}", e))?;
        }
        
        Ok(app_data_dir.join("clipboard_manager.log"))
    }
    
    pub fn log_to_file(app_handle: &AppHandle, level: &str, message: &str) {
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

    pub fn load_from_file(app_handle: &AppHandle) -> Result<AppData, String> {
        let file_path = Self::get_data_file_path(app_handle)?;
        
        if !file_path.exists() {
            log::info!("データファイルが存在しないため、デフォルト設定を使用します");
            return Ok(AppData::default());
        }

        let file_content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read data file: {}", e))?;

        if file_content.trim().is_empty() {
            log::warn!("データファイルが空です。デフォルト設定を使用します。");
            return Ok(AppData::default());
        }

        let loaded_data: AppData = match serde_json::from_str(&file_content) {
            Ok(data) => data,
            Err(e) => {
                log::error!("JSONパースエラー: {}. バックアップを作成してデフォルト設定で続行します", e);
                
                // 破損したファイルをバックアップ
                let backup_path = file_path.with_extension("json.backup");
                if let Err(backup_err) = fs::copy(&file_path, &backup_path) {
                    log::warn!("バックアップ作成失敗: {}", backup_err);
                } else {
                    log::info!("破損したファイルのバックアップを作成: {:?}", backup_path);
                }
                
                return Ok(AppData::default());
            }
        };

        log::info!("データファイルから読み込み完了: {:?}", file_path);
        Ok(loaded_data)
    }

    pub fn save_to_file(app_handle: &AppHandle, data: &AppData) -> Result<(), String> {
        let file_path = Self::get_data_file_path(app_handle)?;

        // エラーハンドリング強化: データサイズチェック
        if data.history.len() > data.settings.history_limit * 2 {
            log::warn!("履歴アイテム数が制限を大幅に超過しています: {}", data.history.len());
        }

        let json_content = serde_json::to_string(&data)
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

    pub fn get_log_content(app_handle: &AppHandle, max_lines: Option<usize>) -> Result<Vec<String>, String> {
        let log_path = Self::get_log_file_path(app_handle)?;
        let max_lines = max_lines.unwrap_or(500);

        if !log_path.exists() {
            return Ok(vec!["ログファイルが存在しません".to_string()]);
        }

        let content = fs::read_to_string(&log_path)
            .map_err(|e| format!("Failed to read log file: {}", e))?;

        let mut log_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        // 最大行数制限
        if log_lines.len() > max_lines {
            let total_lines = log_lines.len();
            log_lines = log_lines.into_iter().skip(total_lines - max_lines).collect();
        }

        Ok(log_lines)
    }

    pub fn clear_log_file(app_handle: &AppHandle) -> Result<String, String> {
        let log_path = Self::get_log_file_path(app_handle)?;

        if log_path.exists() {
            fs::remove_file(&log_path)
                .map_err(|e| format!("Failed to clear log file: {}", e))?;
        }

        Self::log_to_file(app_handle, "INFO", "ログファイルがクリアされました");
        Ok("ログファイルをクリアしました".to_string())
    }

    pub fn get_file_stats(app_handle: &AppHandle) -> Result<serde_json::Value, String> {
        let log_path = Self::get_log_file_path(app_handle)?;
        let data_path = Self::get_data_file_path(app_handle)?;

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

        Ok(serde_json::json!({
            "data_file_path": data_path.to_string_lossy(),
            "data_file_size": data_size,
            "log_file_path": log_path.to_string_lossy(),
            "log_file_size": log_size,
            "total_size": data_size + log_size,
            "disk_usage": if data_size + log_size > 10 * 1024 * 1024 { "High" } else { "Normal" }
        }))
    }
}