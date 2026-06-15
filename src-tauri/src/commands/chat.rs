//! Chat Commands

use crate::commands::config::{self, get_app_data_dir};
use crate::llm::{ChatMessage, ChatResponse, LLMManager};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::RwLock;

// ── Token Budget Constants ─────────────────────────────────────────
// Based on DeepSeek-V4-Flash 128K context window with 20% safety margin.
// Adjust these when switching to models with different context sizes.

/// Total input token budget (leaves room for model response + 20% margin)
const TOTAL_CONTEXT_BUDGET: usize = 96_000;

/// Max tokens for character system prompt (role behavior constraints)
const SYSTEM_PROMPT_BUDGET: usize = 3_000;

/// Max tokens for character preset prompt (background story / personality)
const PRESET_PROMPT_BUDGET: usize = 5_000;

/// Max tokens for user profile section
const USER_PROFILE_BUDGET: usize = 2_000;

/// How many recent short-term messages to load initially (will be trimmed if needed)
const SHORT_TERM_LOAD_COUNT: usize = 30;

/// Minimum short-term messages to keep before trimming other components
const SHORT_TERM_MIN_COUNT: usize = 5;

/// Minimum characters to keep when truncating preset prompt as last resort
const PRESET_MIN_CHARS: usize = 500;

/// ── Token Estimation ──────────────────────────────────────────────

/// Conservative token estimation: 1 character ≈ 1.5 tokens
/// Works well for Chinese (1 char ≈ 1-2 tokens) and English (1 word ≈ 1.3 tokens).
/// The 20% safety margin in the budget covers estimation errors.
fn estimate_tokens(text: &str) -> usize {
    (text.chars().count() as f64 * 1.5) as usize
}

/// Estimate total tokens for a list of messages (includes ~4 tokens per message for role metadata)
fn estimate_messages_tokens(messages: &[ChatMessage]) -> usize {
    messages
        .iter()
        .map(|m| estimate_tokens(&m.content) + 4)
        .sum()
}

/// Truncate text to fit within a token budget, breaking at a natural boundary if possible.
fn truncate_text(text: &str, max_tokens: usize) -> String {
    if estimate_tokens(text) <= max_tokens {
        return text.to_string();
    }

    // Convert token budget to approximate char budget (reverse the 1.5x factor)
    let max_chars = (max_tokens as f64 / 1.5) as usize;
    if max_chars >= text.chars().count() {
        return text.to_string();
    }

    // Try to break at the last newline before the limit
    let truncated: String = text.chars().take(max_chars).collect();
    if let Some(last_nl) = truncated.rfind('\n') {
        if last_nl > max_chars / 2 {
            // Only use newline break if it's not too far back
            return truncated[..last_nl].to_string();
        }
    }

    truncated
}

// ── Data Types ─────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequestInput {
    pub message: String,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponseOutput {
    pub content: String,
    pub model: String,
}

impl From<ChatResponse> for ChatResponseOutput {
    fn from(resp: ChatResponse) -> Self {
        Self {
            content: resp.content,
            model: resp.model,
        }
    }
}

/// Chat state management
pub struct ChatState {
    pub messages: Vec<ChatMessage>,
    pub system_prompt: String,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            system_prompt: String::new(),
        }
    }
}

// ── Profile Auto-Update ────────────────────────────────────────────

/// Check whether user profile needs auto-update (every 50 user messages),
/// and trigger it in background if so.
async fn maybe_auto_update_profile(
    character_id: String,
    llm_manager: Arc<LLMManager>,
) {
    let dates = crate::commands::memory::get_chat_dates(character_id.clone())
        .unwrap_or_default();

    let actual_count: u32 = dates
        .iter()
        .map(|date| {
            crate::commands::memory::get_chat_by_date(character_id.clone(), date.clone())
                .map(|c| c.messages.iter().filter(|m| m.role == "user").count() as u32)
                .unwrap_or(0)
        })
        .sum();

    let profile = match crate::commands::character::get_user_profile(character_id.clone()) {
        Ok(p) => p,
        Err(_) => return,
    };

    let new_since_update = actual_count.saturating_sub(profile.last_update_conversation_count);

    if new_since_update >= 50 {
        tokio::spawn(async move {
            match crate::commands::character::trigger_profile_update(character_id, llm_manager).await {
                Ok(_) => log::info!("[chat] profile auto-updated"),
                Err(e) => log::warn!("[chat] profile auto-update failed: {}", e),
            }
        });
    }
}

