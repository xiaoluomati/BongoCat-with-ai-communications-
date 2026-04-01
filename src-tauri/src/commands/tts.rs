//! TTS (Text-to-Speech) Commands using IndexTTS API

use crate::commands::config::{load_config, VoiceConfig};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

/// IndexTTS API response for submit_and_refresh
#[derive(Debug, Deserialize)]
struct SubmitResponse {
    output: Option<SubmitOutput>,
}

#[derive(Debug, Deserialize)]
struct SubmitOutput {
    #[serde(rename = "output_1")]
    output_1: Option<FileData>,
}

#[derive(Debug, Deserialize)]
struct FileData {
    path: Option<String>,
    url: Option<String>,
}

/// IndexTTS voice list response
#[derive(Debug, Deserialize)]
struct VoiceListResponse {
    output: Vec<String>,
}

/// IndexTTS emotion list response
#[derive(Debug, Deserialize)]
struct EmotionListResponse {
    output: Vec<String>,
}

/// Get TTS configuration
#[tauri::command]
pub async fn get_tts_config() -> Result<crate::commands::config::TTSConfig, String> {
    let config = load_config()?;
    Ok(config.tts)
}

/// Save TTS configuration
#[tauri::command]
pub async fn save_tts_config(tts_config: crate::commands::config::TTSConfig) -> Result<(), String> {
    let mut config = load_config()?;
    config.tts = tts_config;
    crate::commands::save_config(config)
}

/// Get voice configuration for a specific voice_id
#[tauri::command]
pub async fn get_voice_config(voice_id: String) -> Result<VoiceConfig, String> {
    let config = load_config()?;
    let tts_config = &config.tts;
    
    if let Some(voice) = tts_config.voices.get(&voice_id) {
        Ok(voice.clone())
    } else if voice_id == tts_config.default_voice_id {
        Ok(VoiceConfig::default())
    } else {
        Err(format!("Voice '{}' not found", voice_id))
    }
}

/// Add or update a voice in the registry
#[tauri::command]
pub async fn save_voice(voice_id: String, voice: VoiceConfig) -> Result<(), String> {
    let mut config = load_config()?;
    config.tts.voices.insert(voice_id, voice);
    crate::commands::save_config(config)
}

/// Delete a voice from the registry
#[tauri::command]
pub async fn delete_voice(voice_id: String) -> Result<(), String> {
    let mut config = load_config()?;
    if voice_id == config.tts.default_voice_id {
        return Err("Cannot delete the default voice".to_string());
    }
    config.tts.voices.remove(&voice_id);
    crate::commands::save_config(config)
}

/// Get available voices from IndexTTS server
#[tauri::command]
pub async fn get_index_tts_voices(base_url: String) -> Result<Vec<String>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    
    let url = format!("{}/run/update_voices", base_url);
    
    let response = client
        .post(&url)
        .json(&json!({}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to get voices: {}", response.status()));
    }
    
    let result: VoiceListResponse = response.json().await.map_err(|e| e.to_string())?;
    
    Ok(result.output)
}

/// Get available emotions from IndexTTS server
#[tauri::command]
pub async fn get_index_tts_emos(base_url: String) -> Result<Vec<String>, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    
    let url = format!("{}/run/update_emos", base_url);
    
    let response = client
        .post(&url)
        .json(&json!({}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to get emotions: {}", response.status()));
    }
    
    let result: EmotionListResponse = response.json().await.map_err(|e| e.to_string())?;
    
    Ok(result.output.into_iter().filter(|s| s != "请选择情绪").collect())
}

/// Get the cache directory for TTS audio files
fn get_tts_cache_dir() -> PathBuf {
    let cache_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cache")
        .join("bongo-cat")
        .join("tts");
    
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).ok();
    }
    
    cache_dir
}

