mod models;
mod file_manager;
mod clipboard_monitor;
mod window_manager;
mod commands;

use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State, Manager};
use chrono::Utc;

use models::{ClipboardItem, IpHistoryItem, AppData};
use file_manager::FileManager;
use clipboard_monitor::ClipboardMonitor;
use window_manager::WindowManager;
use commands::*;


pub struct ClipboardManager {
    app_data: Arc<Mutex<AppData>>,
    monitor: ClipboardMonitor,
    hotkey_registered: Arc<Mutex<bool>>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        let app_data = Arc::new(Mutex::new(AppData::default()));
        let monitor = ClipboardMonitor::new(Arc::clone(&app_data));
        
        Self {
            app_data,
            monitor,
            hotkey_registered: Arc::new(Mutex::new(false)),
        }
    }

    pub fn load_from_file(&self, app_handle: &AppHandle) -> Result<(), String> {
        let loaded_data = FileManager::load_from_file(app_handle)?;

        match self.app_data.lock() {
            Ok(mut data) => {
                *data = loaded_data;
                
                // 起動時の自動重複削除
                let original_history_count = data.history.len();
                let original_bookmarks_count = data.bookmarks.len();
                
                // クリップボード履歴の重複削除
                use std::collections::HashMap;
                let mut seen_content: HashMap<String, ClipboardItem> = HashMap::new();
                
                for item in data.history.iter() {
                    let content_key = item.content.clone();
                    
                    if let Some(existing_item) = seen_content.get(&content_key) {
                        if item.timestamp > existing_item.timestamp {
                            seen_content.insert(content_key, item.clone());
                        }
                    } else {
                        seen_content.insert(content_key, item.clone());
                    }
                }
                
                let mut unique_history: Vec<ClipboardItem> = seen_content.into_values().collect();
                unique_history.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                data.history = unique_history;
                
                // ブックマークの重複削除
                let mut seen_bookmarks = std::collections::HashSet::new();
                let mut unique_bookmarks = Vec::new();
                
                for bookmark in data.bookmarks.iter().rev() {
                    let content_key = format!("{}:{}", bookmark.name, bookmark.content);
                    if !seen_bookmarks.contains(&content_key) {
                        seen_bookmarks.insert(content_key);
                        unique_bookmarks.push(bookmark.clone());
                    }
                }
                
                unique_bookmarks.reverse();
                data.bookmarks = unique_bookmarks;
                
                let history_removed = original_history_count - data.history.len();
                let bookmarks_removed = original_bookmarks_count - data.bookmarks.len();
                
                if history_removed > 0 || bookmarks_removed > 0 {
                    log::info!("起動時自動重複削除: 履歴{}件、ブックマーク{}件を削除", history_removed, bookmarks_removed);
                }
                
                log::info!("データファイルから読み込み完了");
                Ok(())
            }
            Err(_) => Err("Failed to lock app data for loading".to_string()),
        }
    }

    pub fn save_to_file(&self, app_handle: &AppHandle) -> Result<(), String> {
        let data_to_save = match self.app_data.lock() {
            Ok(data) => data.clone(),
            Err(_) => return Err("Failed to lock app data for saving".to_string()),
        };

        FileManager::save_to_file(app_handle, &data_to_save)
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
        self.monitor.add_item(content, content_type)
    }

    pub fn start_auto_save(&self, app_handle: AppHandle) {
        self.monitor.start_auto_save(app_handle);
    }

    pub fn start_monitoring(&self, app_handle: AppHandle) -> Result<(), String> {
        self.monitor.start_monitoring(app_handle)
    }

    pub fn stop_monitoring(&self) -> Result<(), String> {
        self.monitor.stop_monitoring()
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
    
    // クリップボード監視を開始（エラーを無視）
    if let Err(e) = state.start_monitoring(app_handle.clone()) {
        log::warn!("クリップボード監視開始失敗: {}", e);
    }
    
    // グローバルホットキーを自動登録（エラーを無視）
    match register_global_hotkey("cmd+shift+v".to_string(), state.clone(), app_handle.clone()) {
        Ok(msg) => log::info!("グローバルホットキー自動登録: {}", msg),
        Err(e) => log::warn!("グローバルホットキー自動登録失敗: {}", e),
    }
    
    log::info!("Clipboard manager initialized and monitoring started");
    Ok("Clipboard manager started".to_string())
}

