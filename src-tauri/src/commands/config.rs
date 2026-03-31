//! Configuration Management Commands

use crate::llm::LLMConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm: LLMConfigData,
    pub tts: TTSConfig,
    pub memory: MemoryConfig,
    pub characters: CharactersConfig,
    pub chat: ChatConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatConfig {
    pub enabled: bool,
    pub max_messages: u32,
    pub window_width: u32,
    pub window_height: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LLMConfigData {
    pub provider: String,
    pub deepseek: ProviderConfig,
    pub minimax: MinimaxConfig,
    pub ollama: ProviderConfig,
    pub temperature: f32,
    pub max_tokens: u32,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MinimaxConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TTSConfig {
    pub enabled: bool,
    pub provider: String,
    pub indextts2: IndexTTS2Config,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexTTS2Config {
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub enabled: bool,
    pub retention_days: u32,
    pub context_weeks: u32,
    pub auto_summary: bool,
    pub profile_update_interval: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CharactersConfig {
    pub current: String,
}

fn get_data_dir() -> PathBuf {
    PathBuf::from("data")
}

fn get_config_path() -> PathBuf {
    get_data_dir().join("config.json")
}

fn load_config_sync() -> Result<AppConfig, String> {
    let config_path = get_config_path();
    
    if !config_path.exists() {
        let resource_config = PathBuf::from("data/config.json");
        if resource_config.exists() {
            let content = fs::read_to_string(&resource_config).map_err(|e| e.to_string())?;
            return serde_json::from_str(&content).map_err(|e| e.to_string());
        }
        return Err("Config file not found".to_string());
    }
    
    let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_config() -> Result<AppConfig, String> {
    load_config_sync()
}

#[tauri::command]
pub fn save_config(config: AppConfig) -> Result<(), String> {
    let config_path = get_config_path();
    
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub fn get_llm_config() -> Result<LLMConfig, String> {
    let config = load_config_sync()?;
    
    let provider = match config.llm.provider.as_str() {
        "deepseek" => crate::llm::LLMProvider::DeepSeek,
        "minimax" => crate::llm::LLMProvider::Minimax,
        "ollama" => crate::llm::LLMProvider::Ollama,
        _ => crate::llm::LLMProvider::DeepSeek,
    };
    
    let (api_key, base_url, model) = match provider {
        crate::llm::LLMProvider::DeepSeek => (
            config.llm.deepseek.api_key,
            config.llm.deepseek.base_url,
            config.llm.deepseek.model,
        ),
        crate::llm::LLMProvider::Minimax => (
            config.llm.minimax.api_key,
            "https://api.minimax.chat/v1".to_string(),
            config.llm.minimax.model,
        ),
        crate::llm::LLMProvider::Ollama => (
            config.llm.ollama.api_key,
            config.llm.ollama.base_url,
            config.llm.ollama.model,
        ),
    };
    
    Ok(LLMConfig {
        provider,
        api_key,
        base_url,
        model,
        temperature: config.llm.temperature,
        max_tokens: config.llm.max_tokens,
        stream: config.llm.stream,
    })
}

// ============ Character Management ============

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub description: String,
    pub avatar: String,
    pub preferred_address: String,
    pub system_prompt: String,
    pub preset_prompt: String,
}

fn get_characters_dir() -> PathBuf {
    PathBuf::from("data/characters")
}

fn get_default_characters_dir() -> PathBuf {
    PathBuf::from("data/characters")
}

#[tauri::command]
pub fn load_character(id: String) -> Result<Character, String> {
    let user_path = get_characters_dir().join(&id).join("config.json");
    
    let config_path = if user_path.exists() {
        user_path
    } else {
        get_default_characters_dir().join(&id).join("config.json")
    };
    
    if !config_path.exists() {
        return Err(format!("Character not found: {}", id));
    }
    
    let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_characters() -> Result<Vec<Character>, String> {
    let mut characters = Vec::new();
    
    let user_dir = get_characters_dir();
    if user_dir.exists() {
        for entry in fs::read_dir(&user_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            if path.is_dir() {
                let config_path = path.join("config.json");
                if config_path.exists() {
                    if let Ok(content) = fs::read_to_string(&config_path) {
                        if let Ok(character) = serde_json::from_str::<Character>(&content) {
                            characters.push(character);
                        }
                    }
                }
            }
        }
    }
    
    let default_dir = get_default_characters_dir();
    if default_dir.exists() {
        for entry in fs::read_dir(&default_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            if path.is_dir() {
                let config_path = path.join("config.json");
                if config_path.exists() {
                    if let Ok(content) = fs::read_to_string(&config_path) {
                        if let Ok(character) = serde_json::from_str::<Character>(&content) {
                            if !characters.iter().any(|c: &Character| c.id == character.id) {
                                characters.push(character);
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(characters)
}

#[tauri::command]
pub fn save_character(character: Character) -> Result<(), String> {
    let characters_dir = get_characters_dir();
    fs::create_dir_all(&characters_dir).map_err(|e| e.to_string())?;
    
    let character_dir = characters_dir.join(&character.id);
    fs::create_dir_all(&character_dir).map_err(|e| e.to_string())?;
    
    let config_path = character_dir.join("config.json");
    let content = serde_json::to_string_pretty(&character).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub fn delete_character(id: String) -> Result<(), String> {
    let character_dir = get_characters_dir().join(&id);
    
    if !character_dir.exists() {
        return Err(format!("Character not found: {}", id));
    }
    
    fs::remove_dir_all(&character_dir).map_err(|e| e.to_string())?;
    
    Ok(())
}
