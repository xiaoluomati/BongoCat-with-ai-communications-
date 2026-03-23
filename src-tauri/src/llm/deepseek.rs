//! DeepSeek LLM Client Implementation

use crate::llm::types::{ChatMessage, ChatRequest, ChatResponse};
use crate::llm::LLMError;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const API_URL: &str = "https://api.deepseek.com/v1/chat/completions";

/// Log the request to a local file for debugging
fn log_request(request: &DeepSeekRequest) -> Result<(), std::io::Error> {
    // Get project root (workspace folder)
    let log_dir = PathBuf::from("logs");
    fs::create_dir_all(&log_dir)?;
    
    let log_file = log_dir.join("api_requests.jsonl");
    
    // Create timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    
    // Create log entry
    let log_entry = serde_json::json!({
        "timestamp": timestamp,
        "datetime": chrono::Local::now().to_rfc3339(),
        "provider": "deepseek",
        "url": API_URL,
        "request": request
    });
    
    // Append to file
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    
    writeln!(file, "{}", log_entry.to_string())?;
    
    Ok(())
}

#[derive(Debug, Serialize)]
struct DeepSeekRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeepSeekResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Usage {
    #[serde(rename = "prompt_tokens")]
    prompt_tokens: u32,
    #[serde(rename = "completion_tokens")]
    completion_tokens: u32,
    #[serde(rename = "total_tokens")]
    total_tokens: u32,
}

/// Stream chunk from DeepSeek API
#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Option<Vec<StreamChoice>>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Option<StreamDelta>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

pub struct DeepSeekClient {
    client: Client,
    api_key: String,
    model: String,
}

impl DeepSeekClient {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "deepseek-chat".to_string()),
        }
    }

    /// Non-streaming chat
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let deepseek_request = DeepSeekRequest {
            model: request.model,
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
        };

        // Log the request before sending
        if let Err(e) = log_request(&deepseek_request) {
            eprintln!("[DeepSeek] Failed to log request: {}", e);
        }

        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&deepseek_request)
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(format!(
                "DeepSeek API error ({}): {}",
                status,
                error_text
            )));
        }

        let deepseek_response: DeepSeekResponse = response.json().await?;

        let content = deepseek_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(ChatResponse {
            content,
            model: self.model.clone(),
        })
    }

    /// Streaming chat - yields content chunks via callback
    pub async fn chat_stream<F>(&self, request: ChatRequest, mut on_chunk: F) -> Result<ChatResponse, LLMError>
    where
        F: FnMut(String) + Send,
    {
        let deepseek_request = DeepSeekRequest {
            model: request.model.clone(),
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: true,
        };

        // Log the request before sending
        if let Err(e) = log_request(&deepseek_request) {
            eprintln!("[DeepSeek] Failed to log request: {}", e);
        }

        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&deepseek_request)
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(format!(
                "DeepSeek API error ({}): {}",
                status,
                error_text
            )));
        }

        // Process streaming response
        let mut stream = response.bytes_stream();
        let mut full_content = String::new();
        let mut buffer = String::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        buffer.push_str(&text);
                        
                        // Process complete chunks (lines starting with "data: ")
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer.drain(..newline_pos + 1).collect::<String>();
                            
                            if line.starts_with("data: ") {
                                let data = line.trim_start_matches("data: ");
                                
                                // Skip [DONE] message
                                if data == "[DONE]" {
                                    continue;
                                }
                                
                                // Try to parse as stream chunk
                                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                                    if let Some(choices) = chunk.choices {
                                        for choice in choices {
                                            if let Some(delta) = choice.delta {
                                                if let Some(content) = delta.content {
                                                    on_chunk(content.clone());
                                                    full_content.push_str(&content);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(LLMError::Network(e));
                }
            }
        }

        Ok(ChatResponse {
            content: full_content,
            model: self.model.clone(),
        })
    }

    pub async fn is_available(&self) -> bool {
        let request = ChatRequest::new(&self.model, vec![ChatMessage::user("ping")])
            .with_params(0.0, 1);

        self.chat(request).await.is_ok()
    }
}
