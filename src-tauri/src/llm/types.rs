//! LLM Type Definitions

use serde::{Deserialize, Serialize};

/// Chat message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }
}

/// Chat request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
}

impl ChatRequest {
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: 0.8,
            max_tokens: 500,
        }
    }

    pub fn with_params(mut self, temperature: f32, max_tokens: u32) -> Self {
        self.temperature = temperature;
        self.max_tokens = max_tokens;
        self
    }
}

/// Chat response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
}

/// LLM Provider type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    DeepSeek,
    Minimax,
    Kimi,
}

impl Default for LLMProvider {
    fn default() -> Self {
        Self::DeepSeek
    }
}

impl std::fmt::Display for LLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LLMProvider::DeepSeek => write!(f, "deepseek"),
            LLMProvider::Minimax => write!(f, "minimax"),
            LLMProvider::Kimi => write!(f, "kimi"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_creation() {
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = ChatMessage::assistant("Hi there!");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content, "Hi there!");

        let system_msg = ChatMessage::system("You are a helpful assistant");
        assert_eq!(system_msg.role, "system");
        assert_eq!(system_msg.content, "You are a helpful assistant");
    }

    #[test]
    fn test_chat_request_creation() {
        let messages = vec![
            ChatMessage::system("You are a cat"),
            ChatMessage::user("Meow"),
        ];
        
        let request = ChatRequest::new("deepseek-chat", messages);
        
        assert_eq!(request.model, "deepseek-chat");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, 0.8);
        assert_eq!(request.max_tokens, 500);
    }

    #[test]
    fn test_chat_request_with_params() {
        let messages = vec![ChatMessage::user("Test")];
        let request = ChatRequest::new("deepseek-chat", messages)
            .with_params(0.5, 200);
        
        assert_eq!(request.temperature, 0.5);
        assert_eq!(request.max_tokens, 200);
    }
}