// ── Tauri Commands ─────────────────────────────────────────────────

/// Send a chat message and get AI response
#[tauri::command]
pub async fn send_message(
    request: ChatRequestInput,
    chat_state: State<'_, Arc<RwLock<ChatState>>>,
    llm_manager: State<'_, Arc<LLMManager>>,
    app_handle: tauri::AppHandle,
) -> Result<ChatResponseOutput, String> {
    let user_message = ChatMessage::user(&request.message);

    let messages = build_full_context(
        &request.message,
        request.system_prompt.clone(),
        chat_state.clone(),
    )
    .await?;

    let is_streaming = llm_manager.is_stream_enabled();

    let response = if is_streaming {
        let chat_id = uuid::Uuid::new_v4().to_string();
        println!("[chat] stream start, chat_id={}", chat_id);

        let _ = app_handle.emit("chat_stream_start", (&chat_id,));

        let chat_id_clone = chat_id.clone();
        let app_handle_clone = app_handle.clone();

        let response = llm_manager
            .chat_stream(messages, move |chunk| {
                println!("[chat] chunk received: {} chars", chunk.len());
                let _ = app_handle_clone.emit("chat_stream_chunk", (&chat_id_clone, &chunk));
            })
            .await
            .map_err(|e| e.to_string())?;

        let _ = app_handle.emit("chat_stream_end", (&chat_id,));
        response
    } else {
        llm_manager.chat(messages).await.map_err(|e| e.to_string())?
    };

    // Save to chat state
    let mut state = chat_state.write().await;
    state.messages.push(user_message);
    state.messages.push(ChatMessage::assistant(&response.content));
    drop(state);

    // Auto-update profile if threshold reached
    if let Ok(config) = crate::commands::config::load_config() {
        maybe_auto_update_profile(config.characters.current, llm_manager.inner().clone()).await;
    }

    println!("[chat] stream end, response_len={}", response.content.len());
    Ok(response.into())
}

/// Send a chat message with streaming TTS support
#[tauri::command]
pub async fn send_message_stream(
    request: ChatRequestInput,
    chat_state: State<'_, Arc<RwLock<ChatState>>>,
    llm_manager: State<'_, Arc<LLMManager>>,
    app_handle: tauri::AppHandle,
) -> Result<ChatResponseOutput, String> {
    let user_message = ChatMessage::user(&request.message);

    let messages = build_full_context(
        &request.message,
        request.system_prompt.clone(),
        chat_state.clone(),
    )
    .await?;

    // Add user message to state
    let mut state = chat_state.write().await;
    state.messages.push(user_message);

    let chat_id = uuid::Uuid::new_v4().to_string();
    println!("[chat] stream start, chat_id={}", chat_id);
    drop(state);

    let _ = app_handle.emit("chat_stream_start", (&chat_id,));

    let app_handle_clone = app_handle.clone();
    let chat_id_clone = chat_id.clone();

    let response = llm_manager
        .chat_stream(messages, move |chunk| {
            println!("[chat] chunk received: {} chars", chunk.len());
            let _ = app_handle_clone.emit("chat_stream_chunk", (&chat_id_clone, &chunk));
        })
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit("chat_stream_end", (&chat_id,));

    // Save assistant message to state
    let assistant_message = ChatMessage::assistant(&response.content);
    let mut state = chat_state.write().await;
    state.messages.push(assistant_message);
    drop(state);

    // Auto-update profile if threshold reached
    if let Ok(config) = crate::commands::config::load_config() {
        maybe_auto_update_profile(config.characters.current, llm_manager.inner().clone()).await;
    }

    println!("[chat] stream end, response_len={}", response.content.len());
    Ok(response.into())
}

// ── Context Building ───────────────────────────────────────────────

