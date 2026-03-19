//! Scheduled Tasks - 定时任务

use crate::commands::memory::{get_chat_by_date, save_monthly_summary, save_weekly_summary, QuarterlySummary, YearlySummary, MonthlySummary, WeeklySummary};
use crate::llm::{ChatMessage, LLMManager};
use chrono::{Datelike, Local, NaiveDate};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;
use tokio::time::{interval, Duration};

const SUMMARY_CHECK_INTERVAL_MINUTES: u64 = 60;

/// 启动定时任务调度器
pub async fn start_scheduler(llm_manager: Arc<LLMManager>) {
    tokio::spawn(async move {
        let mut check_interval = interval(Duration::from_secs(SUMMARY_CHECK_INTERVAL_MINUTES * 60));
        
        loop {
            check_interval.tick().await;
            
            // 检查并生成周总结
            if should_generate_weekly_summary() {
                if let Err(e) = generate_weekly_summary(&llm_manager).await {
                    eprintln!("Failed to generate weekly summary: {}", e);
                }
            }
            
            // 检查并生成月总结
            if should_generate_monthly_summary() {
                if let Err(e) = generate_monthly_summary(&llm_manager).await {
                    eprintln!("Failed to generate monthly summary: {}", e);
                }
            }
            
            // 检查并生成季度总结
            if should_generate_quarterly_summary() {
                if let Err(e) = generate_quarterly_summary(&llm_manager).await {
                    eprintln!("Failed to generate quarterly summary: {}", e);
                }
            }
            
            // 检查并生成年度总结
            if should_generate_yearly_summary() {
                if let Err(e) = generate_yearly_summary(&llm_manager).await {
                    eprintln!("Failed to generate yearly summary: {}", e);
                }
            }
        }
    });
}

