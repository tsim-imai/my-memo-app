use tauri::{AppHandle, State};
use crate::models::AppSettings;
use crate::ClipboardManager;

#[tauri::command]
pub fn get_settings(state: State<'_, ClipboardManager>) -> Result<AppSettings, String> {
    match state.app_data.lock() {
        Ok(data) => Ok(data.settings.clone()),
        Err(_) => Err("Failed to access settings".to_string()),
    }
}

#[tauri::command]
pub fn update_settings(
    new_settings: AppSettings,
    state: State<'_, ClipboardManager>,
    app_handle: AppHandle,
) -> Result<String, String> {
    match state.app_data.lock() {
        Ok(mut data) => {
            data.settings = new_settings;
            log::info!("設定を更新しました");

            // 自動保存
            if let Err(e) = state.save_to_file(&app_handle) {
                log::warn!("自動保存エラー: {}", e);
            }

            Ok("Settings updated successfully".to_string())
        }
        Err(_) => Err("Failed to access settings".to_string()),
    }
}