/// Build full context with token budget management.
///
/// Components are assembled in priority order:
///   1. Current user message        (never trimmed)
///   2. Short-term memory           (recent messages, trimmed last)
///   3. Character system + preset   (identity, hard caps)
///   4. User profile                (medium priority, hard cap)
///   5. Long-term memory            (lowest priority, trimmed first)
///
/// When total tokens exceed TOTAL_CONTEXT_BUDGET, components are trimmed
/// from lowest priority to highest.
async fn build_full_context(
    user_message: &str,
    custom_system_prompt: Option<String>,
    _chat_state: State<'_, Arc<RwLock<ChatState>>>,
) -> Result<Vec<ChatMessage>, String> {
    let app_config = config::load_config()?;
    let character_id = app_config.characters.current;
    let character = config::load_character(character_id.clone())?;

    // ── Load all raw components ─────────────────────────────────
    let custom_sp = custom_system_prompt.unwrap_or_default();
    let char_sp = &character.system_prompt;
    let preset_sp = &character.preset_prompt;
    let user_profile_map = load_user_profile(&character_id)?;
    let long_term_raw = load_long_term_memory(&character_id)?;
    let short_term_all = load_short_term_memory(&character_id)?;

    // ── Build profile text (with cap) ───────────────────────────
    let profile_text = if user_profile_map.is_empty() {
        String::new()
    } else {
        let mut text = String::from("## 用户信息\n");
        let mut count = 0;
        for (key, value) in &user_profile_map {
            let line = format!("- {}: {}\n", key, value);
            text.push_str(&line);
            count += 1;
            // Stop if profile section exceeds budget
            if estimate_tokens(&text) > USER_PROFILE_BUDGET {
                break;
            }
        }
        text.push_str("\n\n");
        // Note: this silently drops entries beyond the budget
        if count < user_profile_map.len() {
            log::info!("[context] user_profile trimmed: {}/{} entries (budget {} tokens)",
                     count, user_profile_map.len(), USER_PROFILE_BUDGET);
        }
        text
    };

    // ── Truncation levels for long-term memory ──────────────────
    // Try progressively fewer summaries until budget fits.
    // Each level: (label, weekly_count, monthly_count)
    let lt_levels: &[(&str, usize, usize)] = &[
        ("4w+2m", 4, 2),  // default: 4 weeks + 2 months
        ("2w+1m", 2, 1),  // reduced
        ("1w",    1, 0),  // minimal
        ("none",  0, 0),  // discard entirely
    ];

    // ── Short-term message count levels ─────────────────────────
    let st_levels: &[usize] = &[SHORT_TERM_LOAD_COUNT, 20, 10, SHORT_TERM_MIN_COUNT];

    // ── Iterate truncation levels until budget fits ─────────────
    for &(lt_label, lt_weeks, lt_months) in lt_levels {
        // Build long-term memory section for this level
        let lt_text = build_long_term_section(&long_term_raw, lt_weeks, lt_months);

        for &st_count in st_levels {
            // Select most recent short-term messages
            let total_st = short_term_all.len();
            let st_msgs: Vec<ChatMessage> = short_term_all
                .iter()
                .skip(total_st.saturating_sub(st_count))
                .cloned()
                .collect();

            // ── Assemble system prompt ──────────────────────────
            let mut full_system = String::new();

            // Custom system prompt
            if !custom_sp.is_empty() {
                full_system.push_str(&custom_sp);
                full_system.push_str("\n\n");
            }

            // Character system prompt (hard cap)
            if !char_sp.is_empty() {
                let sp = truncate_text(char_sp, SYSTEM_PROMPT_BUDGET);
                full_system.push_str(&sp);
                full_system.push_str("\n\n");
            }

            // Character preset (hard cap)
            if !preset_sp.is_empty() {
                let pp = truncate_text(preset_sp, PRESET_PROMPT_BUDGET);
                full_system.push_str(&pp);
                full_system.push_str("\n\n");
            }

            // User profile
            if !profile_text.is_empty() {
                full_system.push_str(&profile_text);
            }

            // Long-term memory
            if !lt_text.is_empty() {
                full_system.push_str("## 近期记忆\n");
                full_system.push_str(&lt_text);
                full_system.push_str("\n\n");
            }

            // ── Assemble final messages ─────────────────────────
            let mut messages: Vec<ChatMessage> = Vec::new();
            if !full_system.is_empty() {
                messages.push(ChatMessage::system(&full_system));
            }
            messages.extend(st_msgs);
            messages.push(ChatMessage::user(user_message));

            let tokens = estimate_messages_tokens(&messages);

            if tokens <= TOTAL_CONTEXT_BUDGET {
                // Log if any trimming occurred
                let is_default = lt_label == "4w+2m" && st_count == SHORT_TERM_LOAD_COUNT;
                if !is_default {
                    log::info!(
                        "[context] budget ok: tokens={}, long_term={}, short_term={}/{}",
                        tokens, lt_label, st_count, total_st
                    );
                }
                return Ok(messages);
            }
        }
    }

    // ── Last resort: all levels exhausted, force minimal context ──
    // This should only happen with extremely large system/preset prompts.
    // We truncate the preset prompt aggressively and keep minimal history.
    log::warn!("[context] WARNING: still over budget after all trimming, using last resort");

    let forced_preset = truncate_text(preset_sp, PRESET_MIN_CHARS);
    let mut full_system = String::new();

    if !custom_sp.is_empty() {
        full_system.push_str(&custom_sp);
        full_system.push_str("\n\n");
    }
    if !char_sp.is_empty() {
        let sp = truncate_text(char_sp, SYSTEM_PROMPT_BUDGET);
        full_system.push_str(&sp);
        full_system.push_str("\n\n");
    }
    if !forced_preset.is_empty() {
        full_system.push_str(&forced_preset);
        full_system.push_str("\n\n");
    }

    let total_st = short_term_all.len();
    let st_msgs: Vec<ChatMessage> = short_term_all
        .iter()
        .skip(total_st.saturating_sub(SHORT_TERM_MIN_COUNT))
        .cloned()
        .collect();

    let mut messages: Vec<ChatMessage> = Vec::new();
    if !full_system.is_empty() {
        messages.push(ChatMessage::system(&full_system));
    }
    messages.extend(st_msgs);
    messages.push(ChatMessage::user(user_message));

    let tokens = estimate_messages_tokens(&messages);
    log::warn!("[context] last resort: tokens={}", tokens);

    Ok(messages)
}