/// 生成周总结
async fn generate_weekly_summary(llm: &LLMManager) -> Result<(), String> {
    let (week_start, week_end, _week_label) = get_current_week_dates();
    let now = Local::now();
    let week_num = now.format("%W").to_string();
    let week_label = format!("{}-W{}", now.year(), week_num);
    
    // 收集本周所有对话
    let mut all_messages: Vec<String> = Vec::new();
    let start_date = NaiveDate::parse_from_str(&week_start, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    let end_date = NaiveDate::parse_from_str(&week_end, "%Y-%m-%d")
        .map_err(|e| e.to_string())?;
    
    let mut current = start_date;
    while current <= end_date {
        let date_str = current.format("%Y-%m-%d").to_string();
        if let Ok(day_chat) = get_chat_by_date(date_str) {
            for msg in day_chat.messages {
                all_messages.push(format!("{}: {}", msg.role, msg.content));
            }
        }
        current = current.succ_opt().unwrap_or(current);
    }
    
    if all_messages.is_empty() {
        return Ok(());
    }
    
    // 构建提示词
    let prompt = format!(r#"请分析以下一周的对话记录，生成简洁的周总结。

要求：
1. 提取 3-5 个关键词
2. 描述情绪变化曲线
3. 列出重要事件
4. 用 2-3 句话总结本周

## 对话记录
{}

请按以下 JSON 格式输出（只需输出 JSON，不要其他内容）：
{{"keywords": ["关键词1", "关键词2"], "emotion_arc": ["情绪1", "情绪2"], "summary": "总结内容", "important_events": ["事件1", "事件2"], "chat_count": {}}}"#, 
        all_messages.join("\n"),
        all_messages.len()
    );
    
    // 调用 LLM
    let messages = vec![ChatMessage::user(&prompt)];
    let response = llm.chat(messages).await.map_err(|e| e.to_string())?;
    
    // 解析 JSON 响应
    if let Ok(summary) = serde_json::from_str::<serde_json::Value>(&response.content) {
        let weekly_summary = WeeklySummary {
            week: week_label.clone(),
            date_range: format!("{} ~ {}", week_start, week_end),
            keywords: summary.get("keywords")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            emotion_arc: summary.get("emotion_arc")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            summary: summary.get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            important_events: summary.get("important_events")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            chat_count: summary.get("chat_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        };
        
        save_weekly_summary(weekly_summary)?;
        println!("Weekly summary generated for {}", week_label);
    }
    
    Ok(())
}

/// 生成月度总结
async fn generate_monthly_summary(llm: &LLMManager) -> Result<(), String> {
    let now = Local::now();
    let month_label = now.format("%Y-%m").to_string();
    
    // 收集本月所有对话
    let mut all_messages: Vec<String> = Vec::new();
    let year = now.year();
    let month = now.month();
    
    // 获取本月天数
    let days_in_month = if month == 12 {
        31
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .unwrap_or_default()
            .pred_opt()
            .unwrap_or_default()
            .day() as u32
    };
    
    for day in 1..=days_in_month {
        let date_str = format!("{:04}-{:02}-{:02}", year, month, day);
        if let Ok(day_chat) = get_chat_by_date(date_str.clone()) {
            for msg in day_chat.messages {
                all_messages.push(format!("{}: {}", msg.role, msg.content));
            }
        }
    }
    
    if all_messages.is_empty() {
        return Ok(());
    }
    
    // 构建提示词
    let prompt = format!(r#"请分析本月的对话记录，生成月度总结。

要求：
1. 统计情绪分布 (happy/tired/relaxed/other 百分比)
2. 总结主要话题
3. 描述关系变化
4. 列出里程碑事件

## 本月对话
{}

请按以下 JSON 格式输出（只需输出 JSON，不要其他内容）：
{{"emotion_distribution": {{"happy": 40, "tired": 30, "relaxed": 20, "other": 10}}, "topics": ["话题1", "话题2"], "relationship_growth": "关系变化描述", "milestones": ["里程碑1", "里程碑2"]}}"#,
        all_messages.join("\n")
    );
    
    // 调用 LLM
    let messages = vec![ChatMessage::user(&prompt)];
    let response = llm.chat(messages).await.map_err(|e| e.to_string())?;
    
    // 解析 JSON 响应
    if let Ok(summary) = serde_json::from_str::<serde_json::Value>(&response.content) {
        let mut emotion_dist = HashMap::new();
        if let Some(emotion_obj) = summary.get("emotion_distribution").and_then(|v| v.as_object()) {
            for (key, value) in emotion_obj {
                if let Some(num) = value.as_i64() {
                    emotion_dist.insert(key.clone(), num as i32);
                }
            }
        }
        
        let monthly_summary = MonthlySummary {
            month: month_label.clone(),
            emotion_distribution: emotion_dist,
            topics: summary.get("topics")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            relationship_growth: summary.get("relationship_growth")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            milestones: summary.get("milestones")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        };
        
        save_monthly_summary(monthly_summary)?;
        println!("Monthly summary generated for {}", month_label);
    }
    
    Ok(())
}

/// 生成季度总结
async fn generate_quarterly_summary(llm: &LLMManager) -> Result<(), String> {
    let now = Local::now();
    let quarter = (now.month() - 1) / 3 + 1;
    let quarter_label = format!("{}-Q{}", now.year(), quarter);
    
    // 收集本季度所有月总结
    let monthly_summaries = crate::commands::memory::get_monthly_summaries()?;
    let summaries_text: Vec<String> = monthly_summaries.iter()
        .filter(|s| s.month.starts_with(&now.year().to_string()))
        .map(|s| format!("{}: {} - {}", s.month, s.relationship_growth, s.milestones.join(", ")))
        .collect();
    
    if summaries_text.is_empty() {
        return Ok(());
    }
    
    let prompt = format!(r#"请分析本季度的月度总结，生成季度报告。

要求：
1. 总结本季度的主要变化
2. 列出关键里程碑
3. 用简洁语言描述

## 月度总结
{}

请按以下 JSON 格式输出：
{{"keywords": [], "summary": "", "important_events": [], "milestone": ""}}"#, 
        summaries_text.join("\n"));
    
    let messages = vec![ChatMessage::user(&prompt)];
    let response = llm.chat(messages).await.map_err(|e| e.to_string())?;
    
    if let Ok(result) = serde_json::from_str::<serde_json::Value>(&response.content) {
        let q_summary = QuarterlySummary {
            quarter: quarter_label.clone(),
            date_range: format!("{}季度", quarter),
            keywords: result.get("keywords").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
            summary: result.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            important_events: result.get("important_events").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
            milestone: result.get("milestone").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        };
        
        crate::commands::memory::save_quarterly_summary(q_summary)?;
        println!("Quarterly summary generated for {}", quarter_label);
    }
    
    Ok(())
}

/// 生成年度总结
async fn generate_yearly_summary(llm: &LLMManager) -> Result<(), String> {
    let now = Local::now();
    let year_label = now.year().to_string();
    
    // 收集本年所有季度/月度总结
    let quarterly_summaries = crate::commands::memory::get_quarterly_summaries()?;
    let summaries_text: Vec<String> = quarterly_summaries.iter()
        .filter(|s| s.quarter.starts_with(&year_label))
        .map(|s| format!("{}: {} - {}", s.quarter, s.summary, s.important_events.join(", ")))
        .collect();
    
    if summaries_text.is_empty() {
        return Ok(());
    }
    
    let prompt = format!(r#"请分析本年的季度总结，生成年度报告。

要求：
1. 总结本年的主要变化
2. 描述关系发展
3. 列出最难忘的时刻

## 季度总结
{}

请按以下 JSON 格式输出：
{{"keywords": [], "summary": "", "relationship_growth": "", "milestones": [], "memorable_moments": []}}"#, 
        summaries_text.join("\n"));
    
    let messages = vec![ChatMessage::user(&prompt)];
    let response = llm.chat(messages).await.map_err(|e| e.to_string())?;
    
    if let Ok(result) = serde_json::from_str::<serde_json::Value>(&response.content) {
        let y_summary = YearlySummary {
            year: year_label.clone(),
            keywords: result.get("keywords").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
            summary: result.get("summary").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            relationship_growth: result.get("relationship_growth").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            milestones: result.get("milestones").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
            memorable_moments: result.get("memorable_moments").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
        };
        
        crate::commands::memory::save_yearly_summary(y_summary)?;
        println!("Yearly summary generated for {}", year_label);
    }
    
    Ok(())
}

/// 手动触发周总结生成
#[tauri::command]
pub async fn trigger_weekly_summary(llm_manager: State<'_, Arc<LLMManager>>) -> Result<(), String> {
    generate_weekly_summary(&llm_manager).await
}

/// 手动触发月总结生成
#[tauri::command]
pub async fn trigger_monthly_summary(llm_manager: State<'_, Arc<LLMManager>>) -> Result<(), String> {
    generate_monthly_summary(&llm_manager).await
}

/// 手动触发季度总结生成
#[tauri::command]
pub async fn trigger_quarter(llm_manager: State<'_, Arc<LLMManager>>) -> Result<(), String> {
    generate_quarterly_summary(&llm_manager).await
}

/// 手动触发年度总结生成
#[tauri::command]
pub async fn trigger_year(llm_manager: State<'_, Arc<LLMManager>>) -> Result<(), String> {
    generate_yearly_summary(&llm_manager).await
}

// ============ 辅助函数 - 判断是否需要生成总结 ============

/// 获取当前周的日期范围
fn get_current_week_dates() -> (String, String, String) {
    let today = Local::now().date_naive();
    let weekday = today.weekday().num_days_from_monday(); // 0 = Monday
    
    // 周一
    let week_start = today - chrono::Duration::days(weekday as i64);
    // 周日
    let week_end = week_start + chrono::Duration::days(6);
    
    let week_label = format!("{}-W{:02}", today.format("%Y"), today.iso_week().week());
    let start_str = week_start.format("%Y-%m-%d").to_string();
    let end_str = week_end.format("%Y-%m-%d").to_string();
    
    (start_str, end_str, week_label)
}

/// 检查是否应该生成周总结 (每周日)
fn should_generate_weekly_summary() -> bool {
    let today = Local::now().date_naive();
    today.weekday() == chrono::Weekday::Sun
}

/// 检查是否应该生成月度总结 (每月最后一天)
fn should_generate_monthly_summary() -> bool {
    let today = Local::now().date_naive();
    let last_day = NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
        .unwrap_or(NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap_or(today))
        - chrono::Duration::days(1);
    today == last_day
}

/// 检查是否应该生成季度总结 (3, 6, 9, 12 月最后一天)
fn should_generate_quarterly_summary() -> bool {
    let today = Local::now().date_naive();
    let month = today.month();
    
    // 只有 3, 6, 9, 12 月才可能生成季度总结
    if ![3, 6, 9, 12].contains(&month) {
        return false;
    }
    
    let last_day = NaiveDate::from_ymd_opt(today.year(), month + 1, 1)
        .unwrap_or(NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap_or(today))
        - chrono::Duration::days(1);
    today == last_day
}

/// 检查是否应该生成年度总结 (12月31日)
fn should_generate_yearly_summary() -> bool {
    let today = Local::now().date_naive();
    today.month() == 12 && today.day() == 31
}
