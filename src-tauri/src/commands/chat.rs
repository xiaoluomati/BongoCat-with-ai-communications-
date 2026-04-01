//! Chat Commands

use crate::commands::config;
use crate::llm::{ChatMessage, ChatResponse, LLMManager};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Emitter, State};
use tokio::sync::RwLock;

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

/// Send a chat message and get AI response
#[tauri::command]
pub async fn send_message(
    request: ChatRequestInput,
    chat_state: State<'_, Arc<RwLock<ChatState>>>,
    llm_manager: State<'_, Arc<LLMManager>>,
    app_handle: tauri::AppHandle,
) -> Result<ChatResponseOutput, String> {
    // Add user message
    let user_message = ChatMessage::user(&request.message);
    
    // Build complete context with all components
    let messages = build_full_context(
        &request.message,
        request.system_prompt.clone(),
        chat_state.clone(),
    ).await?;
    
    // Check if streaming is enabled
    let is_streaming = llm_manager.is_stream_enabled();
    
    let response = if is_streaming {
        // Streaming mode: emit chunks via events
        let chat_id = uuid::Uuid::new_v4().to_string();
        
        // Emit start event
        let _ = app_handle.emit("chat_stream_start", (&chat_id,));
        
        let chat_id_clone = chat_id.clone();
        let app_handle_clone = app_handle.clone();
        
        let response = llm_manager
            .chat_stream(messages, move |chunk| {
                let _ = app_handle_clone.emit("chat_stream_chunk", (&chat_id_clone, &chunk));
            })
            .await
            .map_err(|e| e.to_string())?;
        
        // Emit end event
        let _ = app_handle.emit("chat_stream_end", (&chat_id,));
        
        response
    } else {
        // Non-streaming mode: get full response at once
        llm_manager
            .chat(messages)
            .await
            .map_err(|e| e.to_string())?
    };
    
    // Save to chat history
    let mut state = chat_state.write().await;
    state.messages.push(user_message);
    state.messages.push(ChatMessage::assistant(&response.content));
    
    // Auto-update user profile
    let msg_count = state.messages.len() / 2; // user + assistant = 1 pair
    
    // Trigger TTS for AI response (async, non-blocking)
    let tts_text = response.content.clone();
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        let app_handle_inner = app_handle_clone.clone();
        match crate::commands::tts::tts_speak(tts_text, crate::commands::tts::get_current_character_voice_id_internal(), app_handle_clone).await {
            Ok(audio_url) => {
                // Emit TTS ready event with audio URL for frontend to play
                let _ = app_handle_inner.emit("tts_ready", audio_url);
            }
            Err(e) => {
                println!("[TTS] Error: {}", e);
            }
        }
    });
    
    drop(state);
    
    // Check if we should update profile (every 50 messages)
    if msg_count > 0 && msg_count % 50 == 0 {
        // Trigger profile update in background
        let llm_manager = llm_manager.inner().clone();
        tokio::spawn(async move {
            match crate::commands::character::trigger_profile_update(llm_manager).await {
                Ok(_) => {}
                Err(_) => {} // silently ignore errors
            }
        });
    }
    
    Ok(response.into())
}

/// Send a chat message with streaming TTS support
/// This command emits chunks via events for frontend to handle TTS streaming
#[tauri::command]
pub async fn send_message_stream(
    request: ChatRequestInput,
    chat_state: State<'_, Arc<RwLock<ChatState>>>,
    llm_manager: State<'_, Arc<LLMManager>>,
    app_handle: tauri::AppHandle,
) -> Result<ChatResponseOutput, String> {
    // Add user message
    let user_message = ChatMessage::user(&request.message);
    
    // Build complete context
    let messages = build_full_context(
        &request.message,
        request.system_prompt.clone(),
        chat_state.clone(),
    ).await?;
    
    // Add user message to state
    let mut state = chat_state.write().await;
    state.messages.push(user_message);
    
    // Create chat_id for event routing
    let chat_id = uuid::Uuid::new_v4().to_string();
    drop(state);
    
    // Emit start event
    let _ = app_handle.emit("chat_stream_start", (&chat_id,));
    
    // Stream mode - emit chunks via events
    let app_handle_clone = app_handle.clone();
    let chat_id_clone = chat_id.clone();
    
    let response = llm_manager
        .chat_stream(messages, move |chunk| {
            let _ = app_handle_clone.emit("chat_stream_chunk", (&chat_id_clone, &chunk));
        })
        .await
        .map_err(|e| e.to_string())?;
    
    // Emit end event
    let _ = app_handle.emit("chat_stream_end", (&chat_id,));
    
    // Save assistant message to state
    let assistant_message = ChatMessage::assistant(&response.content);
    let mut state = chat_state.write().await;
    state.messages.push(assistant_message);
    
    Ok(response.into())
}

