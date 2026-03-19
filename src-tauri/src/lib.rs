pub mod commands;
pub mod core;
pub mod llm;
pub mod utils;
pub mod window_follower;

// Explicit imports to avoid glob re-export issues
use commands::{
    check_llm_available, clear_chat_history, clear_all_chats, export_all_chats, export_chats_markdown,
    get_chat_by_date, get_chat_dates, get_chat_history, get_llm_config, get_main_window_position,
    get_memory_info, get_monthly_summaries, get_today_chat, get_weekly_summaries,
    hide_chat_window, load_config, save_chat_message, save_config, save_monthly_summary,
    save_weekly_summary, send_message, set_system_prompt, show_chat_window, toggle_chat_window,
    set_chat_always_on_top, activate_window,
    trigger_monthly_summary, trigger_weekly_summary, trigger_quarter, trigger_year,
    ChatState, start_scheduler,
    load_character, list_characters, save_character, delete_character,
    get_current_character, switch_character,
    get_user_profile, save_user_profile, check_and_update_profile,
    save_quarterly_summary, get_quarterly_summaries, save_yearly_summary, get_yearly_summaries,
};
use core::{
    device::start_device_listening,
    gamepad::{start_gamepad_listing, stop_gamepad_listing},
    prevent_default, setup,
};
use llm::LLMManager;
use std::sync::Arc;
use tauri::{Manager, WindowEvent, generate_handler};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_custom_window::{
    MAIN_WINDOW_LABEL, PREFERENCE_WINDOW_LABEL, show_preference_window,
};
use tokio::sync::RwLock;
use utils::fs_extra::copy_dir;
use window_follower::{WindowFollower, init_window_follower};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_handle = app.handle();

            let main_window = app.get_webview_window(MAIN_WINDOW_LABEL).unwrap();

            let preference_window = app.get_webview_window(PREFERENCE_WINDOW_LABEL).unwrap();

            setup::default(&app_handle, main_window.clone(), preference_window.clone());

            // Initialize LLM Manager
            match get_llm_config() {
                Ok(config) => {
                    let manager = Arc::new(LLMManager::new(config));
                    let manager_clone = manager.clone();
                    
                    // Start async tasks
                    tauri::async_runtime::spawn(async move {
                        // Start scheduler for automatic summary generation
                        start_scheduler(manager_clone.clone()).await;
                        
                        // Initialize LLM
                        if let Err(e) = manager_clone.init().await {
                            eprintln!("Failed to initialize LLM Manager: {}", e);
                        } else {
                            println!("LLM Manager initialized successfully");
                        }
                    });
                    
                    app.manage(manager);
                }
                Err(e) => {
                    eprintln!("Failed to load LLM config: {}", e);
                }
            }

            // Initialize Window Follower
            let window_follower = Arc::new(WindowFollower::new());
            app.manage(window_follower);
            
            // 确保主窗口永远置顶
            if let Some(main_window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                main_window.set_always_on_top(true).ok();
            }
            
            // Setup window movement listener
            init_window_follower(&app_handle);

            Ok(())
        })
        .invoke_handler(generate_handler![
            load_config,
            save_config,
            get_llm_config,
            send_message,
            get_chat_history,
            clear_chat_history,
            set_system_prompt,
            check_llm_available,
            load_character,
            list_characters,
            save_character,
            delete_character,
            get_current_character,
            switch_character,
            get_user_profile,
            save_user_profile,
            check_and_update_profile,
            get_main_window_position,
            show_chat_window,
            hide_chat_window,
            toggle_chat_window,
            set_chat_always_on_top,
            activate_window,
            save_chat_message,
            get_today_chat,
            get_chat_by_date,
            get_chat_dates,
            save_weekly_summary,
            get_weekly_summaries,
            save_monthly_summary,
            get_monthly_summaries,
            save_quarterly_summary,
            get_quarterly_summaries,
            save_yearly_summary,
            get_yearly_summaries,
            export_all_chats,
            export_chats_markdown,
            clear_all_chats,
            get_memory_info,
            trigger_weekly_summary,
            trigger_monthly_summary,
            trigger_quarter,
            trigger_year,
            copy_dir,
            start_device_listening,
            start_gamepad_listing,
            stop_gamepad_listing
        ])
        .manage(Arc::new(RwLock::new(ChatState::default())))
        .plugin(tauri_plugin_custom_window::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_pinia::init())
        .plugin(prevent_default::init())
        .plugin(tauri_plugin_single_instance::init(
            |app_handle, _argv, _cwd| {
                show_preference_window(app_handle);
            },
        ))
        .plugin(
            tauri_plugin_log::Builder::new()
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .filter(|metadata| !metadata.target().contains("gilrs"))
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_locale::init())
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                let _ = window.hide();

                api.prevent_close();
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|app_handle, event| match event {
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen { .. } => {
            show_preference_window(app_handle);
        }
        _ => {
            let _ = app_handle;
        }
    });
}