/// Generate cache key from text and voice parameters
fn generate_cache_key(text: &str, speaker: &str, emo: &str, emo_method: &str, speed: f32) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let input = format!("{}|{}|{}|{}|{}", text, speaker, emo, emo_method, speed);
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Speak text using TTS
/// Returns the audio URL for frontend to play
#[tauri::command]
pub async fn tts_speak(
    text: String,
    voice_id: Option<String>,
    _app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let config = load_config()?;
    let tts_config = &config.tts;
    
    if !tts_config.enabled {
        return Err("TTS is disabled".to_string());
    }
    
    if text.is_empty() {
        return Err("Text is empty".to_string());
    }
    
    // Get voice configuration
    let voice = if let Some(ref vid) = voice_id {
        tts_config.voices.get(vid).cloned()
    } else {
        None
    };
    
    let (speaker, emo, emo_method, speed): (String, String, String, f32) = if let Some(v) = voice {
        (
            v.speaker,
            v.emo,
            v.emo_method.unwrap_or_else(|| "使用情感描述文本控制".to_string()),
            v.speed.unwrap_or(1.0),
        )
    } else {
        (
            tts_config.voices.get(&tts_config.default_voice_id)
                .map(|v| v.speaker.clone())
                .unwrap_or_else(|| "苏瑶".to_string()),
            tts_config.voices.get(&tts_config.default_voice_id)
                .map(|v| v.emo.clone())
                .unwrap_or_else(|| "高兴.wav".to_string()),
            tts_config.voices.get(&tts_config.default_voice_id)
                .map(|v| v.emo_method.clone().unwrap_or_else(|| "使用情感描述文本控制".to_string()))
                .unwrap_or_else(|| "使用情感描述文本控制".to_string()),
            tts_config.voices.get(&tts_config.default_voice_id)
                .map(|v| v.speed.unwrap_or(1.0))
                .unwrap_or(1.0),
        )
    };
    
    // Check cache first
    let cache_dir = get_tts_cache_dir();
    let cache_key = generate_cache_key(&text, &speaker, &emo, &emo_method, speed);
    let cache_path = cache_dir.join(format!("{}.wav", cache_key));
    
    let audio_url: String;
    
    if cache_path.exists() {
        // Use cached file
        audio_url = format!("file://{}", cache_path.to_string_lossy());
    } else {
        // Call IndexTTS API
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| e.to_string())?;
        
        let url = format!("{}/run/submit_and_refresh", tts_config.base_url);
        
        // Parse emo text (remove .wav extension if present)
        let emo_text = emo.replace(".wav", "");
        
        let request_body = json!({
            "voices_dropdown": speaker,
            "speed": speed,
            "text": text,
            "emo_control_method": emo_method,
            "emo_weight": 0.8,
            "emo_text": emo_text,
            "emo_random": false,
            "max_tokens": 100,
            "do_sample": true,
            "temperature": 0.7,
            "top_p": 0.9,
            "top_k": 50
        });
        
        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("TTS API error ({}): {}", status, error_text));
        }
        
        // Parse response to get audio URL
        let result: SubmitResponse = response.json().await.map_err(|e| e.to_string())?;
        
        // Construct audio URL from path or url
        audio_url = if let Some(output) = result.output {
            if let Some(file_data) = output.output_1 {
                if let Some(url) = file_data.url {
                    url
                } else if let Some(path) = file_data.path {
                    // Construct URL from path
                    format!("{}/file={}", tts_config.base_url.trim_end_matches('/'), path)
                } else {
                    return Err("No audio URL or path in response".to_string());
                }
            } else {
                return Err("No output_1 in response".to_string());
            }
        } else {
            return Err("No output in response".to_string());
        };
        
        // Try to download and cache the audio
        if let Ok(audio_response) = client.get(&audio_url).send().await {
            if let Ok(bytes) = audio_response.bytes().await {
                let _ = fs::write(&cache_path, &bytes);
            }
        }
    }
    
    Ok(audio_url)
}

/// Clear TTS cache
#[tauri::command]
pub async fn clear_tts_cache() -> Result<u64, String> {
    let cache_dir = get_tts_cache_dir();
    
    if !cache_dir.exists() {
        return Ok(0);
    }
    
    let mut deleted_count = 0u64;
    for entry in fs::read_dir(&cache_dir).map_err(|e| e.to_string())? {
        if let Ok(entry) = entry {
            if entry.path().extension().map_or(false, |ext| ext == "wav") {
                fs::remove_file(entry.path()).ok();
                deleted_count += 1;
            }
        }
    }
    
    Ok(deleted_count)
}

/// Get TTS cache info
#[tauri::command]
pub async fn get_tts_cache_info() -> Result<(u64, u64), String> {
    let cache_dir = get_tts_cache_dir();
    
    if !cache_dir.exists() {
        return Ok((0, 0));
    }
    
    let mut file_count = 0u64;
    let mut total_size = 0u64;
    
    for entry in fs::read_dir(&cache_dir).map_err(|e| e.to_string())? {
        if let Ok(entry) = entry {
            if entry.path().extension().map_or(false, |ext| ext == "wav") {
                if let Ok(metadata) = entry.metadata() {
                    file_count += 1;
                    total_size += metadata.len();
                }
            }
        }
    }
    
    Ok((file_count, total_size))
}

/// Internal helper to get current character's voice_id
pub fn get_current_character_voice_id_internal() -> Option<String> {
    let config = load_config().ok()?;
    let current_char_id = config.characters.current;
    let char_config = crate::commands::config::load_character(current_char_id).ok()?;
    char_config.voice_id
}
