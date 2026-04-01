//! TTS (Text-to-Speech) Commands using IndexTTS API

use crate::commands::config::{load_config, VoiceConfig};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
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

/// TTS Meta for a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSMeta {
    pub audio_files: Vec<TTSAudioFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSAudioFile {
    pub seq: u32,
    pub path: String,
    pub text: String,
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
        .join("tts")
        .join("cache");
    
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).ok();
    }
    
    cache_dir
}

/// Get the archive directory for TTS audio files
fn get_tts_archive_dir() -> PathBuf {
    let archive_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("cache")
        .join("bongo-cat")
        .join("tts")
        .join("archive");
    
    if !archive_dir.exists() {
        fs::create_dir_all(&archive_dir).ok();
    }
    
    archive_dir
}

/// Get today's date string
fn get_today_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    
    let secs_per_day: u64 = 24 * 60 * 60;
    let days = now.as_secs() / secs_per_day;
    
    // Simple date calculation
    let mut year: u64 = 1970;
    let mut remaining_days = days;
    
    // Count years
    loop {
        let days_in_year: u64 = 365 + if is_leap_year(year) { 1 } else { 0 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }
    
    // Count months
    let days_in_months: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month: u64 = 1;
    for days_in_month in days_in_months.iter() {
        let dim = *days_in_month as u64;
        if is_leap_year(year) && month == 2 {
            if remaining_days < dim + 1 {
                break;
            }
            remaining_days -= dim + 1;
        } else if remaining_days < dim {
            break;
        } else {
            remaining_days -= dim;
        }
        month += 1;
    }
    
    let day = remaining_days + 1;
    
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Get archive path for a specific date
fn get_archive_date_dir(date: &str) -> PathBuf {
    let dir = get_tts_archive_dir().join(date);
    if !dir.exists() {
        fs::create_dir_all(&dir).ok();
    }
    dir
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

/// Save audio to archive and return local path
fn save_audio_to_archive(
    audio_bytes: &[u8],
    msg_id: &Option<String>,
    seq: Option<u32>,
) -> Result<String, String> {
    let date = get_today_date();
    let date_dir = get_archive_date_dir(&date);
    
    let filename = match (msg_id, seq) {
        (Some(id), Some(s)) => format!("{}_{:03}.wav", id, s),
        _ => {
            use uuid::Uuid;
            format!("{}.wav", Uuid::new_v4())
        }
    };
    
    let path = date_dir.join(&filename);
    fs::write(&path, audio_bytes).map_err(|e| e.to_string())?;
    
    Ok(path.to_string_lossy().to_string())
}

/// Speak text using TTS
/// Returns the audio URL for frontend to play
#[tauri::command]
pub async fn tts_speak(
    text: String,
    voice_id: Option<String>,
    msg_id: Option<String>,  // Message ID to associate with this audio
    seq: Option<u32>,       // Sequence number in the message
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
    
    // Check cache first (only for non-archived requests)
    if msg_id.is_none() {
        let cache_dir = get_tts_cache_dir();
        let cache_key = generate_cache_key(&text, &speaker, &emo, &emo_method, speed);
        let cache_path = cache_dir.join(format!("{}.wav", cache_key));
        
        if cache_path.exists() {
            return Ok(format!("file://{}", cache_path.to_string_lossy()));
        }
    }
    
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
    let audio_url = if let Some(output) = result.output {
        if let Some(file_data) = output.output_1 {
            if let Some(url) = file_data.url {
                url
            } else if let Some(path) = file_data.path {
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
    
    // Download and save to archive
    let audio_response = client.get(&audio_url).send().await.map_err(|e| e.to_string())?;
    let audio_bytes = audio_response.bytes().await.map_err(|e| e.to_string())?;
    
    // Save to archive and return local path
    let local_path = save_audio_to_archive(&audio_bytes, &msg_id, seq)?;
    
    // Also cache it for non-archived requests
    let cache_dir = get_tts_cache_dir();
    let cache_key = generate_cache_key(&text, &speaker, &emo, &emo_method, speed);
    let cache_path = cache_dir.join(format!("{}.wav", cache_key));
    let _ = fs::write(&cache_path, &audio_bytes);
    
    Ok(format!("file://{}", local_path))
}

/// Clear TTS cache (not archive)
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

/// Save TTS meta for a message
#[tauri::command]
pub async fn save_tts_meta(
    msg_id: String,
    date: String,
    audio_files: Vec<TTSAudioFile>,
) -> Result<(), String> {
    let archive_dir = get_archive_date_dir(&date);
    let meta_path = archive_dir.join("meta.json");
    
    // Load existing meta or create new
    let mut meta: HashMap<String, TTSMeta> = if meta_path.exists() {
        let content = fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    };
    
    // Update meta for this message
    meta.insert(msg_id.clone(), TTSMeta { audio_files: audio_files.clone() });
    
    // Write back
    let content = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(&meta_path, content).map_err(|e| e.to_string())?;
    
    Ok(())
}

/// Get TTS meta for a message
#[tauri::command]
pub async fn get_tts_meta(msg_id: String, date: String) -> Result<Option<TTSMeta>, String> {
    let archive_dir = get_archive_date_dir(&date);
    let meta_path = archive_dir.join("meta.json");
    
    if !meta_path.exists() {
        return Ok(None);
    }
    
    let content = fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
    let meta: HashMap<String, TTSMeta> = serde_json::from_str(&content).unwrap_or_default();
    
    Ok(meta.get(&msg_id).cloned())
}

/// Get TTS audio paths for replay
#[tauri::command]
pub async fn get_tts_replay_paths(msg_id: String, date: String) -> Result<Vec<String>, String> {
    let meta = get_tts_meta(msg_id, date).await?;
    
    match meta {
        Some(m) => Ok(m.audio_files.into_iter().map(|f| format!("file://{}", f.path)).collect()),
        None => Err("No TTS meta found for this message".to_string()),
    }
}

/// Internal helper to get current character's voice_id
pub fn get_current_character_voice_id_internal() -> Option<String> {
    let config = load_config().ok()?;
    let current_char_id = config.characters.current;
    let char_config = crate::commands::config::load_character(current_char_id).ok()?;
    char_config.voice_id
}
