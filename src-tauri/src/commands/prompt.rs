//! Prompt Builder - 构建对话提示词

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromptTemplate {
    #[serde(rename = "template_version")]
    pub template_version: String,
    #[serde(rename = "system_prompt_template")]
    pub system_prompt_template: String,
    #[serde(rename = "components")]
    pub components: PromptComponents,
    #[serde(rename = "summary_prompts")]
    pub summary_prompts: SummaryPrompts,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromptComponents {
    #[serde(rename = "role_preset")]
    pub role_preset: ComponentConfig,
    #[serde(rename = "user_profile")]
    pub user_profile: ComponentConfig,
    #[serde(rename = "long_term_memory")]
    pub long_term_memory: ComponentConfig,
    #[serde(rename = "short_term_memory")]
    pub short_term_memory: ComponentConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentConfig {
    #[serde(rename = "enabled")]
    pub enabled: bool,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "prompt")]
    pub prompt: Option<String>,
    #[serde(rename = "max_weeks")]
    pub max_weeks: Option<u32>,
    #[serde(rename = "max_messages")]
    pub max_messages: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SummaryPrompts {
    #[serde(rename = "weekly")]
    pub weekly: String,
    #[serde(rename = "monthly")]
    pub monthly: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RolePreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub prompt: String,
}

fn get_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("data")
}

fn get_presets_dir() -> PathBuf {
    get_data_dir().join("presets")
}

/// Load prompt template
#[tauri::command]
pub fn load_prompt_template() -> Result<PromptTemplate, String> {
    // First try user config
    let user_template_path = get_data_dir().join("prompt_template.json");
    
    let template_path = if user_template_path.exists() {
        user_template_path
    } else {
        // Fall back to default in resources
        PathBuf::from("data/prompt_template.json")
    };
    
    if !template_path.exists() {
        // Return default template
        return Ok(PromptTemplate {
            template_version: "1.0".to_string(),
            system_prompt_template: "{role_preset}\n\n{user_profile}\n\n{long_term_memory}\n\n{短期记忆}\n\n当前对话：".to_string(),
            components: PromptComponents {
                role_preset: ComponentConfig {
                    enabled: true,
                    description: "角色预设".to_string(),
                    prompt: None,
                    max_weeks: None,
                    max_messages: None,
                },
                user_profile: ComponentConfig {
                    enabled: true,
                    description: "用户画像".to_string(),
                    prompt: Some("## 用户信息\n{user_info}".to_string()),
                    max_weeks: None,
                    max_messages: None,
                },
                long_term_memory: ComponentConfig {
                    enabled: true,
                    description: "长期记忆".to_string(),
                    prompt: Some("## 近期记忆\n{memory_summary}".to_string()),
                    max_weeks: Some(4),
                    max_messages: None,
                },
                short_term_memory: ComponentConfig {
                    enabled: true,
                    description: "短期记忆".to_string(),
                    prompt: Some("## 今日对话\n{chat_history}".to_string()),
                    max_weeks: None,
                    max_messages: Some(20),
                },
            },
            summary_prompts: SummaryPrompts {
                weekly: "请分析以下一周的对话记录...".to_string(),
                monthly: "请分析本月的对话记录...".to_string(),
            },
        });
    }
    
    let content = fs::read_to_string(&template_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// Load role preset
#[tauri::command]
pub fn load_role_preset(preset_id: String) -> Result<RolePreset, String> {
    // First try user presets
    let user_preset_path = get_presets_dir().join(format!("{}.json", preset_id));
    
    let preset_path = if user_preset_path.exists() {
        user_preset_path
    } else {
        // Fall back to default in resources
        PathBuf::from("data/presets").join(format!("{}.json", preset_id))
    };
    
    if !preset_path.exists() {
        return Err(format!("Preset not found: {}", preset_id));
    }
    
    let content = fs::read_to_string(&preset_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// List available presets
#[tauri::command]
pub fn list_presets() -> Result<Vec<RolePreset>, String> {
    // List from user presets directory
    let presets_dir = get_presets_dir();
    
    let mut presets = Vec::new();
    
    if presets_dir.exists() {
        for entry in fs::read_dir(&presets_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
                if let Ok(preset) = serde_json::from_str::<RolePreset>(&content) {
                    presets.push(preset);
                }
            }
        }
    }
    
    // Also check default presets in resources
    let default_presets_dir = PathBuf::from("data/presets");
    if default_presets_dir.exists() {
        for entry in fs::read_dir(&default_presets_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
                if let Ok(preset) = serde_json::from_str::<RolePreset>(&content) {
                    // Avoid duplicates
                    if !presets.iter().any(|p| p.id == preset.id) {
                        presets.push(preset);
                    }
                }
            }
        }
    }
    
    Ok(presets)
}

/// Save role preset (to user directory)
#[tauri::command]
pub fn save_role_preset(preset: RolePreset) -> Result<(), String> {
    let presets_dir = get_presets_dir();
    fs::create_dir_all(&presets_dir).map_err(|e| e.to_string())?;
    
    let preset_path = presets_dir.join(format!("{}.json", preset.id));
    let content = serde_json::to_string_pretty(&preset).map_err(|e| e.to_string())?;
    fs::write(&preset_path, content).map_err(|e| e.to_string())?;
    
    Ok(())
}
