//! Window Follower - Sync chat window position with main window

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Manager;

const MAIN_WINDOW_LABEL: &str = "main";
const CHAT_WINDOW_LABEL: &str = "chat";

const WINDOW_GAP: i32 = 12; // 主窗口与聊天窗口间距
const CHAT_WINDOW_WIDTH: i32 = 500; // 聊天窗口宽度
const CHAT_WINDOW_HEIGHT: i32 = 700; // 聊天窗口高度

/// 记录窗口跟随状态
pub struct WindowFollower {
    pub enabled: AtomicBool,
    pub chat_visible: AtomicBool,
}

impl WindowFollower {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            chat_visible: AtomicBool::new(false),
        }
    }
}

impl Default for WindowFollower {
    fn default() -> Self {
        Self::new()
    }
}

/// 计算并设置聊天窗口位置
pub fn sync_chat_window_position(app: &tauri::AppHandle) -> Result<(), String> {
    let follower = app
        .try_state::<Arc<WindowFollower>>()
        .ok_or("WindowFollower not initialized")?;

    // 如果跟随功能未启用或聊天窗口不可见，跳过
    if !follower.enabled.load(Ordering::Relaxed) || !follower.chat_visible.load(Ordering::Relaxed)
    {
        return Ok(());
    }

    let main_window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or("Main window not found")?;

    let chat_window = app
        .get_webview_window(CHAT_WINDOW_LABEL)
        .ok_or("Chat window not found")?;

    // 获取主窗口位置
    let main_pos = main_window.outer_position().map_err(|e| e.to_string())?;
    let main_size = main_window.outer_size().map_err(|e| e.to_string())?;

    // 获取当前显示器
    let monitor = main_window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or("No monitor found")?;
    let screen_size = monitor.size();

    // 计算主窗口中心点相对于屏幕的位置
    let pet_center_x = main_pos.x + (main_size.width as i32 / 2);
    let screen_center_x = screen_size.width as i32 / 2;

    // 根据主窗口位置决定聊天窗口显示在哪一侧
    let chat_x = if pet_center_x < screen_center_x {
        // 主窗口在左，聊天窗口在右侧
        main_pos.x + main_size.width as i32 + WINDOW_GAP
    } else {
        // 主窗口在右，聊天窗口在左侧
        main_pos.x - CHAT_WINDOW_WIDTH - WINDOW_GAP
    };

    // 垂直居中
    let chat_y = main_pos.y + (main_size.height as i32 - CHAT_WINDOW_HEIGHT) / 2;

    // 边界检查，确保不超出屏幕
    let chat_x = chat_x.max(0).min(screen_size.width as i32 - CHAT_WINDOW_WIDTH);
    let chat_y = chat_y.max(0).min(screen_size.height as i32 - CHAT_WINDOW_HEIGHT);

    chat_window
        .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
            x: chat_x,
            y: chat_y,
        }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 启用窗口跟随
pub fn enable_follow(app: &tauri::AppHandle) {
    if let Some(follower) = app.try_state::<Arc<WindowFollower>>() {
        follower.enabled.store(true, Ordering::Relaxed);
    }
}

/// 禁用窗口跟随
pub fn disable_follow(app: &tauri::AppHandle) {
    if let Some(follower) = app.try_state::<Arc<WindowFollower>>() {
        follower.enabled.store(false, Ordering::Relaxed);
    }
}

/// 设置聊天窗口可见状态
pub fn set_chat_visible(app: &tauri::AppHandle, visible: bool) {
    if let Some(follower) = app.try_state::<Arc<WindowFollower>>() {
        follower.chat_visible.store(visible, Ordering::Relaxed);
    }
}

/// 初始化窗口跟随监听器
pub fn init_window_follower(app_handle: &tauri::AppHandle) {
    let app_handle_clone = app_handle.clone();

    if let Some(main_window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
        main_window.on_window_event(move |event| {
            if let tauri::WindowEvent::Moved(_) = event {
                // 主窗口移动时，同步更新聊天窗口位置
                if let Err(e) = sync_chat_window_position(&app_handle_clone) {
                    eprintln!("Failed to sync chat window position: {}", e);
                }
            }
        });
    }
}
