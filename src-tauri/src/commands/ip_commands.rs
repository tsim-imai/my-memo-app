use tauri::{AppHandle, State};
use regex::Regex;
use crate::models::IpHistoryItem;
use crate::ClipboardManager;

// IP関数をlib.rsから移動
fn extract_ip_addresses(text: &str) -> Vec<String> {
    // IPv4アドレスのパターン: xxx.xxx.xxx.xxx
    let ip_regex = Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").unwrap();
    
    let mut ips = Vec::new();
    for cap in ip_regex.find_iter(text) {
        let ip = cap.as_str().to_string();
        
        // 有効なIPアドレスかチェック（各オクテットが0-255の範囲内）
        if is_valid_ip(&ip) {
            ips.push(ip);
        }
    }
    
    ips
}

fn is_valid_ip(ip: &str) -> bool {
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

#[tauri::command]
pub fn get_recent_ips(state: State<'_, ClipboardManager>) -> Result<Vec<IpHistoryItem>, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.recent_ips.clone()),
        Err(_) => Err("Failed to access IP history".to_string()),
    }
}

#[tauri::command]
pub fn add_ip_to_recent(
    ip: String,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    // IP形式の検証
    if !is_valid_ip(&ip) {
        return Err("Invalid IP address format".to_string());
    }

    state.add_ip_to_history(ip.clone())?;

    // 自動保存
    if let Err(e) = state.save_to_file(&app_handle) {
        log::warn!("自動保存エラー: {}", e);
    }

    Ok(format!("IP {} added to history", ip))
}

#[tauri::command]
pub fn remove_ip_from_recent(
    ip: String,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            if let Some(pos) = data.recent_ips.iter().position(|item| item.ip == ip) {
                data.recent_ips.remove(pos);
                log::info!("IP履歴から削除: {}", ip);

                // 自動保存
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }

                Ok(format!("IP {} removed from history", ip))
            } else {
                Err("IP not found in history".to_string())
            }
        }
        Err(_) => Err("Failed to access IP history".to_string()),
    }
}

#[tauri::command]
pub fn detect_ips_in_text(text: String) -> Result<Vec<String>, String> {
    let detected_ips = extract_ip_addresses(&text);
    Ok(detected_ips)
}

#[tauri::command]
pub fn clear_ip_history(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let count = data.recent_ips.len();
            data.recent_ips.clear();
            log::info!("IP履歴をクリア: {} items", count);

            // 自動保存
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }

            Ok(format!("Cleared {} IP entries", count))
        }
        Err(_) => Err("Failed to access IP history".to_string()),
    }
}

#[tauri::command]
pub fn search_ip_history(
    query: String,
    state: State<'_, ClipboardManager>,
) -> Result<Vec<IpHistoryItem>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            if query.trim().is_empty() {
                return Ok(data.recent_ips.clone());
            }

            let results: Vec<IpHistoryItem> = data
                .recent_ips
                .iter()
                .filter(|item| item.ip.contains(&query))
                .cloned()
                .collect();

            log::info!("IP履歴検索: '{}' -> {} 件", query, results.len());
            Ok(results)
        }
        Err(_) => Err("Failed to access IP history".to_string()),
    }
}

#[tauri::command]
pub fn reset_ip_count(
    ip: String,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            if let Some(ip_item) = data.recent_ips.iter_mut().find(|item| item.ip == ip) {
                ip_item.count = 1;
                log::info!("IPカウントをリセット: {}", ip);

                // 自動保存
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }

                Ok(format!("Reset count for IP {}", ip))
            } else {
                Err("IP not found in history".to_string())
            }
        }
        Err(_) => Err("Failed to access IP history".to_string()),
    }
}

