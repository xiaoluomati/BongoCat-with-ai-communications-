//! Character Enhancement - User Profile & Extended Commands

use crate::commands::config::get_app_data_dir;
use crate::llm::LLMManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use chrono::Local;
use tauri::State;

fn get_data_dir() -> PathBuf { get_app_data_dir() }

fn get_profile_dir(character_id: &str) -> PathBuf {
    get_data_dir().join("profile").join(character_id)
}

/// Try to extract JSON from LLM response that may contain extra text/markdown
fn extract_json(text: &str) -> Option<serde_json::Value> {
    // Try direct parse first
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return Some(v);
    }

    // Try to extract from ```json ... ``` or ``` ... ``` blocks
    let text_lower = text.to_lowercase();
    if let Some(start) = text_lower.find("```json") {
        let after_start = &text[start + 6..];
        if let Some(end) = after_start.find("```") {
            let json_str = &after_start[..end];
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str.trim()) {
                return Some(v);
            }
        }
    }

    // Try to find first { to last } in the response
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                let json_str = &text[start..=end];
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                    return Some(v);
                }
            }
        }
    }

    None
}

#[allow(dead_code)]
fn get_memory_base_dir() -> PathBuf {
    get_data_dir().join("memory")
}

pub fn ensure_character_dirs(character_id: &str) -> Result<(), String> {
    let base = get_data_dir();
    fs::create_dir_all(base.join("profile").join(character_id)).map_err(|e| e.to_string())?;
    fs::create_dir_all(base.join("memory").join(character_id).join("chat")).map_err(|e| e.to_string())?;
    fs::create_dir_all(base.join("memory").join(character_id).join("weekly")).map_err(|e| e.to_string())?;
    fs::create_dir_all(base.join("memory").join(character_id).join("monthly")).map_err(|e| e.to_string())?;
    Ok(())
}

/// 用户画像
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UserProfile {
    pub user_name: Option<String>,
    pub traits: Vec<String>,
    pub preferences: HashMap<String, String>,
    pub important_dates: HashMap<String, String>,
    pub recent_interactions: Vec<Interaction>,      // 最近互动
    pub special_memories: Vec<SpecialMemory>,       // 专属回忆
    pub last_update_conversation_count: u32,       // 上次更新画像时的对话轮数
    pub last_updated: String,
}

/// 最近互动
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Interaction {
    pub date: String,           // 日期
    pub activity: String,       // 活动内容
    pub summary: String,       // 简要总结
}

/// 专属回忆
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpecialMemory {
    pub title: String,         // 回忆标题
    pub description: String,    // 回忆描述
    pub date: String,          // 发生日期
    pub tags: Vec<String>,     // 标签
}

/// 角色列表项 (不含完整prompt)
#[derive(Debug, Serialize, Deserialize)]
pub struct CharacterBrief {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avatar: String,
    pub preferred_address: String,
}

/// 获取用户画像
#[tauri::command]
pub fn get_user_profile(character_id: String) -> Result<UserProfile, String> {
    ensure_character_dirs(&character_id)?;
    let profile_path = get_profile_dir(&character_id).join("user_profile.json");
    
    if !profile_path.exists() {
        return Ok(UserProfile::default());
    }
    
    let content = fs::read_to_string(&profile_path).map_err(|e| e.to_string())?;
    let profile: UserProfile = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    
    Ok(profile)
}

