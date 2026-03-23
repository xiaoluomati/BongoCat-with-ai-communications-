//! Unit tests for Memory module structures

#[cfg(test)]
mod tests {
    use bongo_cat_lib::commands::memory::{
        ChatMessage, DayChat, WeeklySummary, MonthlySummary,
    };
    
    #[test]
    fn test_chat_message_creation() {
        let msg = ChatMessage {
            id: "test_001".to_string(),
            role: "user".to_string(),
            content: "Hello".to_string(),
            timestamp: 1709875200,
        };
        
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }
    
    #[test]
    fn test_day_chat_creation() {
        let day_chat = DayChat {
            date: "2026-03-09".to_string(),
            messages: vec![
                ChatMessage {
                    id: "msg_1".to_string(),
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                    timestamp: 1000,
                },
                ChatMessage {
                    id: "msg_2".to_string(),
                    role: "assistant".to_string(),
                    content: "Hello!".to_string(),
                    timestamp: 2000,
                },
            ],
        };
        
        assert_eq!(day_chat.date, "2026-03-09");
        assert_eq!(day_chat.messages.len(), 2);
    }
    
    #[test]
    fn test_weekly_summary_creation() {
        let summary = WeeklySummary {
            week: "2026-W10".to_string(),
            date_range: "2026-03-03 ~ 2026-03-09".to_string(),
            keywords: vec!["忙碌".to_string(), "加班".to_string()],
            emotion_arc: vec!["平静".to_string(), "紧张".to_string()],
            summary: "本周工作较忙".to_string(),
            important_events: vec!["项目上线".to_string()],
            chat_count: 45,
        };
        
        assert_eq!(summary.week, "2026-W10");
        assert_eq!(summary.keywords.len(), 2);
    }
    
    #[test]
    fn test_monthly_summary_creation() {
        use std::collections::HashMap;
        
        let mut emotion_dist = HashMap::new();
        emotion_dist.insert("happy".to_string(), 40);
        emotion_dist.insert("tired".to_string(), 30);
        
        let summary = MonthlySummary {
            month: "2026-03".to_string(),
            emotion_distribution: emotion_dist,
            topics: vec!["工作".to_string(), "生活".to_string()],
            relationship_growth: "关系加深".to_string(),
            milestones: vec!["第一次聊天".to_string()],
        };
        
        assert_eq!(summary.month, "2026-03");
        assert_eq!(summary.topics.len(), 2);
    }
    
    #[test]
    fn test_message_serialization() {
        let msg = ChatMessage {
            id: "test_001".to_string(),
            role: "user".to_string(),
            content: "Hello".to_string(),
            timestamp: 1709875200,
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("test_001"));
        assert!(json.contains("user"));
        assert!(json.contains("Hello"));
    }
    
    #[test]
    fn test_day_chat_serialization() {
        let day_chat = DayChat {
            date: "2026-03-09".to_string(),
            messages: vec![
                ChatMessage {
                    id: "msg_1".to_string(),
                    role: "user".to_string(),
                    content: "Hi".to_string(),
                    timestamp: 1000,
                },
            ],
        };
        
        let json = serde_json::to_string_pretty(&day_chat).unwrap();
        assert!(json.contains("2026-03-09"));
        assert!(json.contains("Hi"));
    }
}
