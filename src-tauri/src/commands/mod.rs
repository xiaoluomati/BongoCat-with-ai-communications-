//! Tauri Commands Module

pub mod chat;
pub mod character;
pub mod config;
pub mod memory;
pub mod prompt;
pub mod scheduler;
pub mod tts;
pub mod window;

// Explicit re-exports - avoid glob re-exports due to naming conflicts
pub use chat::*;
pub use config::*;

// Memory - only export what doesn't conflict
pub use memory::{
    ChatMessage, DayChat, WeeklySummary, MonthlySummary, QuarterlySummary, YearlySummary,
    save_chat_message, get_today_chat, get_chat_by_date, get_chat_dates,
    save_weekly_summary, get_weekly_summaries, save_monthly_summary, get_monthly_summaries,
    save_quarterly_summary, get_quarterly_summaries, save_yearly_summary, get_yearly_summaries,
    export_all_chats, export_chats_markdown, clear_all_chats, get_memory_info
};

pub use prompt::*;
pub use scheduler::{start_scheduler, trigger_weekly_summary, trigger_monthly_summary, trigger_quarter, trigger_year};
pub use window::{
    activate_window, get_main_window_position, 
    show_chat_window, hide_chat_window, toggle_chat_window, set_chat_always_on_top,
    exit_app, relaunch_app
};

// Character - explicit export to avoid conflict with config
pub use character::{UserProfile, CharacterBrief, get_user_profile, save_user_profile, check_and_update_profile, trigger_profile_update, get_current_character, switch_character, list_character_briefs};

// TTS commands
pub use tts::{
    get_tts_config, save_tts_config, get_voice_config, save_voice, delete_voice,
    tts_speak, clear_tts_cache, get_tts_cache_info,
    get_index_tts_voices, get_index_tts_emos
};