/// 保存用户画像
#[tauri::command]
pub fn save_user_profile(character_id: String, profile: UserProfile) -> Result<(), String> {
    ensure_character_dirs(&character_id)?;
    let profile_path = get_profile_dir(&character_id).join("user_profile.json");
    let content = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    fs::write(&profile_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 手动触发用户画像更新 (异步)
// Internal async function (not a command)
pub async fn trigger_profile_update(character_id: String, llm_manager: Arc<LLMManager>) -> Result<UserProfile, String> {
    use crate::llm::ChatMessage;

    // Get current profile
    let current_profile = get_user_profile(character_id.clone())?;

    // Collect all messages from all dates, sorted by timestamp
    let dates = crate::commands::memory::get_chat_dates(character_id.clone())
        .unwrap_or_default();

    // Compute actual conversation count from chat history (not from profile)
    let actual_count: u32 = dates.iter().map(|date| {
        crate::commands::memory::get_chat_by_date(character_id.clone(), date.clone())
            .map(|c| c.messages.iter().filter(|m| m.role == "user").count() as u32)
            .unwrap_or(0)
    }).sum();

    let mut all_messages: Vec<(i64, ChatMessage)> = Vec::new();
    for date in dates.iter() {
        if let Ok(day_chat) = crate::commands::memory::get_chat_by_date(character_id.clone(), date.clone()) {
            for msg in day_chat.messages {
                all_messages.push((msg.timestamp, ChatMessage {
                    role: msg.role,
                    content: msg.content,
                }));
            }
        }
    }

    // Sort by timestamp ascending
    all_messages.sort_by_key(|(ts, _)| *ts);

    // Take most recent 50 user messages
    let recent_user_messages: Vec<_> = all_messages
        .into_iter()
        .filter(|(_, m)| m.role == "user")
        .rev()
        .take(50)
        .collect();

    if recent_user_messages.is_empty() {
        return Err("暂无对话数据".to_string());
    }

    let new_chat_text: String = recent_user_messages.iter()
        .map(|(ts, m)| {
            let datetime = chrono::DateTime::from_timestamp(*ts, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "未知时间".to_string());
            let role_label = match m.role.as_str() {
                "user" => "我",
                "assistant" => "Bongo",
                _ => &m.role,
            };
            format!("[{}] {}: {}", datetime, role_label, m.content)
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Build profile summary for LLM
    let profile_summary = format!(
        r#"旧用户画像：
- 用户名: {}
- 性格特点: {}
- 偏好: {}
- 重要日期: {}
- 最近互动: {}
- 专属回忆: {}"#,
        current_profile.user_name.clone().unwrap_or_else(|| "未知".to_string()),
        if current_profile.traits.is_empty() { "暂无".to_string() } else { current_profile.traits.join(", ") },
        if current_profile.preferences.is_empty() { "暂无".to_string() } else {
            current_profile.preferences.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join(", ")
        },
        if current_profile.important_dates.is_empty() { "暂无".to_string() } else {
            current_profile.important_dates.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join(", ")
        },
        if current_profile.recent_interactions.is_empty() { "暂无".to_string() } else {
            current_profile.recent_interactions.iter().map(|i| format!("{}: {} - {}", i.date, i.activity, i.summary)).collect::<Vec<_>>().join("; ")
        },
        if current_profile.special_memories.is_empty() { "暂无".to_string() } else {
            current_profile.special_memories.iter().map(|m| format!("{}: {}", m.title, m.description)).collect::<Vec<_>>().join("; ")
        },
    );

    let prompt = format!(r#"请根据以下信息，更新用户画像。

## 旧用户画像（已有信息，保留并合并）
{}

## 新对话（分析这些对话，对旧画像进行更新和补充）
{}

请根据新对话更新用户画像，要求：
1. 保留旧画像中仍有价值的信息
2. 分析新对话，提炼新的性格特点、偏好、重要日期、互动和回忆
3. 如果旧信息与新对话矛盾，以新对话为准
4. 如果没有新对话中的某个维度信息，则保留旧画像中的对应内容
5. 注意：对话记录中已标注准确时间，输出的日期必须使用对话中实际出现的时间，不得使用"未知"等占位文字

请按以下JSON格式输出（只需输出JSON，不要其他内容）：
{{"user_name": "用户名或null", "traits": ["特点1", "特点2"], "preferences": {{"喜欢音乐": "古典音乐"}}, "important_dates": {{"生日": "06-15"}}, "recent_interactions": [{{"date": "2024-01-01", "activity": "一起听音乐", "summary": "用户分享了他喜欢的古典音乐"}}], "special_memories": [{{"title": "第一次聊天", "description": "用户第一次打开应用和我们聊天", "date": "2024-01-01", "tags": ["回忆"]}}]}}"#,
        profile_summary, new_chat_text);

    let messages = vec![ChatMessage::user(&prompt)];
    let response = llm_manager.chat_with_params(messages, 0.7, 2000).await.map_err(|e| e.to_string())?;

    println!("[profile] LLM raw response: {}", &response.content);

    // Parse and update profile
    if let Some(data) = extract_json(&response.content) {
        let mut profile = current_profile;

        profile.user_name = data.get("user_name").and_then(|v| v.as_str()).map(String::from);
        profile.traits = data.get("traits")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        if let Some(obj) = data.get("preferences").and_then(|v| v.as_object()) {
            profile.preferences.clear();
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    profile.preferences.insert(k.clone(), s.to_string());
                }
            }
        }

        if let Some(obj) = data.get("important_dates").and_then(|v| v.as_object()) {
            profile.important_dates.clear();
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    profile.important_dates.insert(k.clone(), s.to_string());
                }
            }
        }

        if let Some(arr) = data.get("recent_interactions").and_then(|v| v.as_array()) {
            profile.recent_interactions = arr.iter()
                .filter_map(|item| {
                    Some(Interaction {
                        date: item.get("date")?.as_str()?.to_string(),
                        activity: item.get("activity")?.as_str()?.to_string(),
                        summary: item.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    })
                })
                .collect();
        }

        if let Some(arr) = data.get("special_memories").and_then(|v| v.as_array()) {
            profile.special_memories = arr.iter()
                .filter_map(|item| {
                    Some(SpecialMemory {
                        title: item.get("title")?.as_str()?.to_string(),
                        description: item.get("description")?.as_str()?.to_string(),
                        date: item.get("date").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        tags: item.get("tags")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default(),
                    })
                })
                .collect();
        }

        profile.last_updated = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        profile.last_update_conversation_count = actual_count;

        save_user_profile(character_id, profile.clone())?;
        return Ok(profile);
    }

    Err("无法解析用户画像".to_string())
}

