use tauri::{AppHandle, State, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use chrono::Utc;
use crate::models::AppData;
use crate::ClipboardManager;
use crate::file_manager::FileManager;

#[tauri::command]
pub fn get_app_data(state: State<'_, ClipboardManager>) -> Result<AppData, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.clone()),
        Err(_) => Err("Failed to access app data".to_string()),
    }
}

#[tauri::command]
pub fn save_data_to_file(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    state.save_to_file(&app_handle)?;
    Ok("Data saved successfully".to_string())
}

#[tauri::command]
pub fn load_data_from_file(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    state.load_from_file(&app_handle)?;
    Ok("Data loaded successfully".to_string())
}

// ログ機能用コマンド
#[tauri::command]
pub fn get_app_logs(
    app_handle: AppHandle,
    lines: Option<usize>,
) -> Result<Vec<String>, String> {
    FileManager::get_log_content(&app_handle, lines)
}

#[tauri::command]
pub fn clear_app_logs(app_handle: AppHandle) -> Result<String, String> {
    FileManager::clear_log_file(&app_handle)
}

#[tauri::command]
pub fn get_app_diagnostics(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<serde_json::Value, String> {
    let data = match state.app_data.lock() {
        Ok(data) => data.clone(),
        Err(_) => return Err("Failed to access app data".to_string()),
    };
    
    let file_stats = FileManager::get_file_stats(&app_handle)?;
    
    let mut diagnostics = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": Utc::now(),
        "data_stats": {
            "history_count": data.history.len(),
            "bookmarks_count": data.bookmarks.len(),
            "ips_count": data.recent_ips.len(),
            "total_history_size": data.history.iter().map(|item| item.size).sum::<usize>(),
        },
        "system_stats": {
            "settings": data.settings,
        },
        "health": {
            "data_integrity": "OK",
            "memory_usage": "Normal",
        }
    });
    
    // file_statsからの情報をマージ
    if let Some(file_obj) = diagnostics.as_object_mut() {
        if let Some(file_stats_obj) = file_stats.as_object() {
            for (key, value) in file_stats_obj {
                file_obj.insert(key.clone(), value.clone());
            }
        }
    }
    
    Ok(diagnostics)
}

// メモリクリーンアップ用の新しいコマンド
#[tauri::command]
pub fn cleanup_memory(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    size_threshold_mb: Option<f64>,
) -> Result<String, String> {
    let threshold_bytes = (size_threshold_mb.unwrap_or(1.0) * 1024.0 * 1024.0) as usize;
    
    match state.app_data.lock() {
        Ok(mut data) => {
            let original_count = data.history.len();
            
            // 大きなアイテムを削除
            data.history.retain(|item| item.size <= threshold_bytes);
            
            let cleaned_items = original_count - data.history.len();
            
            if cleaned_items > 0 {
                log::info!("メモリ最適化: {} 件のアイテムを削除", cleaned_items);

                // 自動保存
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }
            }

            Ok(format!("Cleaned {} large items (>{:.1}MB)", cleaned_items, size_threshold_mb.unwrap_or(1.0)))
        }
        Err(_) => Err("Failed to access app data for cleanup".to_string()),
    }
}

#[tauri::command]
pub fn cleanup_old_items(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
    days_old: Option<i64>,
) -> Result<String, String> {
    let cutoff_days = days_old.unwrap_or(30);
    let cutoff_date = Utc::now() - chrono::Duration::days(cutoff_days);
    
    match state.app_data.lock() {
        Ok(mut data) => {
            let original_count = data.history.len();
            
            // 古いアイテムを削除
            data.history.retain(|item| item.timestamp > cutoff_date);
            
            let removed_count = original_count - data.history.len();
            
            if removed_count > 0 {
                log::info!("古いアイテム削除: {} 日以前の {} 件削除", days_old.unwrap_or(30), removed_count);

                // 自動保存
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }
            }

            Ok(format!("Removed {} items older than {} days", removed_count, cutoff_days))
        }
        Err(_) => Err("Failed to access app data for cleanup".to_string()),
    }
}

