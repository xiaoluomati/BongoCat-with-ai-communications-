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

/// 找到包含给定点的显示器
fn find_monitor_at_point(
    monitors: &[tauri::Monitor],
    x: i32,
    y: i32,
) -> Option<tauri::Monitor> {
    monitors.iter().find(|m| {
        let pos = m.position();
        let size = m.size();
        x >= pos.x
            && x < pos.x + size.width as i32
            && y >= pos.y
            && y < pos.y + size.height as i32
    }).cloned()
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

    // 获取聊天窗口当前位置
    let chat_pos = chat_window.outer_position().map_err(|e| e.to_string())?;

    // 获取所有可用显示器
    let monitors = main_window
        .available_monitors()
        .map_err(|e| e.to_string())?;

    // 计算主窗口中心点
    let pet_center_x = main_pos.x + (main_size.width as i32 / 2);
    let pet_center_y = main_pos.y + (main_size.height as i32 / 2);

    // 找到主窗口实际所在的显示器（通过中心点判断）
    let monitor = find_monitor_at_point(&monitors, pet_center_x, pet_center_y)
        .or_else(|| main_window.current_monitor().ok().flatten()) // 回退到 current_monitor
        .or_else(|| monitors.first().cloned()) // 最后回退到第一个显示器
        .ok_or("No monitor available")?;

    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();

    // 判断聊天窗口当前在主窗口的哪一侧
    let chat_center_x = chat_pos.x + CHAT_WINDOW_WIDTH / 2;
    let is_chat_on_left = chat_center_x < pet_center_x;

    // 根据当前位置计算新的聊天窗口位置，保持间距避免重叠
    let chat_x = if is_chat_on_left {
        // 聊天窗口在左侧：放在主窗口左边，保持间距
        // 如果主窗口向左移动接近聊天窗口，聊天窗口也向左移动保持间距
        let desired_x = main_pos.x - CHAT_WINDOW_WIDTH - WINDOW_GAP;
        // 确保不超出显示器左边界
        desired_x.max(monitor_pos.x)
    } else {
        // 聊天窗口在右侧：放在主窗口右边，保持间距
        let desired_x = main_pos.x + main_size.width as i32 + WINDOW_GAP;
        // 确保不超出显示器右边界
        let max_x = monitor_pos.x + monitor_size.width as i32 - CHAT_WINDOW_WIDTH;
        desired_x.min(max_x)
    };

    // 垂直居中
    let chat_y = main_pos.y + (main_size.height as i32 - CHAT_WINDOW_HEIGHT) / 2;

    // 边界检查，确保在当前显示器范围内
    let min_y = monitor_pos.y;
    let max_y = monitor_pos.y + monitor_size.height as i32 - CHAT_WINDOW_HEIGHT;
    let chat_y = chat_y.max(min_y).min(max_y);

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
