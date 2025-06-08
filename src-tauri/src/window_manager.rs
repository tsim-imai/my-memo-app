use tauri::{AppHandle, Manager};

// ウィンドウ管理クラス
#[derive(Debug, Clone)]
pub struct WindowManager {
    app_handle: AppHandle,
}

#[derive(Debug, Clone)]
struct MousePosition {
    x: i32,
    y: i32,
    scale_factor: f64,
    display_info: String,
}

#[derive(Debug, Clone)]
struct WindowPosition {
    x: i32,
    y: i32,
    calculation_log: String,
}

impl WindowManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self { app_handle }
    }
    
    // フォーカス状態とディスプレイ情報を取得
    fn get_focus_and_display_info(&self, mouse_x: f64, mouse_y: f64) -> (bool, String) {
        // メインウィンドウのフォーカス状態を確認
        let main_window_focused = if let Some(main_window) = self.app_handle.get_webview_window("main") {
            main_window.is_focused().unwrap_or(false)
        } else {
            false
        };
        
        // マウス位置のディスプレイ情報を取得
        let scale_factor = self.get_display_scale_factor_for_point(mouse_x, mouse_y);
        let display_info = if scale_factor == 2.0 {
            "4Kディスプレイ（メイン）".to_string()
        } else {
            "フルHDディスプレイ（サブ）".to_string()
        };
        
        let focus_info = if main_window_focused {
            "メインウィンドウフォーカス中".to_string()
        } else {
            "外部アプリフォーカス中".to_string()
        };
        
        println!("CONSOLE: フォーカス状態: {}, ディスプレイ: {}", focus_info, display_info);
        
        (main_window_focused, format!("{} on {}", focus_info, display_info))
    }
    
    // ディスプレイのスケールファクターを取得
    #[cfg(target_os = "macos")]
    fn get_display_scale_factor_for_point(&self, x: f64, y: f64) -> f64 {
        println!("🔍 DEBUG: macOS: ディスプレイスケールファクター取得 - 座標: ({}, {})", x, y);
        extern "C" {
            fn CGDisplayPixelsWide(display: u32) -> usize;
            fn CGDisplayPixelsHigh(display: u32) -> usize;
            fn CGGetDisplaysWithPoint(point_x: f64, point_y: f64, max_displays: u32, displays: *mut u32, display_count: *mut u32) -> i32;
        }
        
        unsafe {
            let mut display_id: u32 = 0;
            let mut display_count: u32 = 0;
            
            let result = CGGetDisplaysWithPoint(x, y, 1, &mut display_id, &mut display_count);
            
            if result == 0 && display_count > 0 {
                let logical_width = CGDisplayPixelsWide(display_id) as f64;
                let logical_height = CGDisplayPixelsHigh(display_id) as f64;
                
                let scale_factor = if logical_width == 1512.0 && logical_height == 982.0 {
                    2.0
                } else if logical_width == 1920.0 && logical_height == 1080.0 {
                    1.0
                } else {
                    1.0
                };
                
                scale_factor
            } else {
                1.0
            }
        }
    }
    
    // マウス位置を同期的に取得
    #[cfg(target_os = "macos")]
    fn get_mouse_position_sync(&self) -> serde_json::Value {
        println!("🔍 DEBUG: macOS: マウス位置同期取得開始");
        #[repr(C)]
        struct CGPoint {
            x: f64,
            y: f64,
        }
        
        extern "C" {
            fn CGEventCreate(source: *const std::ffi::c_void) -> *const std::ffi::c_void;
            fn CGEventGetLocation(event: *const std::ffi::c_void) -> CGPoint;
            fn CFRelease(cf: *const std::ffi::c_void);
        }
        
        unsafe {
            let event = CGEventCreate(std::ptr::null());
            if !event.is_null() {
                let location = CGEventGetLocation(event);
                CFRelease(event);
                
                let x = location.x as i32;
                let y = location.y as i32;
                let scale_factor = self.get_display_scale_factor_for_point(location.x, location.y);
                
                return serde_json::json!({
                    "x": x,
                    "y": y,
                    "scale_factor": scale_factor
                });
            }
        }
        
        // フォールバック
        serde_json::json!({
            "x": 960,
            "y": 540,
            "scale_factor": 2.0
        })
    }
    
    // マウス位置とフォーカス情報を取得
    fn get_current_mouse_position(&self) -> (MousePosition, bool) {
        let mouse_pos = self.get_mouse_position_sync();
        let raw_x = mouse_pos.get("x").and_then(|v| v.as_i64()).unwrap_or(960) as i32;
        let raw_y = mouse_pos.get("y").and_then(|v| v.as_i64()).unwrap_or(540) as i32;
        let scale_factor = mouse_pos.get("scale_factor").and_then(|v| v.as_f64()).unwrap_or(1.0);
        
        // フォーカス情報を取得
        let (main_window_focused, display_info) = self.get_focus_and_display_info(raw_x as f64, raw_y as f64);
        
        println!("CONSOLE: マウス位置取得: x={}, y={}, scale={}, {}", 
                raw_x, raw_y, scale_factor, display_info);
        
        let mouse_position = MousePosition {
            x: raw_x,
            y: raw_y,
            scale_factor,
            display_info,
        };
        
        (mouse_position, main_window_focused)
    }
    
    // ウィンドウ位置を計算
    fn calculate_window_position(&self, mouse_pos: &MousePosition) -> WindowPosition {
        let _window_width = 400;  // 将来の境界チェック用に予約
        let window_height = 500;
        
        let (final_x, final_y, log) = if mouse_pos.scale_factor == 2.0 {
            // 4Kディスプレイ: スケーリング適用
            let scaled_x = (mouse_pos.x as f64 * mouse_pos.scale_factor) as i32;
            let scaled_y = (mouse_pos.y as f64 * mouse_pos.scale_factor) as i32;
            let scaled_height = (window_height as f64 * mouse_pos.scale_factor) as i32;
            
            let window_x = scaled_x;
            let window_y = scaled_y - (scaled_height / 2);
            
            let log = format!(
                "{}：元座標({}, {}) → スケーリング後({}, {}) → ウィンドウ位置({}, {})",
                mouse_pos.display_info, mouse_pos.x, mouse_pos.y, scaled_x, scaled_y, window_x, window_y
            );
            
            (window_x, window_y, log)
        } else {
            // フルHDディスプレイ: 生座標使用
            let window_x = mouse_pos.x;
            let window_y = mouse_pos.y - (window_height / 2);
            
            let log = format!(
                "{}：マウス座標({}, {}) → ウィンドウ位置({}, {})",
                mouse_pos.display_info, mouse_pos.x, mouse_pos.y, window_x, window_y
            );
            
            (window_x, window_y, log)
        };
        
        println!("CONSOLE: {}", log);
        
        WindowPosition {
            x: final_x,
            y: final_y,
            calculation_log: log,
        }
    }
    
    // ウィンドウを表示（安定化処理）
    async fn show_window_at_position(&self, position: &WindowPosition) -> Result<String, String> {
        if let Some(small_window) = self.app_handle.get_webview_window("small") {
            println!("CONSOLE: ウィンドウ位置設定開始: target=({}, {})", position.x, position.y);
            
            // 位置設定
            use tauri::Position;
            let tauri_position = Position::Physical(tauri::PhysicalPosition { 
                x: position.x, 
                y: position.y 
            });
            
            match small_window.set_position(tauri_position) {
                Ok(_) => {
                    println!("CONSOLE: 位置設定成功");
                    
                    // 位置設定後の確認（参考情報として）
                    if let Ok(actual_pos) = small_window.inner_position() {
                        println!("CONSOLE: 設定後の実際位置: ({}, {}) [期待値: ({}, {})]", 
                                actual_pos.x, actual_pos.y, position.x, position.y);
                    }
                    
                    // ウィンドウ表示
                    match small_window.show() {
                        Ok(_) => {
                            let _ = small_window.set_focus();
                            
                            // 表示後の最終位置確認
                            if let Ok(final_pos) = small_window.inner_position() {
                                println!("CONSOLE: 表示後の最終位置: {:?}", final_pos);
                            }
                            
                            // スモールウィンドウにフォーカス設定（フォーカス問題対策）
                            println!("CONSOLE: スモールウィンドウにフォーカス設定");
                            
                            Ok(format!("ウィンドウ表示成功: {}", position.calculation_log))
                        }
                        Err(e) => {
                            println!("CONSOLE: ウィンドウ表示失敗: {}", e);
                            Err(format!("ウィンドウ表示失敗: {}", e))
                        }
                    }
                }
                Err(e) => {
                    println!("CONSOLE: 位置設定失敗: {}", e);
                    Err(format!("位置設定失敗: {}", e))
                }
            }
        } else {
            Err("スモールウィンドウが見つかりません".to_string())
        }
    }
    
    // メイン処理：ホットキーからウィンドウ表示まで
    pub async fn handle_hotkey_display(&self) -> Result<String, String> {
        // ホットキー実行回数をカウント（静的変数使用）
        static mut HOTKEY_COUNTER: u32 = 0;
        let current_count = unsafe {
            HOTKEY_COUNTER += 1;
            HOTKEY_COUNTER
        };
        
        println!("CONSOLE: ========================================");
        println!("CONSOLE: 🔥 ホットキー処理開始 ({}回目)", current_count);
        println!("CONSOLE: ========================================");
        
        // 1. マウス位置とフォーカス情報を取得
        println!("CONSOLE: 📍 ステップ1: マウス位置とフォーカス情報取得");
        let (mouse_pos, main_window_focused) = self.get_current_mouse_position();
        
        // フォーカス問題対策: メインウィンドウが非フォーカスの場合は座標系を安定化
        if !main_window_focused {
            println!("CONSOLE: ⚠️ フォーカス問題検出: メインウィンドウが非フォーカス状態");
            
            // メインウィンドウにフォーカスして座標系を統一
            if let Some(main_window) = self.app_handle.get_webview_window("main") {
                println!("CONSOLE: 🎯 メインウィンドウにフォーカス設定中...");
                let _ = main_window.set_focus();
                
                // フォーカスが完全に設定されるまで待機
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                println!("CONSOLE: ✅ フォーカス設定完了");
            }
        } else {
            println!("CONSOLE: ✅ フォーカス状態正常: メインウィンドウがフォーカス中");
        }
        
        // 2. ウィンドウ位置計算
        println!("CONSOLE: 🧮 ステップ2: ウィンドウ位置計算");
        let window_pos = self.calculate_window_position(&mouse_pos);
        
        // 3. ウィンドウ表示
        println!("CONSOLE: 🪟 ステップ3: ウィンドウ表示");
        let result = self.show_window_at_position(&window_pos).await;
        
        // 処理完了ログ
        match &result {
            Ok(_) => {
                println!("CONSOLE: ========================================");
                println!("CONSOLE: ✅ ホットキー処理完了 ({}回目) - 成功", current_count);
                println!("CONSOLE: ========================================");
            }
            Err(e) => {
                println!("CONSOLE: ========================================");
                println!("CONSOLE: ❌ ホットキー処理完了 ({}回目) - 失敗: {}", current_count, e);
                println!("CONSOLE: ========================================");
            }
        }
        
        result
    }
    
    // スモールウィンドウを非表示
    pub async fn hide_window(&self) -> Result<String, String> {
        if let Some(small_window) = self.app_handle.get_webview_window("small") {
            match small_window.hide() {
                Ok(_) => {
                    log::info!("スモールウィンドウを非表示");
                    Ok("Small window hidden successfully".to_string())
                }
                Err(e) => {
                    log::error!("スモールウィンドウ非表示失敗: {}", e);
                    Err(format!("Failed to hide small window: {}", e))
                }
            }
        } else {
            Err("Small window not found".to_string())
        }
    }
}