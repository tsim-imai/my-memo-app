use tauri::{AppHandle, State};
use uuid::Uuid;
use chrono::Utc;
use crate::models::BookmarkItem;
use crate::ClipboardManager;

#[tauri::command]
pub fn get_bookmarks(state: State<'_, ClipboardManager>) -> Result<Vec<BookmarkItem>, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.bookmarks.clone()),
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
pub fn add_bookmark(
    name: String,
    content: String,
    content_type: String,
    tags: Vec<String>,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
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

            // 自動保存
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }

            Ok("Bookmark added successfully".to_string())
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
pub fn delete_bookmark(
    bookmark_id: String,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            if let Some(pos) = data.bookmarks.iter().position(|b| b.id == bookmark_id) {
                data.bookmarks.remove(pos);
                log::info!("ブックマークを削除しました: {}", bookmark_id);

                // 自動保存
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }

                Ok("Bookmark deleted successfully".to_string())
            } else {
                Err("Bookmark not found".to_string())
            }
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
pub fn update_bookmark(
    bookmark_id: String,
    name: Option<String>,
    content: Option<String>,
    tags: Option<Vec<String>>,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            if let Some(bookmark) = data.bookmarks.iter_mut().find(|b| b.id == bookmark_id) {
                if let Some(new_name) = name {
                    bookmark.name = new_name;
                }
                if let Some(new_content) = content {
                    bookmark.content = new_content;
                }
                if let Some(new_tags) = tags {
                    bookmark.tags = new_tags;
                }
                bookmark.last_accessed = Some(Utc::now());

                log::info!("ブックマークを更新: {}", bookmark_id);

                // 自動保存
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
pub fn search_bookmarks(
    query: String,
    state: State<'_, ClipboardManager>,
) -> Result<Vec<BookmarkItem>, String> {
    match state.app_data.lock() {
        Ok(data) => {
            if query.trim().is_empty() {
                return Ok(data.bookmarks.clone());
            }

            let query_lower = query.to_lowercase();
            let mut results: Vec<BookmarkItem> = data
                .bookmarks
                .iter()
                .filter(|bookmark| {
                    bookmark.name.to_lowercase().contains(&query_lower)
                        || bookmark.content.to_lowercase().contains(&query_lower)
                        || bookmark.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
                })
                .cloned()
                .collect();

            // 最新順でソート
            results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            
            log::info!("ブックマーク検索: '{}' -> {} 件", query, results.len());
            Ok(results)
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
pub fn duplicate_bookmark(
    bookmark_id: String,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            if let Some(original) = data.bookmarks.iter().find(|b| b.id == bookmark_id).cloned() {
                let mut duplicate = original;
                duplicate.id = Uuid::new_v4().to_string();
                duplicate.name = format!("{} (コピー)", duplicate.name);
                duplicate.timestamp = Utc::now();
                duplicate.access_count = 0;
                duplicate.last_accessed = None;

                data.bookmarks.push(duplicate);
                log::info!("ブックマークを複製: {}", bookmark_id);

                // 自動保存
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
pub fn clear_all_bookmarks(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let count = data.bookmarks.len();
            data.bookmarks.clear();
            log::info!("全ブックマークをクリア: {} items", count);

            // 自動保存
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }

            Ok(format!("Cleared {} bookmarks", count))
        }
        Err(_) => Err("Failed to access bookmarks".to_string()),
    }
}

#[tauri::command]
pub fn find_duplicate_bookmarks(
    state: State<'_, ClipboardManager>,
) -> Result<serde_json::Value, String> {
    use std::collections::HashMap;
    
    match state.app_data.lock() {
        Ok(data) => {
            let mut content_map: HashMap<String, Vec<&BookmarkItem>> = HashMap::new();
            
            // コンテンツ別にグループ化
            for bookmark in &data.bookmarks {
                let key = format!("{}:{}", bookmark.name, bookmark.content);
                content_map.entry(key).or_default().push(bookmark);
            }
            
            // 重複があるグループのみ抽出
            let duplicates: HashMap<String, Vec<&BookmarkItem>> = content_map
                .into_iter()
                .filter(|(_, items)| items.len() > 1)
                .collect();
            
            log::info!("重複ブックマーク検出: {} グループ", duplicates.len());
            
            let duplicate_info: Vec<serde_json::Value> = duplicates
                .iter()
                .map(|(key, items)| {
                    serde_json::json!({
                        "key": key,
                        "count": items.len(),
                        "bookmark_ids": items.iter().map(|item| &item.id).collect::<Vec<_>>(),
                        "names": items.iter().map(|item| &item.name).collect::<Vec<_>>()
                    })
                })
                .collect();
            
            Ok(serde_json::json!({
                "duplicate_groups": duplicate_info,
                "total_duplicates": duplicates.len()
            }))
        }
        Err(_) => Err("Failed to access bookmark data".to_string()),
    }
}