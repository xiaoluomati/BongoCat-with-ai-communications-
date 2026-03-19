//! Window Management Commands

use tauri::Manager;
use crate::window_follower::{set_chat_visible, enable_follow, disable_follow, sync_chat_window_position};

const MAIN_WINDOW_LABEL: &str = "main";
const CHAT_WINDOW_LABEL: &str = "chat";
const PREFERENCE_WINDOW_LABEL: &str = "preference";
const COMPREHENSIVE_WINDOW_LABEL: &str = "comprehensive_function";

/// 所有需要管理的窗口标签（除了主窗口，主窗口永远置顶）
const MANAGED_WINDOWS: &[&str] = &[CHAT_WINDOW_LABEL, PREFERENCE_WINDOW_LABEL, COMPREHENSIVE_WINDOW_LABEL];

/// 激活指定窗口，确保它在最前面，同时处理其他窗口的层级
/// 注意：主窗口永远保持置顶，不参与此管理
#[tauri::command]
pub fn activate_window(app: tauri::AppHandle, window_label: String) -> Result<(), String> {
    // 只降低其他管理窗口的层级（不包括主窗口）
    for label in MANAGED_WINDOWS {
        if *label != window_label.as_str() {
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.set_always_on_top(false);
            }
        }
    }
    
    // 激活目标窗口
    let target_window = app
        .get_webview_window(&window_label)
        .ok_or_else(|| format!("Window {} not found", window_label))?;
    
    // 设置置顶并显示
    target_window.set_always_on_top(true).map_err(|e| e.to_string())?;
    target_window.show().map_err(|e| e.to_string())?;
    target_window.set_focus().map_err(|e| e.to_string())?;
    
    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Get main window position
#[tauri::command]
pub fn get_main_window_position(app: tauri::AppHandle) -> Result<WindowPosition, String> {
    let main_window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or("Main window not found")?;

    let position = main_window
        .outer_position()
        .map_err(|e| e.to_string())?;
    let size = main_window.outer_size().map_err(|e| e.to_string())?;

    Ok(WindowPosition {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

/// Show chat window at appropriate position
#[tauri::command]
pub fn show_chat_window(app: tauri::AppHandle) -> Result<(), String> {
    // Enable window follow and set chat as visible
    enable_follow(&app);
    set_chat_visible(&app, true);
    
    // Calculate and set position
    sync_chat_window_position(&app)?;
    
    // Show and focus chat window
    let chat_window = app
        .get_webview_window(CHAT_WINDOW_LABEL)
        .ok_or("Chat window not found")?;
    
    // 恢复置顶状态
    chat_window.set_always_on_top(true).map_err(|e| e.to_string())?;
    
    chat_window.show().map_err(|e| e.to_string())?;
    chat_window.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

/// Hide chat window
#[tauri::command]
pub fn hide_chat_window(app: tauri::AppHandle) -> Result<(), String> {
    let chat_window = app
        .get_webview_window(CHAT_WINDOW_LABEL)
        .ok_or("Chat window not found")?;

    chat_window.hide().map_err(|e| e.to_string())?;
    
    // Update follow state
    set_chat_visible(&app, false);
    disable_follow(&app);

    Ok(())
}

/// Toggle chat window
#[tauri::command]
pub fn toggle_chat_window(app: tauri::AppHandle) -> Result<bool, String> {
    let chat_window = app
        .get_webview_window(CHAT_WINDOW_LABEL)
        .ok_or("Chat window not found")?;

    if chat_window.is_visible().map_err(|e| e.to_string())? {
        chat_window.hide().map_err(|e| e.to_string())?;
        set_chat_visible(&app, false);
        disable_follow(&app);
        Ok(false)
    } else {
        // 使用 activate_window 逻辑确保层级正确
        drop(chat_window);
        
        // 降低其他窗口层级
        for label in MANAGED_WINDOWS {
            if *label != CHAT_WINDOW_LABEL {
                if let Some(window) = app.get_webview_window(label) {
                    let _ = window.set_always_on_top(false);
                }
            }
        }
        
        // 启用跟随并显示
        enable_follow(&app);
        set_chat_visible(&app, true);
        sync_chat_window_position(&app)?;
        
        let chat_window = app
            .get_webview_window(CHAT_WINDOW_LABEL)
            .ok_or("Chat window not found")?;
        
        chat_window.set_always_on_top(true).map_err(|e| e.to_string())?;
        chat_window.show().map_err(|e| e.to_string())?;
        chat_window.set_focus().map_err(|e| e.to_string())?;
        
        Ok(true)
    }
}

/// Set chat window always on top
#[tauri::command]
pub fn set_chat_always_on_top(app: tauri::AppHandle, always_on_top: bool) -> Result<(), String> {
    let chat_window = app
        .get_webview_window(CHAT_WINDOW_LABEL)
        .ok_or("Chat window not found")?;

    chat_window
        .set_always_on_top(always_on_top)
        .map_err(|e| e.to_string())?;

    Ok(())
}