// ウィンドウ操作用のカスタムコマンド
#[tauri::command]
async fn show_small_window_at_mouse(app_handle: AppHandle) -> Result<String, String> {
    println!("🔍 DEBUG: show_small_window_at_mouse: 開始");
    let window_manager = WindowManager::new(app_handle);
    let result = window_manager.handle_hotkey_display().await;
    println!("🔍 DEBUG: show_small_window_at_mouse: 結果 = {:?}", result);
    result
}

#[tauri::command]
async fn hide_small_window(app_handle: AppHandle) -> Result<String, String> {
    let window_manager = WindowManager::new(app_handle);
    window_manager.hide_window().await
}

// アクセシビリティ権限チェック（macOS専用）
#[tauri::command]
#[cfg(target_os = "macos")]
async fn check_accessibility_permission() -> Result<bool, String> {
    use std::process::Command;
    
    let output = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get properties")
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
            Ok(false)
        }
    }
}

#[tauri::command]
#[cfg(not(target_os = "macos"))]
async fn check_accessibility_permission() -> Result<bool, String> {
    Ok(true) // non-macOSでは常にtrue
}

#[tauri::command]
#[cfg(target_os = "macos")]
async fn request_accessibility_permission() -> Result<String, String> {
    use std::process::Command;
    
    match Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn() {
        Ok(_) => {
            log::info!("システム環境設定のアクセシビリティ画面を開きました");
            Ok("Accessibility permission panel opened".to_string())
        }
        Err(_) => {
            log::error!("システム環境設定を開けませんでした");
            Err("Failed to open accessibility settings".to_string())
        }
    }
}

#[tauri::command]
#[cfg(not(target_os = "macos"))]
async fn request_accessibility_permission() -> Result<String, String> {
    Err("Accessibility permission request is only available on macOS".to_string())
}

#[tauri::command]
async fn check_permissions_status() -> Result<serde_json::Value, String> {
    let accessibility_permission = check_accessibility_permission().await.unwrap_or(false);
    
    log::info!("権限ステータス: {:?}", accessibility_permission);
    
    Ok(serde_json::json!({
        "accessibility": accessibility_permission,
        "all_granted": accessibility_permission
    }))
}

#[tauri::command]
async fn get_permission_instructions() -> Result<serde_json::Value, String> {
    let instructions = serde_json::json!({
        "title": "アクセシビリティ権限が必要です",
        "description": "クリップボード履歴を使用するには、システム環境設定でアクセシビリティ権限を許可してください。",
        "steps": [
            "1. システム環境設定を開く",
            "2. セキュリティとプライバシー → プライバシー",
            "3. アクセシビリティを選択",
            "4. このアプリにチェックを入れる"
        ]
    });
    
    Ok(instructions)
}