/// 触发用户画像更新 (Tauri command wrapper)
#[tauri::command]
pub async fn trigger_profile_update_command(
    character_id: String,
    llm_manager: State<'_, Arc<LLMManager>>,
) -> Result<UserProfile, String> {
    let profile_path = get_profile_dir(&character_id).join("user_profile.json");
    let profile_exists = profile_path.exists();

    // Count actual user messages from chat history
    let dates = crate::commands::memory::get_chat_dates(character_id.clone()).unwrap_or_default();
    let actual_count: u32 = dates.iter().map(|date| {
        crate::commands::memory::get_chat_by_date(character_id.clone(), date.clone())
            .map(|c| c.messages.iter().filter(|m| m.role == "user").count() as u32)
            .unwrap_or(0)
    }).sum();

    if profile_exists {
        let profile = get_user_profile(character_id.clone()).ok();
        if let Some(p) = profile {
            if actual_count == 0 || p.last_update_conversation_count >= actual_count {
                return Err("画像已是最新，无需更新".to_string());
            }
        }
    }

    trigger_profile_update(character_id, llm_manager.inner().clone()).await
}

/// 获取当前角色ID
#[tauri::command]
pub fn get_current_character() -> Result<String, String> {
    let config = crate::commands::config::load_config()?;
    Ok(config.characters.current)
}

/// 切换当前角色
#[tauri::command]
pub fn switch_character(id: String) -> Result<(), String> {
    // 验证角色存在
    let _ = crate::commands::config::load_character(id.clone())?;
    
    // 更新配置
    let mut config = crate::commands::config::load_config()?;
    config.characters.current = id;
    
    crate::commands::config::save_config(config)?;
    Ok(())
}

/// 获取简略角色列表 (不含prompt)
#[tauri::command]
pub fn list_character_briefs() -> Result<Vec<CharacterBrief>, String> {
    let characters = crate::commands::config::list_characters()?;
    
    Ok(characters.into_iter().map(|c| CharacterBrief {
        id: c.id,
        name: c.name,
        description: c.description,
        avatar: c.avatar,
        preferred_address: c.preferred_address,
    }).collect())
}

/// 获取当前角色的音色 ID
#[tauri::command]
pub fn get_current_character_voice_id() -> Result<Option<String>, String> {
    let config = crate::commands::config::load_config()?;
    let current_char_id = config.characters.current;
    let char_config = crate::commands::config::load_character(current_char_id)?;
    Ok(char_config.voice_id)
}