/// Build long-term memory section from raw text by preserving only
/// the specified number of weekly and monthly summary blocks.
fn build_long_term_section(raw: &str, weeks: usize, months: usize) -> String {
    if raw.is_empty() || (weeks == 0 && months == 0) {
        return String::new();
    }

    let mut result = String::new();
    let mut week_count = 0;
    let mut month_count = 0;

    for line in raw.lines() {
        let is_week_header = line.starts_with("### ") && line.contains("周");
        let is_month_header = line.starts_with("### ") && line.contains("月");

        if is_week_header {
            if week_count >= weeks {
                // Skip remaining weekly entries
                continue;
            }
            week_count += 1;
            result.push_str(line);
            result.push('\n');
        } else if is_month_header {
            if month_count >= months {
                continue;
            }
            month_count += 1;
            result.push_str(line);
            result.push('\n');
        } else {
            // Content line: only include if we're still collecting
            if (week_count > 0 && week_count <= weeks)
                || (month_count > 0 && month_count <= months)
                || (week_count == 0 && month_count == 0)
            {
                result.push_str(line);
                result.push('\n');
            }
        }
    }

    result
}

// ── Data Loading Helpers ────────────────────────────────────────────

/// Load user profile
fn load_user_profile(character_id: &str) -> Result<HashMap<String, String>, String> {
    let profile_path = get_app_data_dir()
        .join("profile")
        .join(character_id)
        .join("user_profile.json");

    if !profile_path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(&profile_path).map_err(|e| e.to_string())?;

    // Try to parse as UserProfile or simple HashMap
    if let Ok(profile) = serde_json::from_str::<HashMap<String, String>>(&content) {
        return Ok(profile);
    }

    // If it's a different format, extract relevant fields
    #[derive(serde::Deserialize)]
    struct SimpleProfile {
        user_name: Option<String>,
        traits: Option<Vec<String>>,
        preferences: Option<HashMap<String, String>>,
        important_dates: Option<HashMap<String, String>>,
    }

    if let Ok(profile) = serde_json::from_str::<SimpleProfile>(&content) {
        let mut result = HashMap::new();
        if let Some(name) = profile.user_name {
            result.insert("用户名".to_string(), name);
        }
        if let Some(traits) = profile.traits {
            result.insert("性格特点".to_string(), traits.join(", "));
        }
        if let Some(prefs) = profile.preferences {
            result.insert("用户偏好".to_string(), serde_json::to_string(&prefs).unwrap_or_default());
        }
        if let Some(dates) = profile.important_dates {
            result.insert("重要日期".to_string(), serde_json::to_string(&dates).unwrap_or_default());
        }
        return Ok(result);
    }

    Ok(HashMap::new())
}

