//! Unit tests for Prompt module

#[cfg(test)]
mod tests {
    use bongo_cat_lib::commands::prompt::{
        ComponentConfig, PromptComponents, PromptTemplate, RolePreset, SummaryPrompts,
    };

    #[test]
    fn test_prompt_template_creation() {
        let template = PromptTemplate {
            template_version: "1.0".to_string(),
            system_prompt_template: "{role_preset}".to_string(),
            components: PromptComponents {
                role_preset: ComponentConfig {
                    enabled: true,
                    description: "角色预设".to_string(),
                    prompt: None,
                    max_weeks: None,
                    max_messages: None,
                },
                user_profile: ComponentConfig {
                    enabled: true,
                    description: "用户画像".to_string(),
                    prompt: Some("## 用户信息".to_string()),
                    max_weeks: None,
                    max_messages: None,
                },
                long_term_memory: ComponentConfig {
                    enabled: true,
                    description: "长期记忆".to_string(),
                    prompt: Some("## 记忆".to_string()),
                    max_weeks: Some(4),
                    max_messages: None,
                },
                short_term_memory: ComponentConfig {
                    enabled: true,
                    description: "短期记忆".to_string(),
                    prompt: Some("## 对话".to_string()),
                    max_weeks: None,
                    max_messages: Some(20),
                },
            },
            summary_prompts: SummaryPrompts {
                weekly: "请分析周对话".to_string(),
                monthly: "请分析月对话".to_string(),
            },
        };

        assert_eq!(template.template_version, "1.0");
        assert_eq!(template.components.role_preset.enabled, true);
        assert_eq!(template.components.long_term_memory.max_weeks, Some(4));
    }

    #[test]
    fn test_role_preset_creation() {
        let preset = RolePreset {
            id: "cat_default".to_string(),
            name: "猫咪默认".to_string(),
            description: "Bongo Cat 角色的默认预设".to_string(),
            prompt: "你是一只可爱的猫咪...".to_string(),
        };

        assert_eq!(preset.id, "cat_default");
        assert_eq!(preset.name, "猫咪默认");
        assert!(preset.prompt.contains("猫咪"));
    }

    #[test]
    fn test_role_preset_serialization() {
        let preset = RolePreset {
            id: "test_preset".to_string(),
            name: "测试角色".to_string(),
            description: "这是一个测试角色".to_string(),
            prompt: "你是测试角色".to_string(),
        };

        let json = serde_json::to_string(&preset).unwrap();
        assert!(json.contains("test_preset"));
        assert!(json.contains("测试角色"));

        let deserialized: RolePreset = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test_preset");
    }

    #[test]
    fn test_prompt_template_serialization() {
        let template = PromptTemplate {
            template_version: "1.0".to_string(),
            system_prompt_template: "{role_preset}\n{user_profile}".to_string(),
            components: PromptComponents {
                role_preset: ComponentConfig {
                    enabled: true,
                    description: "角色".to_string(),
                    prompt: None,
                    max_weeks: None,
                    max_messages: None,
                },
                user_profile: ComponentConfig {
                    enabled: true,
                    description: "用户".to_string(),
                    prompt: Some("## 用户信息".to_string()),
                    max_weeks: None,
                    max_messages: None,
                },
                long_term_memory: ComponentConfig {
                    enabled: false,
                    description: "".to_string(),
                    prompt: None,
                    max_weeks: None,
                    max_messages: None,
                },
                short_term_memory: ComponentConfig {
                    enabled: false,
                    description: "".to_string(),
                    prompt: None,
                    max_weeks: None,
                    max_messages: None,
                },
            },
            summary_prompts: SummaryPrompts {
                weekly: "weekly prompt".to_string(),
                monthly: "monthly prompt".to_string(),
            },
        };

        let json = serde_json::to_string_pretty(&template).unwrap();
        assert!(json.contains("1.0"));
        assert!(json.contains("weekly prompt"));

        let deserialized: PromptTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.template_version, "1.0");
    }
}