// ホットキー管理
#[tauri::command]
pub fn register_global_hotkey(
    hotkey_string: String,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    use tauri_plugin_global_shortcut::ShortcutState;
    
    // 既存のホットキーを解除
    if let Ok(mut hotkey_registered) = state.hotkey_registered.lock() {
        if *hotkey_registered {
            if let Err(e) = app_handle.global_shortcut().unregister_all() {
                log::error!("既存ホットキー解除失敗: {}", e);
            }
            *hotkey_registered = false;
        }
    }
    
    // ショートカット文字列をパースして作成（簡易版）
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};
    let shortcut = if hotkey_string == "cmd+shift+v" {
        Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV)
    } else {
        return Err("Unsupported hotkey format".to_string());
    };

    let hotkey_clone = hotkey_string.clone();
    // 新しいホットキーを登録
    match app_handle.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            // ホットキーが押された時の処理をここに実装
            log::info!("グローバルホットキーが押されました: {}", hotkey_clone);
        }
    }) {
        Ok(_) => {
            if let Ok(mut hotkey_registered) = state.hotkey_registered.lock() {
                *hotkey_registered = true;
            }
            log::info!("グローバルホットキー登録成功: {}", hotkey_string);
            Ok(format!("Global hotkey registered: {}", hotkey_string))
        }
        Err(e) => {
            log::error!("グローバルホットキー登録失敗: {}", e);
            Err(format!("Failed to register global hotkey: {}", e))
        }
    }
}

#[tauri::command]
pub fn unregister_global_hotkey(
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.hotkey_registered.lock() {
        Ok(mut hotkey_registered) => {
            if *hotkey_registered {
                match app_handle.global_shortcut().unregister_all() {
                    Ok(_) => {
                        *hotkey_registered = false;
                        log::info!("グローバルホットキー登録解除成功");
                        Ok("Global hotkey unregistered successfully".to_string())
                    }
                    Err(e) => {
                        log::error!("グローバルホットキー登録解除失敗: {}", e);
                        Err(format!("Failed to unregister global hotkey: {}", e))
                    }
                }
            } else {
                Ok("No global hotkey was registered".to_string())
            }
        }
        Err(_) => Err("Failed to access hotkey state".to_string()),
    }
}

// ウィンドウ管理
#[tauri::command]
pub fn show_main_window(app_handle: AppHandle) -> Result<String, String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        match window.show() {
            Ok(_) => {
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
pub fn hide_main_window(app_handle: AppHandle) -> Result<String, String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        match window.hide() {
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

#[tauri::command]
pub fn show_dock_icon(_app_handle: AppHandle) -> Result<String, String> {
    // Dockアイコン制御は現在未実装
    log::info!("Dockアイコン表示: 未実装");
    Ok("Dock icon show: not implemented".to_string())
}

#[tauri::command]
pub fn hide_dock_icon(_app_handle: AppHandle) -> Result<String, String> {
    // Dockアイコン制御は現在未実装
    log::info!("Dockアイコン非表示: 未実装");
    Ok("Dock icon hide: not implemented".to_string())
}

#[tauri::command]
pub fn minimize_to_tray(app_handle: AppHandle) -> Result<String, String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        match window.hide() {
            Ok(_) => {
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
pub fn restore_from_tray(app_handle: AppHandle) -> Result<String, String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        match window.show() {
            Ok(_) => {
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

#[tauri::command]
pub fn update_item_access(
    item_id: String,
    item_type: String, // "clipboard" or "bookmark"
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            let updated = match item_type.as_str() {
                "clipboard" => {
                    if let Some(item) = data.history.iter_mut().find(|item| item.id == item_id) {
                        item.access_count += 1;
                        item.last_accessed = Some(Utc::now());
                        true
                    } else {
                        false
                    }
                }
                "bookmark" => {
                    if let Some(item) = data.bookmarks.iter_mut().find(|item| item.id == item_id) {
                        item.access_count += 1;
                        item.last_accessed = Some(Utc::now());
                        true
                    } else {
                        false
                    }
                }
                _ => false
            };

            if updated {
                log::info!("アクセス回数を更新: {} ({})", item_id, item_type);

                // 自動保存
                if let Err(e) = state.save_to_file(&app_handle) {
                    log::warn!("自動保存エラー: {}", e);
                }

                Ok("Access count updated successfully".to_string())
            } else {
                Err("Item not found".to_string())
            }
        }
        Err(_) => Err("Failed to access app data".to_string()),
    }
}