/// Load long term memory (weekly/monthly summaries)
fn load_long_term_memory(character_id: &str) -> Result<String, String> {
    let memory_dir = get_app_data_dir().join("memory").join(character_id);

    let mut memory = String::new();

    // Load weekly summaries
    let weekly_dir = memory_dir.join("weekly");
    if weekly_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&weekly_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
                .collect();

            files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

            for entry in files.iter().take(4) {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(summary) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(summary_text) = summary.get("summary").and_then(|s| s.as_str()) {
                            if let Some(week) = summary.get("week").and_then(|w| w.as_str()) {
                                memory.push_str(&format!("### {} 周\n", week));
                                memory.push_str(summary_text);
                                memory.push_str("\n\n");
                            }
                        }
                    }
                }
            }
        }
    }

    // Load monthly summaries
    let monthly_dir = memory_dir.join("monthly");
    if monthly_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&monthly_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
                .collect();

            files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

            for entry in files.iter().take(2) {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(summary) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(growth) = summary.get("relationship_growth").and_then(|s| s.as_str()) {
                            if let Some(month) = summary.get("month").and_then(|m| m.as_str()) {
                                memory.push_str(&format!("### {} 月\n", month));
                                memory.push_str(growth);
                                memory.push_str("\n\n");
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(memory)
}

/// Load short term memory (today's conversation) — returns the most recent messages.
fn load_short_term_memory(character_id: &str) -> Result<Vec<ChatMessage>, String> {
    let today = Local::now().format("%Y-%m-%d").to_string();

    let chat_path = get_app_data_dir()
        .join("memory")
        .join(character_id)
        .join("chat")
        .join(format!("{}.json", today));

    if !chat_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&chat_path).map_err(|e| e.to_string())?;

    #[derive(Deserialize)]
    struct ChatFile {
        messages: Vec<ChatMessage>,
    }

    let chat_file: ChatFile = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    // Return most recent messages (not oldest!)
    let total = chat_file.messages.len();
    Ok(chat_file
        .messages
        .into_iter()
        .skip(total.saturating_sub(SHORT_TERM_LOAD_COUNT))
        .collect())
}

// ── Other Commands ─────────────────────────────────────────────────

/// Get chat history
#[tauri::command]
pub async fn get_chat_history(
    chat_state: State<'_, Arc<RwLock<ChatState>>>,
) -> Result<Vec<ChatMessage>, String> {
    Ok(chat_state.read().await.messages.clone())
}

/// Clear chat history
#[tauri::command]
pub async fn clear_chat_history(
    chat_state: State<'_, Arc<RwLock<ChatState>>>,
) -> Result<(), String> {
    chat_state.write().await.messages.clear();
    Ok(())
}

/// Set system prompt
#[tauri::command]
pub async fn set_system_prompt(
    prompt: String,
    chat_state: State<'_, Arc<RwLock<ChatState>>>,
) -> Result<(), String> {
    chat_state.write().await.system_prompt = prompt;
    Ok(())
}

/// Check if LLM is available
#[tauri::command]
pub async fn check_llm_available(
    llm_manager: State<'_, Arc<LLMManager>>,
) -> Result<bool, String> {
    Ok(llm_manager.is_available().await)
}
