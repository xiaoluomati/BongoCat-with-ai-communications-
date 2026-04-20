//! Llama.cpp LLM Client Implementation
//! Compatible with llama.cpp server's OpenAI-compatible HTTP API

use crate::llm::types::{ChatMessage, ChatRequest, ChatResponse};
use crate::llm::LLMError;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const API_URL: &str = "http://localhost:8080";
const CHAT_API_PATH: &str = "/v1/chat/completions";
const MODELS_API_PATH: &str = "/api/tags";

#[derive(Debug, Serialize)]
struct LlamaCppRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct LlamaCppResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

/// Stream chunk from Llama.cpp API
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

/// Model info from /api/tags (llama.cpp format)
#[derive(Debug, Deserialize)]
struct LlamaCppModelsResponse {
    models: Vec<LlamaCppModelInfo>,
}

#[derive(Debug, Deserialize)]
struct LlamaCppModelInfo {
    #[serde(rename = "name")]
    name: String,
}

pub struct LlamaCppClient {
    client: Client,
    base_url: String,
    model: String,
}

impl LlamaCppClient {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| API_URL.to_string()),
            model: model.unwrap_or_else(|| "llama-3.2-1b-instruct-q4_k_m.gguf".to_string()),
        }
    }

    /// List available models from llama.cpp
    pub async fn list_models(&self) -> Result<Vec<String>, LLMError> {
        let url = format!("{}{}", self.base_url, MODELS_API_PATH);
        
        let response = self
            .client
            .get(&url)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(format!(
                "Llama.cpp API error ({}): {}",
                status,
                error_text
            )));
        }

        let result: LlamaCppModelsResponse = response.json().await?;
        
        let model_names = result
            .models
            .into_iter()
            .map(|m| m.name)
            .collect();

        Ok(model_names)
    }

    /// Non-streaming chat
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let llama_cpp_request = LlamaCppRequest {
            model: request.model,
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
        };

        let url = format!("{}{}", self.base_url, CHAT_API_PATH);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&llama_cpp_request)
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(format!(
                "Llama.cpp API error ({}): {}",
                status,
                error_text
            )));
        }

        let llama_cpp_response: LlamaCppResponse = response.json().await?;

        let content = llama_cpp_response
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
        let llama_cpp_request = LlamaCppRequest {
            model: request.model.clone(),
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: true,
        };

        let url = format!("{}{}", self.base_url, CHAT_API_PATH);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&llama_cpp_request)
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(format!(
                "Llama.cpp API error ({}): {}",
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
        use std::time::Duration;
        // 超时2秒，最多重试3次，间隔1秒
        for attempt in 0..3 {
            match tokio::time::timeout(Duration::from_secs(2), self.list_models()).await {
                Ok(Ok(_)) => return true,
                _ => {
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
        false
    }
}
