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
    
    
    // ディスプレイのスケールファクターを取得
    #[cfg(target_os = "macos")]
    fn get_display_scale_factor_for_point(&self, x: f64, y: f64) -> f64 {
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
    
    // マウス位置のウィンドウを強制フォーカス（Core Graphics使用）
    #[cfg(target_os = "macos")]
    async fn force_focus_window_at_mouse(&self, x: f64, y: f64) -> bool {
        extern "C" {
            fn CGEventCreateMouseEvent(
                source: *const std::ffi::c_void,
                mouseType: u32,
                mouseCursorPosition_x: f64,
                mouseCursorPosition_y: f64,
                mouseButton: u32,
            ) -> *const std::ffi::c_void;
            fn CGEventPost(tap: u32, event: *const std::ffi::c_void);
            fn CFRelease(cf: *const std::ffi::c_void);
        }
        
        const K_CG_EVENT_LEFT_MOUSE_DOWN: u32 = 1;
        const K_CG_EVENT_LEFT_MOUSE_UP: u32 = 2;
        const K_CG_MOUSE_BUTTON_LEFT: u32 = 0;
        const K_CG_HID_EVENT_TAP: u32 = 0;
        
        unsafe {
            println!("🖱️ マウス位置({}, {})でクリックイベント生成", x, y);
            
            // マウスダウンイベント
            let mouse_down_event = CGEventCreateMouseEvent(
                std::ptr::null(),
                K_CG_EVENT_LEFT_MOUSE_DOWN,
                x,
                y,
                K_CG_MOUSE_BUTTON_LEFT,
            );
            
            // マウスアップイベント
            let mouse_up_event = CGEventCreateMouseEvent(
                std::ptr::null(),
                K_CG_EVENT_LEFT_MOUSE_UP,
                x,
                y,
                K_CG_MOUSE_BUTTON_LEFT,
            );
            
            if !mouse_down_event.is_null() && !mouse_up_event.is_null() {
                // クリックイベントを送信
                CGEventPost(K_CG_HID_EVENT_TAP, mouse_down_event);
                CGEventPost(K_CG_HID_EVENT_TAP, mouse_up_event);
                
                // メモリ解放
                CFRelease(mouse_down_event);
                CFRelease(mouse_up_event);
                
                println!("✅ クリックイベント送信完了");
                true
            } else {
                println!("❌ クリックイベント作成失敗");
                false
            }
        }
    }
    
    // フォールバック: 非macOS環境
    #[cfg(not(target_os = "macos"))]
    async fn force_focus_window_at_mouse(&self, _x: f64, _y: f64) -> bool {
        false
    }
    
    // マウス位置を同期的に取得
    #[cfg(target_os = "macos")]
    fn get_mouse_position_sync(&self) -> serde_json::Value {
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
        let _scale_factor = mouse_pos.get("scale_factor").and_then(|v| v.as_f64()).unwrap_or(1.0);
        
        // ディスプレイ情報のみ取得（フォーカスは既に事前確認済み）
        let scale_factor = self.get_display_scale_factor_for_point(raw_x as f64, raw_y as f64);
        let display_info = if scale_factor == 2.0 {
            "4Kディスプレイ（メイン）".to_string()
        } else {
            "フルHDディスプレイ（サブ）".to_string()
        };
        let display_info = format!("統一座標系 on {}", display_info);
        
        println!("📍 マウス座標: ({}, {}) on {}", raw_x, raw_y, 
                if scale_factor == 2.0 { "4K" } else { "フルHD" });
        
        let mouse_position = MousePosition {
            x: raw_x,
            y: raw_y,
            scale_factor,
            display_info,
        };
        
        (mouse_position, true) // フォーカスは事前に統一済み
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
        
        println!("🧮 ウィンドウ位置: ({}, {})", final_x, final_y);
        
        WindowPosition {
            x: final_x,
            y: final_y,
            calculation_log: log,
        }
    }
    
    // ウィンドウを表示（安定化処理）
    async fn show_window_at_position(&self, position: &WindowPosition) -> Result<String, String> {
        if let Some(small_window) = self.app_handle.get_webview_window("small") {
            // 強力な位置リセット: 非表示 → 最小化解除
            let _ = small_window.hide();
            let _ = small_window.unminimize();
            
            // Tauri内部の位置記憶をリセットするため、画面外の位置に一度設定
            // center()の代わりに画面外位置を使用してフォーカス競合を回避
            use tauri::Position;
            let reset_position = Position::Physical(tauri::PhysicalPosition { x: -1000, y: -1000 });
            let _ = small_window.set_position(reset_position);
            
            // 短時間待機してTauriの内部状態をリセット
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            
            // スケールファクターに基づいて座標種類を決定
            let tauri_position = if position.calculation_log.contains("スケーリング後") {
                // 4Kディスプレイ: 既にスケーリング済みなのでLogical座標で設定
                let logical_x = (position.x as f64 / 2.0) as i32;
                let logical_y = (position.y as f64 / 2.0) as i32;
                println!("🖥️ 4K座標変換: ({}, {}) → Logical({}, {})", position.x, position.y, logical_x, logical_y);
                Position::Logical(tauri::LogicalPosition { 
                    x: logical_x as f64, 
                    y: logical_y as f64 
                })
            } else {
                // フルHDディスプレイ: Physical座標のまま
                println!("🖥️ フルHD座標: Physical({}, {})", position.x, position.y);
                Position::Physical(tauri::PhysicalPosition { 
                    x: position.x, 
                    y: position.y 
                })
            };
            
            // 位置を複数回設定して確実に反映（Tauri位置記憶の強制上書き）
            for i in 1..=3 {
                match small_window.set_position(tauri_position.clone()) {
                    Ok(_) => {
                        println!("✅ 位置設定成功 ({}回目)", i);
                        break;
                    }
                    Err(e) => {
                        println!("⚠️ 位置設定失敗 ({}回目): {}", i, e);
                        if i == 3 {
                            return Err(format!("位置設定失敗: {}", e));
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                    }
                }
            }
            
            // ウィンドウ表示
            match small_window.show() {
                Ok(_) => {
                    let _ = small_window.set_focus();
                    
                    // 表示後の最終位置確認（問題2のデバッグ用）
                    if let Ok(final_pos) = small_window.inner_position() {
                        println!("🪟 最終位置: {:?}", final_pos);
                        println!("📏 位置差異: 計算({}, {}) vs 実際({:?})", position.x, position.y, final_pos);
                    }
                    
                    Ok("ウィンドウ表示成功".to_string())
                }
                Err(e) => Err(format!("ウィンドウ表示失敗: {}", e))
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
        
        println!("🔥 ホットキー処理開始 ({}回目)", current_count);
        let main_window_focused = if let Some(main_window) = self.app_handle.get_webview_window("main") {
            main_window.is_focused().unwrap_or(false)
        } else {
            false
        };
        
        // スモールウィンドウの現在の状態をチェック
        let small_window_visible = if let Some(small_window) = self.app_handle.get_webview_window("small") {
            small_window.is_visible().unwrap_or(false)
        } else {
            false
        };
        
        // 新しいアプローチ: マウス位置のウィンドウを強制フォーカス
        if !main_window_focused && !small_window_visible {
            println!("🎯 マウス位置のウィンドウを強制フォーカス中...");
            
            // マウス座標を先に取得
            let temp_mouse_pos = self.get_mouse_position_sync();
            let mouse_x = temp_mouse_pos.get("x").and_then(|v| v.as_f64()).unwrap_or(960.0);
            let mouse_y = temp_mouse_pos.get("y").and_then(|v| v.as_f64()).unwrap_or(540.0);
            
            // マウス位置のウィンドウを強制フォーカス
            if self.force_focus_window_at_mouse(mouse_x, mouse_y).await {
                println!("✅ マウス位置ウィンドウのフォーカス成功");
                // 座標系が安定するまで短時間待機
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            } else {
                println!("⚠️ フォーカス失敗 - 従来方式で継続");
                // フォールバック: 従来のメインウィンドウフォーカス
                if let Some(main_window) = self.app_handle.get_webview_window("main") {
                    let _ = main_window.set_focus();
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        } else if small_window_visible {
            println!("🔄 スモールウィンドウ既表示 - クリックスキップ");
        } else {
            println!("✅ 既にフォーカス中 - 即座に処理");
        }
        
        // 統一された座標系でマウス位置を取得し、ウィンドウを表示
        let (mouse_pos, _) = self.get_current_mouse_position();
        let window_pos = self.calculate_window_position(&mouse_pos);
        let result = self.show_window_at_position(&window_pos).await;
        
        // 処理完了ログ
        match &result {
            Ok(_) => println!("✅ ウィンドウ表示完了 ({}回目)", current_count),
            Err(e) => println!("❌ ウィンドウ表示失敗 ({}回目): {}", current_count, e),
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