/// Build full context: system_prompt + preset_prompt + user_profile + long_term_memory + short_term_memory + current dialog
async fn build_full_context(
    user_message: &str,
    custom_system_prompt: Option<String>,
    _chat_state: State<'_, Arc<RwLock<ChatState>>>,
) -> Result<Vec<ChatMessage>, String> {
    let mut messages = Vec::new();
    
    // Get current character info
    let character = get_current_character()?;
    let system_prompt = character.system_prompt;
    let preset_prompt = character.preset_prompt;
    
    // Build system prompt from components
    let mut full_system = String::new();
    
    // 1. system_prompt (系统预设：限定LLM输出的文风、格式等)
    if let Some(custom) = custom_system_prompt {
        if !custom.is_empty() {
            full_system.push_str(&custom);
            full_system.push_str("\n\n");
        }
    }
    
    if !system_prompt.is_empty() {
        full_system.push_str(&system_prompt);
        full_system.push_str("\n\n");
    }
    
    // 2. preset_prompt (性格预设：描述角色背景信息)
    if !preset_prompt.is_empty() {
        full_system.push_str(&preset_prompt);
        full_system.push_str("\n\n");
    }
    
    // 3. user_profile (用户画像)
    let user_profile = load_user_profile()?;
    if !user_profile.is_empty() {
        full_system.push_str("## 用户信息\n");
        for (key, value) in &user_profile {
            full_system.push_str(&format!("- {}: {}\n", key, value));
        }
        full_system.push_str("\n\n");
    }
    
    // 4. long_term_memory (长期记忆：周/月总结)
    let long_term_memory = load_long_term_memory()?;
    if !long_term_memory.is_empty() {
        full_system.push_str("## 近期记忆\n");
        full_system.push_str(&long_term_memory);
        full_system.push_str("\n\n");
    }
    
    // Add system message if not empty
    if !full_system.is_empty() {
        messages.push(ChatMessage::system(&full_system));
    }
    
    // 5. short_term_memory (短期记忆：当天对话)
    let short_term_memory = load_short_term_memory()?;
    messages.extend(short_term_memory);
    
    // 6. current dialog
    messages.push(ChatMessage::user(user_message));
    
    Ok(messages)
}

/// Get current character
fn get_current_character() -> Result<config::Character, String> {
    let app_config = config::load_config()?;
    let current_id = app_config.characters.current;
    config::load_character(current_id)
}

/// Load user profile
fn load_user_profile() -> Result<HashMap<String, String>, String> {
    let profile_path = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("data")
        .join("profile")
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
fn load_long_term_memory() -> Result<String, String> {
    let memory_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("data")
        .join("memory");
    
    let mut memory = String::new();
    
    // Load weekly summaries
    let weekly_dir = memory_dir.join("weekly");
    if weekly_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&weekly_dir) {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
                .collect();
            
            // Sort by file name (newest first, assuming filename like 2026-W10.json)
            files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            
            for entry in files.iter().take(4) { // Last 4 weeks
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
            
            // Sort by file name (newest first)
            files.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            
            for entry in files.iter().take(2) { // Last 2 months
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

/// Load short term memory (today's conversation)
fn load_short_term_memory() -> Result<Vec<ChatMessage>, String> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    
    let chat_path = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("data")
        .join("memory")
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
    
    // Return last 20 messages
    Ok(chat_file.messages.into_iter().take(20).collect())
}

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

/// Get available models from Ollama server
#[tauri::command]
pub async fn get_ollama_models() -> Result<Vec<String>, String> {
    // Get the current Ollama configuration
    let config = crate::commands::get_llm_config().map_err(|e| e.to_string())?;
    
    // Only works if current provider is Ollama
    if config.provider != crate::llm::LLMProvider::Ollama {
        return Err("当前提供商不是 Ollama".to_string());
    }
    
    // Create a temporary client to list models
    let client = crate::llm::OllamaClient::new(
        Some(config.base_url),
        Some(config.model),
    );
    
    client.list_models().await.map_err(|e| e.to_string())
}
