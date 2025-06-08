use tauri::{AppHandle, State};
use crate::models::ClipboardItem;
use crate::ClipboardManager;

#[tauri::command]
pub fn get_clipboard_history(state: State<'_, ClipboardManager>) -> Result<Vec<ClipboardItem>, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.history.clone()),
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
pub fn add_clipboard_item(
    content: String,
    content_type: String,
    state: State<'_, ClipboardManager>,
) -> Result<String, String> {
    state.add_item(content, content_type)?;
    Ok("Clipboard item added successfully".to_string())
}

#[tauri::command]
pub fn delete_clipboard_item(
    item_id: String,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            if let Some(pos) = data.history.iter().position(|item| item.id == item_id) {
                data.history.remove(pos);
                log::info!("クリップボード履歴アイテム削除: {}", item_id);

                // 自動保存
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
pub fn clear_clipboard_history(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let count = data.history.len();
            data.history.clear();
            log::info!("クリップボード履歴をクリア: {} items", count);

            // 自動保存
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }

            Ok(format!("Cleared {} clipboard items", count))
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
pub fn search_clipboard_history(
    query: String,
    state: State<'_, ClipboardManager>,
) -> Result<Vec<ClipboardItem>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            if query.trim().is_empty() {
                return Ok(data.history.clone());
            }

            let query_lower = query.to_lowercase();
            let mut results: Vec<ClipboardItem> = data
                .history
                .iter()
                .filter(|item| {
                    item.content.to_lowercase().contains(&query_lower)
                        || item.content_type.to_lowercase().contains(&query_lower)
                })
                .cloned()
                .collect();

            // 最新順でソート
            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            
            log::info!("クリップボード検索: '{}' -> {} 件", query, results.len());
            Ok(results)
        }
        Err(_) => Err("Failed to access clipboard history".to_string()),
    }
}

#[tauri::command]
pub fn get_clipboard_stats(
    state: State<'_, ClipboardManager>,
) -> Result<serde_json::Value, String> {
    match state.app_data.lock() {
        Ok(data) => {
            let total_items = data.history.len();
            let total_size: usize = data.history.iter().map(|item| item.size).sum();
            let most_recent = data.history.last().map(|item| &item.timestamp);
            
            Ok(serde_json::json!({
                "total_items": total_items,
                "total_size_bytes": total_size,
                "average_size": if total_items > 0 { total_size / total_items } else { 0 },
                "most_recent_timestamp": most_recent
            }))
        }
        Err(_) => Err("Failed to access clipboard data".to_string()),
    }
}

#[tauri::command]
pub fn stop_clipboard_monitoring(state: State<'_, ClipboardManager>) -> Result<String, String> {
    state.stop_monitoring()?;
    Ok("Clipboard monitoring stopped".to_string())
}

#[tauri::command]
pub fn find_duplicate_clipboard_items(
    state: State<'_, ClipboardManager>,
) -> Result<serde_json::Value, String> {
    use std::collections::HashMap;
    
    match state.app_data.lock() {
        Ok(data) => {
            let mut content_map: HashMap<String, Vec<&ClipboardItem>> = HashMap::new();
            
            // コンテンツ別にグループ化
            for item in &data.history {
                content_map.entry(item.content.clone()).or_default().push(item);
            }
            
            // 重複があるグループのみ抽出
            let duplicates: HashMap<String, Vec<&ClipboardItem>> = content_map
                .into_iter()
                .filter(|(_, items)| items.len() > 1)
                .collect();
            
            log::info!("重複クリップボードアイテム検出: {} グループ", duplicates.len());
            
            let duplicate_info: Vec<serde_json::Value> = duplicates
                .iter()
                .map(|(content, items)| {
                    serde_json::json!({
                        "content_preview": if content.len() > 50 { 
                            format!("{}...", &content[..50]) 
                        } else { 
                            content.clone() 
                        },
                        "count": items.len(),
                        "item_ids": items.iter().map(|item| &item.id).collect::<Vec<_>>(),
                        "timestamps": items.iter().map(|item| &item.timestamp).collect::<Vec<_>>()
                    })
                })
                .collect();
            
            Ok(serde_json::json!({
                "duplicate_groups": duplicate_info,
                "total_duplicates": duplicates.len()
            }))
        }
        Err(_) => Err("Failed to access clipboard data".to_string()),
    }
}