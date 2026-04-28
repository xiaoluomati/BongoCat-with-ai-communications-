//! Configuration Tests

#[cfg(test)]
mod tests {
    use bongo_cat_lib::llm::{LLMConfig, LLMProvider};
    use bongo_cat_lib::commands::config::TTSConfig;

    #[test]
    fn test_tts_config_default() {
        let config = TTSConfig::default();
        assert_eq!(config.enabled, false);
        assert_eq!(config.base_url, "http://localhost:9880");
        assert_eq!(config.default_voice_id, "suyao");
    }

    #[test]
    fn test_llm_config_default() {
        let config = LLMConfig::default();
        assert_eq!(config.provider, LLMProvider::DeepSeek);
        assert_eq!(config.model, "deepseek-v4-flash");
        assert_eq!(config.temperature, 0.8);
    }

    #[test]
    fn test_tts_config_serialization() {
        let config = TTSConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: TTSConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.base_url, config.base_url);
        assert_eq!(deserialized.default_voice_id, config.default_voice_id);
    }

    #[test]
    fn test_llm_provider_display() {
        assert_eq!(format!("{}", LLMProvider::DeepSeek), "deepseek");
        assert_eq!(format!("{}", LLMProvider::Minimax), "minimax");
        assert_eq!(format!("{}", LLMProvider::LlamaCpp), "llama.cpp");
    }

    #[test]
    fn test_llm_config_serialization() {
        let config = LLMConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LLMConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.provider, config.provider);
        assert_eq!(deserialized.model, config.model);
        assert_eq!(deserialized.temperature, config.temperature);
    }
}
