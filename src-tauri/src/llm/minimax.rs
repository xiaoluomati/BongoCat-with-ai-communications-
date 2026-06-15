//! Minimax LLM Client Implementation

use crate::llm::types::{ChatMessage, ChatRequest, ChatResponse};
use crate::llm::LLMError;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_URL: &str = "https://api.minimaxi.com/v1/text/chatcompletion_v2";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MinimaxMessage {
    role: String,
    content: String,
}

impl From<ChatMessage> for MinimaxMessage {
    fn from(msg: ChatMessage) -> Self {
        Self {
            role: msg.role,
            content: msg.content,
        }
    }
}

#[derive(Debug, Serialize)]
struct MinimaxRequest {
    model: String,
    messages: Vec<MinimaxMessage>,
    temperature: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct MinimaxResponse {
    choices: Vec<MinimaxChoice>,
}

#[derive(Debug, Deserialize)]
struct MinimaxChoice {
    message: MinimaxMessage,
}

/// Stream chunk from Minimax API
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

pub struct MinimaxClient {
    client: Client,
    api_key: String,
    model: String,
}

impl MinimaxClient {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            api_key,
            model: model.unwrap_or_else(|| "M2-her".to_string()),
        }
    }

    /// Non-streaming chat
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, LLMError> {
        let minimax_request = MinimaxRequest {
            model: request.model,
            messages: request.messages.into_iter().map(|m| m.into()).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
        };

        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&minimax_request)
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(format!(
                "Minimax API error ({}): {}",
                status,
                error_text
            )));
        }

        let minimax_response: MinimaxResponse = response.json().await?;

        let content = minimax_response
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
        let minimax_request = MinimaxRequest {
            model: request.model.clone(),
            messages: request.messages.into_iter().map(|m| m.into()).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: true,
        };

        let response = self
            .client
            .post(API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&minimax_request)
            .send()
            .await?;

        let status = response.status();
        
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LLMError::Api(format!(
                "Minimax API error ({}): {}",
                status,
                error_text
            )));
        }

        // Process streaming response with SSE-aware parsing.
        // Events are delimited by double newlines per SSE spec.
        let mut stream = response.bytes_stream();
        let mut full_content = String::new();
        let mut buffer = String::new();

        while let Some(item) = stream.next().await {
            match item {
                Ok(bytes) => {
                    match String::from_utf8(bytes.to_vec()) {
                        Ok(text) => buffer.push_str(&text),
                        Err(e) => {
                            log::warn!("[MiniMax] invalid UTF-8 in stream: {}", e);
                            continue;
                        }
                    };

                    // Process complete SSE events (delimited by \n\n)
                    while let Some(event_end) = buffer.find("\n\n") {
                        let event = buffer.drain(..event_end + 2).collect::<String>();

                        for line in event.lines() {
                            let data = match line.strip_prefix("data: ") {
                                Some(d) => d,
                                None => continue,
                            };

                            if data == "[DONE]" {
                                break;
                            }

                            match serde_json::from_str::<StreamChunk>(data) {
                                Ok(chunk) => {
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
                                Err(e) => {
                                    let preview = if data.len() > 200 {
                                        format!("{}...", &data[..200])
                                    } else {
                                        data.to_string()
                                    };
                                    log::error!("[MiniMax] SSE parse error: {}, data: {}", e, preview);
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

        // Flush any remaining buffer (incomplete event at end of stream)
        let remaining = buffer.trim().to_string();
        if !remaining.is_empty() {
            log::warn!("[MiniMax] unprocessed stream tail ({} chars): {}", remaining.len(), remaining);
        }

        Ok(ChatResponse {
            content: full_content,
            model: self.model.clone(),
        })
    }

    pub async fn is_available(&self) -> bool {
        // Simple check - try a minimal request
        let request = ChatRequest::new(&self.model, vec![ChatMessage::user("ping")])
            .with_params(0.0, 1);

        self.chat(request).await.is_ok()
    }
}
