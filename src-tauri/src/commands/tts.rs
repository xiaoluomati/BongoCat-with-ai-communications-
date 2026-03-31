//! TTS (Text-to-Speech) Commands

use crate::commands::config::{load_config, VoiceConfig};
use std::fs;
use std::path::PathBuf;

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
        // Return default if voice not found
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
    
    // Cannot delete default voice
    if voice_id == config.tts.default_voice_id {
        return Err("Cannot delete the default voice".to_string());
    }
    
    config.tts.voices.remove(&voice_id);
    crate::commands::save_config(config)
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
fn generate_cache_key(text: &str, speaker: &str, emo: &str, weight: f32) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let input = format!("{}|{}|{}|{}", text, speaker, emo, weight);
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}.wav", hasher.finish())
}

/// Speak text using TTS
/// This is called after an LLM response is generated
#[tauri::command]
pub async fn tts_speak(
    text: String,
    voice_id: Option<String>,
    _app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // Get TTS config
    let config = load_config()?;
    let tts_config = &config.tts;
    
    if !tts_config.enabled {
        return Ok(()); // TTS disabled, do nothing
    }
    
    if text.is_empty() {
        return Ok(()); // No text to speak
    }
    
    // Get voice configuration
    let voice = if let Some(ref vid) = voice_id {
        tts_config.voices.get(vid).cloned()
    } else {
        None
    };
    
    let (speaker, emo, weight): (String, String, f32) = if let Some(v) = voice {
        (v.speaker, v.emo, v.weight)
    } else {
        // Use default voice
        tts_config.voices.get(&tts_config.default_voice_id)
            .map(|v| (v.speaker.clone(), v.emo.clone(), v.weight))
            .unwrap_or_else(|| {
                (VoiceConfig::default().speaker, VoiceConfig::default().emo, VoiceConfig::default().weight)
            })
    };
    
    // Check cache first
    let cache_dir = get_tts_cache_dir();
    let cache_key = generate_cache_key(&text, &speaker, &emo, weight);
    let cache_path = cache_dir.join(&cache_key);
    
    if cache_path.exists() {
        // Play from cache
        play_audio_file(&cache_path).await?;
        return Ok(());
    }
    
    // Call IndePTTS2 API
    let url = format!(
        "{}?text={}&speaker={}&emo={}&weight={}",
        tts_config.base_url,
        urlencoding::encode(&text),
        urlencoding::encode(&speaker),
        urlencoding::encode(&emo),
        weight
    );
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    
    let response = client.get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("TTS API error ({}): {}", status, error_text));
    }
    
    // Save to cache
    let audio_bytes = response.bytes().await.map_err(|e| e.to_string())?;
    fs::write(&cache_path, &audio_bytes).map_err(|e| e.to_string())?;
    
    // Play audio
    play_audio_file(&cache_path).await?;
    
    Ok(())
}

/// Play an audio file
async fn play_audio_file(path: &PathBuf) -> Result<(), String> {
    // TODO: Implement actual audio playback
    // For now, just return success if file exists
    if path.exists() {
        Ok(())
    } else {
        Err(format!("Audio file not found: {:?}", path))
    }
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

/// Get TTS cache info (number of cached files and total size)
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
