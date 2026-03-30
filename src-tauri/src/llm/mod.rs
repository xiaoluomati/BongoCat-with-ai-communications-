//! LLM Module - Multi-Provider Implementation

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod deepseek;
pub mod minimax;
pub mod ollama;
pub mod types;

pub use deepseek::DeepSeekClient;
pub use minimax::MinimaxClient;
pub use ollama::OllamaClient;
pub use types::{ChatMessage, ChatRequest, ChatResponse, LLMProvider};

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("API error: {0}")]
    Api(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMConfig {
    pub provider: LLMProvider,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub stream: bool,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::DeepSeek,
            api_key: String::new(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            model: "deepseek-chat".to_string(),
            temperature: 0.8,
            max_tokens: 500,
            stream: false,
        }
    }
}

/// LLM Manager - manages different LLM clients
pub struct LLMManager {
    deepseek_client: Arc<RwLock<Option<DeepSeekClient>>>,
    minimax_client: Arc<RwLock<Option<MinimaxClient>>>,
    ollama_client: Arc<RwLock<Option<OllamaClient>>>,
    config: LLMConfig,
}

impl LLMManager {
    pub fn new(config: LLMConfig) -> Self {
        Self {
            deepseek_client: Arc::new(RwLock::new(None)),
            minimax_client: Arc::new(RwLock::new(None)),
            ollama_client: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Initialize the client based on current provider
    pub async fn init(&self) -> Result<(), LLMError> {
        match self.config.provider {
            LLMProvider::DeepSeek => {
                let client = DeepSeekClient::new(
                    self.config.api_key.clone(),
                    Some(self.config.model.clone()),
                );
                let mut lock = self.deepseek_client.write().await;
                *lock = Some(client);
            }
            LLMProvider::Minimax => {
                let client = MinimaxClient::new(
                    self.config.api_key.clone(),
                    Some(self.config.model.clone()),
                );
                let mut lock = self.minimax_client.write().await;
                *lock = Some(client);
            }
            LLMProvider::Ollama => {
                let client = OllamaClient::new(
                    Some(self.config.base_url.clone()),
                    Some(self.config.model.clone()),
                );
                let mut lock = self.ollama_client.write().await;
                *lock = Some(client);
            }
        }

        Ok(())
    }

    /// Check if LLM is available
    pub async fn is_available(&self) -> bool {
        match self.config.provider {
            LLMProvider::DeepSeek => {
                let lock = self.deepseek_client.read().await;
                if let Some(client) = lock.as_ref() {
                    client.is_available().await
                } else {
                    false
                }
            }
            LLMProvider::Minimax => {
                let lock = self.minimax_client.read().await;
                if let Some(client) = lock.as_ref() {
                    client.is_available().await
                } else {
                    false
                }
            }
            LLMProvider::Ollama => {
                let lock = self.ollama_client.read().await;
                if let Some(client) = lock.as_ref() {
                    client.is_available().await
                } else {
                    false
                }
            }
        }
    }

    /// Send chat request (non-streaming)
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatResponse, LLMError> {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        };

        match self.config.provider {
            LLMProvider::DeepSeek => {
                let lock = self.deepseek_client.read().await;
                let client = lock.as_ref().ok_or_else(|| {
                    LLMError::ProviderUnavailable("DeepSeek client not initialized".to_string())
                })?;
                client.chat(request).await
            }
            LLMProvider::Minimax => {
                let lock = self.minimax_client.read().await;
                let client = lock.as_ref().ok_or_else(|| {
                    LLMError::ProviderUnavailable("Minimax client not initialized".to_string())
                })?;
                client.chat(request).await
            }
            LLMProvider::Ollama => {
                let lock = self.ollama_client.read().await;
                let client = lock.as_ref().ok_or_else(|| {
                    LLMError::ProviderUnavailable("Ollama client not initialized".to_string())
                })?;
                client.chat(request).await
            }
        }
    }

    /// Send chat request with streaming
    pub async fn chat_stream<F>(&self, messages: Vec<ChatMessage>, on_chunk: F) -> Result<ChatResponse, LLMError>
    where
        F: FnMut(String) + Send + 'static,
    {
        let request = ChatRequest {
            model: self.config.model.clone(),
            messages,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
        };

        match self.config.provider {
            LLMProvider::DeepSeek => {
                let lock = self.deepseek_client.read().await;
                let client = lock.as_ref().ok_or_else(|| {
                    LLMError::ProviderUnavailable("DeepSeek client not initialized".to_string())
                })?;
                client.chat_stream(request, on_chunk).await
            }
            LLMProvider::Minimax => {
                let lock = self.minimax_client.read().await;
                let client = lock.as_ref().ok_or_else(|| {
                    LLMError::ProviderUnavailable("Minimax client not initialized".to_string())
                })?;
                client.chat_stream(request, on_chunk).await
            }
            LLMProvider::Ollama => {
                let lock = self.ollama_client.read().await;
                let client = lock.as_ref().ok_or_else(|| {
                    LLMError::ProviderUnavailable("Ollama client not initialized".to_string())
                })?;
                client.chat_stream(request, on_chunk).await
            }
        }
    }

    /// Check if streaming is enabled
    pub fn is_stream_enabled(&self) -> bool {
        self.config.stream
    }
}
