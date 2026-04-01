//! Character Enhancement - User Profile & Extended Commands

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use chrono::Local;

fn get_data_dir() -> PathBuf {
    PathBuf::from("data")
}

fn get_profile_dir() -> PathBuf {
    get_data_dir().join("profile")
}

fn ensure_dirs() -> Result<(), String> {
    fs::create_dir_all(get_profile_dir()).map_err(|e| e.to_string())?;
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
    pub conversation_count: u32,
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
pub fn get_user_profile() -> Result<UserProfile, String> {
    ensure_dirs()?;
    let profile_path = get_profile_dir().join("user_profile.json");
    
    if !profile_path.exists() {
        return Ok(UserProfile::default());
    }
    
    let content = fs::read_to_string(&profile_path).map_err(|e| e.to_string())?;
    let profile: UserProfile = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    
    Ok(profile)
}

/// 保存用户画像
#[tauri::command]
pub fn save_user_profile(profile: UserProfile) -> Result<(), String> {
    ensure_dirs()?;
    let profile_path = get_profile_dir().join("user_profile.json");
    let content = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    fs::write(&profile_path, content).map_err(|e| e.to_string())?;
    Ok(())
}

/// 更新对话计数并检查是否需要更新画像
#[tauri::command]
pub fn check_and_update_profile(conversation_count: u32, force_update: bool) -> Result<bool, String> {
    let mut profile = get_user_profile()?;
    let old_count = profile.conversation_count;
    profile.conversation_count = conversation_count;
    
    // 检查是否需要更新: 强制更新 或 满50轮
    let needs_update = force_update || (conversation_count > 0 && (conversation_count - old_count) >= 50);
    
    if needs_update {
        profile.last_updated = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        save_user_profile(profile)?;
        return Ok(true);
    }
    
    Ok(false)
}

/// 手动触发用户画像更新 (异步)
// #[tauri::command]  // 移除 command 属性，作为内部函数使用
pub async fn trigger_profile_update(llm_manager: Arc<crate::llm::LLMManager>) -> Result<UserProfile, String> {
    use crate::llm::ChatMessage;
    
    // 获取最近对话
    let chat_result = crate::commands::memory::get_today_chat();
    let messages: Vec<_> = match chat_result {
        Ok(chat) => chat.messages.iter().take(50).cloned().collect(),
        Err(_) => vec![],
    };
    
    if messages.is_empty() {
        return Err("暂无对话数据".to_string());
    }
    
    let chat_text: String = messages.iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    
    let prompt = format!(r#"请从以下对话中分析用户的特点，生成用户画像。

要求：
1. 提取用户名称（如果有）
2. 分析用户性格特点
3. 提取用户偏好（如音乐、电影、游戏、饮食习惯等）
4. 标记重要日期（如生日、纪念日）
5. 分析最近互动（用户和AI一起做了什么有趣的事）
6. 提取专属回忆（用户分享的重要经历或特别时刻）

## 最近对话
{}

请按以下JSON格式输出：
{{"user_name": "用户名或null", "traits": ["特点1", "特点2"], "preferences": {{"喜欢音乐": "古典音乐"}}, "important_dates": {{"生日": "06-15", "纪念日": "2024-01-01"}}, "recent_interactions": [{{"date": "2024-01-01", "activity": "一起听音乐", "summary": "用户分享了他喜欢的古典音乐"}}, ...], "special_memories": [{{"title": "第一次聊天", "description": "用户第一次打开应用和我们聊天", "date": "2024-01-01", "tags": ["回忆", "第一次"]}}, ...]}}"#, 
        chat_text);
    
    let messages = vec![ChatMessage::user(&prompt)];
    let response = llm_manager.chat(messages).await.map_err(|e| e.to_string())?;
    
    // 解析响应
    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&response.content) {
        let mut profile = get_user_profile()?;
        
        profile.user_name = data.get("user_name").and_then(|v| v.as_str()).map(String::from);
        profile.traits = data.get("traits")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        
        if let Some(obj) = data.get("preferences").and_then(|v| v.as_object()) {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    profile.preferences.insert(k.clone(), s.to_string());
                }
            }
        }
        
        if let Some(obj) = data.get("important_dates").and_then(|v| v.as_object()) {
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    profile.important_dates.insert(k.clone(), s.to_string());
                }
            }
        }
        
        // 解析最近互动
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
        
        // 解析专属回忆
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
        
        save_user_profile(profile.clone())?;
        return Ok(profile);
    }
    
    Err("无法解析用户画像".to_string())
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