// コンテンツ貼り付け機能
#[tauri::command]
async fn paste_content(content: String) -> Result<String, String> {
    use std::process::Command;
    
    // AppleScriptを使用してコンテンツをクリップボードに設定し、貼り付け
    let script = format!(
        r#"
        set the clipboard to "{}"
        tell application "System Events"
            keystroke "v" using command down
        end tell
        "#,
        content.replace("\"", "\\\"").replace("\n", "\\n")
    );
    
    match Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output() {
        Ok(output) => {
            if output.status.success() {
                log::info!("貼り付け成功: {} chars", content.len());
                Ok("Content pasted successfully".to_string())
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                log::error!("貼り付け失敗: {}", error);
                Err(format!("Failed to paste content: {}", error))
            }
        }
        Err(e) => {
            log::error!("AppleScript実行エラー: {}", e);
            Err(format!("AppleScript execution failed: {}", e))
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .manage(ClipboardManager::new())
    .setup(|app| {
      log::info!("App setup completed");
      
      // グローバルホットキーイベントリスナーを設定
      use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, GlobalShortcutExt};
      
      let app_handle = app.handle().clone();
      let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);
      
      log::info!("グローバルホットキー登録試行: Cmd+Shift+V");
      
      match app.global_shortcut().on_shortcut(shortcut, move |_app_handle, _shortcut, event| {
        println!("🔥 HOTKEY: グローバルホットキーが押されました: Cmd+Shift+V, イベント: {:?}", event);
        
        // イベントをStringに変換して判定（プレス時のみ反応）
        let event_str = format!("{:?}", event);
        if event_str.contains("Released") {
          println!("🔥 HOTKEY: Released イベントをスキップ");
          return; // キーを離した時は何もしない
        }
        
        println!("🔥 HOTKEY: Pressed イベント - 処理開始");
        
        // マウス位置にスモールウィンドウを表示
        let app_handle_clone = app_handle.clone();
        // ランタイムをチェックして処理を分岐
        println!("🔥 HOTKEY: tokioランタイムを確認中...");
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
          println!("🔥 HOTKEY: tokioランタイム発見 - 非同期処理に進む");
          runtime.spawn(async move {
            println!("🔥 HOTKEY: ホットキー処理開始: 非同期処理");
            
            // まずマウス位置での表示を試行
            match show_small_window_at_mouse(app_handle_clone.clone()).await {
              Ok(msg) => {
                log::info!("マウス位置でのスモールウィンドウ表示成功: {}", msg);
              },
              Err(e) => {
                log::error!("マウス位置での表示失敗: {}", e);
                // フォールバック: 通常の表示方法
                if let Some(small_window) = app_handle_clone.get_webview_window("small") {
                  if let Ok(_) = small_window.show() {
                    log::info!("フォールバック表示成功（center）");
                  } else {
                    log::error!("スモールウィンドウが見つかりません");
                  }
                }
              }
            }
          });
        } else {
          println!("🔥 HOTKEY: tokioランタイムが見つかりません - 同期処理でWindowManager実行");
          
          // 同期処理ではWindowManagerを直接使えないので、非同期ランタイムを作成
          let app_handle_sync = app_handle.clone();
          std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
              println!("🔥 HOTKEY: 新しいランタイムで WindowManager 実行");
              match show_small_window_at_mouse(app_handle_sync).await {
                Ok(msg) => println!("🔥 HOTKEY: WindowManager成功: {}", msg),
                Err(e) => println!("🔥 HOTKEY: WindowManagerエラー: {}", e),
              }
            });
          });
        }
      }) {
        Ok(_) => {
          log::info!("グローバルホットキー登録成功: Cmd+Shift+V");
        }
        Err(e) => {
          log::error!("グローバルホットキー登録失敗: {}", e);
        }
      }
      
      // システムトレイメニューの設定
      use tauri::{
        menu::{Menu, MenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
      };
      
      let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>).unwrap();
      let show_item = MenuItem::with_id(app, "show", "ウィンドウを表示", true, None::<&str>).unwrap();
      let hide_item = MenuItem::with_id(app, "hide", "ウィンドウを非表示", true, None::<&str>).unwrap();
      let clear_item = MenuItem::with_id(app, "clear", "履歴をクリア", true, None::<&str>).unwrap();
      
      let menu = Menu::with_items(app, &[&show_item, &hide_item, &clear_item, &quit_item]).unwrap();
      
      let _tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
          "quit" => {
            log::info!("トレイメニュー: アプリケーション終了");
            app.exit(0);
          }
          "show" => {
            log::info!("トレイメニュー: ウィンドウを表示");
            if let Some(window) = app.get_webview_window("main") {
              let _ = window.show();
              let _ = window.set_focus();
            }
          }
          "hide" => {
            log::info!("トレイメニュー: ウィンドウを非表示");
            if let Some(window) = app.get_webview_window("main") {
              let _ = window.hide();
            }
          }
          "clear" => {
            log::info!("トレイメニュー: 履歴をクリア");
            // ここでクリップボード履歴をクリアする処理を追加
          }
          _ => {}
        })
        .on_tray_icon_event(|_tray, event| {
          if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
          } = event {
            log::info!("トレイアイコンをクリック");
          }
          if let TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
          } = event {
            log::info!("トレイアイコンをダブルクリック");
          }
        })
        .build(app);
      
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
        init_clipboard_manager,
        show_small_window_at_mouse,
        hide_small_window,
        check_accessibility_permission,
        request_accessibility_permission,
        check_permissions_status,
        get_permission_instructions,
        paste_content,
        // commandsモジュールのコマンドを追加
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
        duplicate_bookmark,
        clear_all_bookmarks,
        clear_ip_history,
        search_ip_history,
        reset_ip_count,
        find_duplicate_clipboard_items,
        find_duplicate_bookmarks,
        get_app_logs,
        clear_app_logs,
        get_app_diagnostics,
        cleanup_memory,
        cleanup_old_items,
        register_global_hotkey,
        unregister_global_hotkey,
        show_main_window,
        hide_main_window,
        show_dock_icon,
        hide_dock_icon,
        minimize_to_tray,
        restore_from_tray,
        update_item_access
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}