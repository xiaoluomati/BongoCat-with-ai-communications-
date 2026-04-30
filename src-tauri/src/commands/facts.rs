// User Facts Memory Commands
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use uuid::Uuid;

use crate::commands::character::{get_user_profile, save_user_profile, UserProfile};

// ── 敏感信息检测 ────────────────────────────────────────────

/// 敏感谓词屏蔽列表
const SENSITIVE_PREDICATES: &[&str] = &[
    "住在", "地址", "密码", "银行卡", "账号", "手机号", "电话", "身份证", "护照",
];

/// 检测事实是否包含敏感信息
fn is_sensitive_fact(predicate: &str, object: &str) -> bool {
    for sp in SENSITIVE_PREDICATES {
        if predicate.contains(sp) || object.contains(sp) {
            return true;
        }
    }
    // 手机号11位检测
    if object.len() >= 11 {
        for i in 0..object.len() {
            if i + 11 <= object.len() {
                let slice = &object[i..i + 11];
                if slice.chars().all(|c| c.is_ascii_digit()) {
                    return true;
                }
            }
        }
    }
    false
}

// ── 数据结构 ────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Evidence {
    pub date: String,
    pub message_id: String,
    pub snippet: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum FactStatus {
    Active,
    Superseded,
    Deleted,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserFact {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub evidence: Vec<Evidence>,
    pub confidence: f32,
    pub status: FactStatus,
    pub superseded_by: Option<String>,
    pub replaces: Option<String>,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub access_count: i32,
}

impl UserFact {
    /// 事实的唯一标识键（用于去重判断）
    fn identity_key(&self) -> (String, String, String) {
        (self.subject.clone(), self.predicate.clone(), self.object.clone())
    }
}

// ── 文件路径 ────────────────────────────────────────────

fn get_app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("BongoCat")
        .join("data")
}

fn get_facts_dir() -> PathBuf {
    get_app_data_dir().join("facts")
}

fn ensure_facts_dir() -> Result<(), String> {
    fs::create_dir_all(get_facts_dir()).map_err(|e| e.to_string())
}

fn get_pending_path() -> PathBuf {
    get_facts_dir().join("pending_facts.jsonl")
}

fn get_facts_path() -> PathBuf {
    get_facts_dir().join("facts.json")
}

fn get_pending_count_path() -> PathBuf {
    get_facts_dir().join(".pending_count")
}

fn now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ── 并发保护（定时合并）─────────────────────────────────────

/// 异步 Mutex 保护 pending 文件写入
static PENDING_FILE_MUTEX: Mutex<()> = Mutex::new(());

/// 上次合并时间戳
static LAST_MERGE_TS: AtomicU64 = AtomicU64::new(0);

/// 合并冷却时间（秒）
const MERGE_COOLDOWN_SECS: u64 = 600; // 10分钟

/// 触发合并的 pending 数量阈值
const MERGE_THRESHOLD: u64 = 20;

fn should_auto_merge() -> bool {
    let pending_count = read_pending_count();
    let now = now_timestamp() as u64;
    let last_merge = LAST_MERGE_TS.load(Ordering::Relaxed);

    pending_count >= MERGE_THRESHOLD || (now - last_merge >= MERGE_COOLDOWN_SECS && pending_count > 0)
}

fn read_pending_count() -> u64 {
    let path = get_pending_count_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    } else {
        0
    }
}

fn write_pending_count(count: u64) {
    let path = get_pending_count_path();
    let _ = fs::write(&path, count.to_string());
}

fn increment_pending_count() {
    let current = read_pending_count();
    write_pending_count(current + 1);
}

// ── Tauri Commands ─────────────────────────────────────────

#[tauri::command]
pub fn save_user_fact(
    subject: String,
    predicate: String,
    object: String,
    evidence_date: String,
    evidence_message_id: String,
    evidence_snippet: String,
    confidence: f32,
    tags: Vec<String>,
) -> Result<String, String> {
    ensure_facts_dir()?;

    // 敏感信息检测
    if is_sensitive_fact(&predicate, &object) {
        return Err("敏感信息过滤：该事实包含禁止提取的内容".to_string());
    }

    let id = Uuid::new_v4().to_string();
    let now = now_timestamp();

    let fact = UserFact {
        id: id.clone(),
        subject,
        predicate,
        object,
        evidence: vec![Evidence {
            date: evidence_date,
            message_id: evidence_message_id,
            snippet: evidence_snippet,
        }],
        confidence,
        status: FactStatus::Active,
        superseded_by: None,
        replaces: None,
        tags,
        created_at: now,
        updated_at: now,
        access_count: 0,
    };

    let json = serde_json::to_string(&fact).map_err(|e| e.to_string())?;
    let jsonl = json + "\n";

    // 使用 std::sync::Mutex（持有锁时间尽量短）
    let _guard = PENDING_FILE_MUTEX.lock().unwrap();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_pending_path())
        .map_err(|e| e.to_string())?;
    file.write_all(jsonl.as_bytes()).map_err(|e| e.to_string())?;
    drop(_guard);

    // 增加 pending 计数
    increment_pending_count();

    // 检查是否触发自动合并
    if should_auto_merge() {
        LAST_MERGE_TS.store(now as u64, Ordering::Relaxed);
        let _ = merge_pending_facts();
    }

    Ok(id)
}

#[tauri::command]
pub fn merge_pending_facts() -> Result<i32, String> {
    ensure_facts_dir()?;

    let pending_path = get_pending_path();
    if !pending_path.exists() {
        return Ok(0);
    }

    let file = fs::File::open(&pending_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    let mut pending_facts: Vec<UserFact> = Vec::new();

    // 读取所有 pending facts
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<UserFact>(&line) {
            Ok(fact) => pending_facts.push(fact),
            Err(_) => continue,
        }
    }

    // 读取已有 facts
    let facts_path = get_facts_path();
    let mut all_facts: std::collections::HashMap<String, UserFact> = std::collections::HashMap::new();

    if facts_path.exists() {
        let content = fs::read_to_string(&facts_path).map_err(|e| e.to_string())?;
        let existing: Vec<UserFact> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        for fact in existing {
            all_facts.insert(fact.id.clone(), fact);
        }
    }

    // 处理 pending facts
    for pending in pending_facts {
        // 低置信度（< 0.7）暂不合并，保留在 pending 中
        if pending.confidence < 0.7 {
            continue;
        }

        // 检查完全匹配（同 id）
        if all_facts.contains_key(&pending.id) {
            // 更新 access_count 和 updated_at
            if let Some(existing) = all_facts.get_mut(&pending.id) {
                existing.access_count += 1;
                existing.updated_at = pending.updated_at;
                existing.evidence.extend(pending.evidence);
            }
        } else {
            // 检查冲突：同 subject+predicate 但不同 object
            let key = pending.identity_key();
            let conflict_id = all_facts.values()
                .find(|f| f.status == FactStatus::Active && f.identity_key() == key && f.object != pending.object)
                .map(|f| f.id.clone());

            if let Some(old_id) = conflict_id {
                // 冲突处理：旧事实标记为 superseded，新事实替换
                if let Some(old_fact) = all_facts.get_mut(&old_id) {
                    old_fact.status = FactStatus::Superseded;
                    old_fact.superseded_by = Some(pending.id.clone());
                }

                let mut new_fact = pending.clone();
                new_fact.status = FactStatus::Active;
                new_fact.replaces = Some(old_id);
                all_facts.insert(new_fact.id.clone(), new_fact);
            } else {
                // 无冲突，直接添加
                all_facts.insert(pending.id.clone(), pending);
            }
        }
    }

    // 写入合并结果
    let merged: Vec<UserFact> = all_facts.into_values().collect();
    let content = serde_json::to_string_pretty(&merged).map_err(|e| e.to_string())?;
    fs::write(&facts_path, content).map_err(|e| e.to_string())?;

    // 清空 pending 文件和计数
    fs::write(&pending_path, "").map_err(|e| e.to_string())?;
    write_pending_count(0);
    LAST_MERGE_TS.store(now_timestamp() as u64, Ordering::Relaxed);

    // ── UserProfile 同步 ─────────────────────────────────────────────
    let mut profile = get_user_profile().unwrap_or_else(|_| UserProfile {
        user_name: None,
        traits: vec![],
        preferences: HashMap::new(),
        important_dates: HashMap::new(),
        recent_interactions: vec![],
        special_memories: vec![],
        conversation_count: 0,
        last_updated: String::new(),
    });

    for fact in &merged {
        // 只同步 Active 且高置信度
        if fact.status != FactStatus::Active || fact.confidence < 0.7 {
            continue;
        }

        match fact.predicate.as_str() {
            "喜欢"       => { profile.preferences.insert("喜欢".to_string(),   fact.object.clone()); }
            "不喜欢"     => { profile.preferences.insert("不喜欢".to_string(), fact.object.clone()); }
            "讨厌"       => { profile.preferences.insert("讨厌".to_string(),   fact.object.clone()); }
            "职业"       => { profile.preferences.insert("职业".to_string(),   fact.object.clone()); }
            "工作"       => { profile.preferences.insert("工作".to_string(),   fact.object.clone()); }
            "生日"       => { profile.important_dates.insert("生日".to_string(),   fact.object.clone()); }
            "纪念日"     => { profile.important_dates.insert("纪念日".to_string(), fact.object.clone()); }
            "地点" | "住在" => {
                profile.preferences.insert("地点".to_string(), fact.object.clone());
            }
            _ => {}
        }
    }

    profile.last_updated = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = save_user_profile(profile);

    Ok(merged.len() as i32)
}

#[tauri::command]
pub fn get_user_facts(tag_filter: Option<String>) -> Result<Vec<UserFact>, String> {
    let facts_path = get_facts_path();
    if !facts_path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&facts_path).map_err(|e| e.to_string())?;
    let mut facts: Vec<UserFact> = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    // 只返回 active 状态
    facts.retain(|f| f.status == FactStatus::Active);

    // 标签筛选
    if let Some(tag) = tag_filter {
        facts.retain(|f| f.tags.iter().any(|t| t == &tag));
    }

    // 按 access_count 和 updated_at 排序
    facts.sort_by(|a, b| {
        let a_score = a.access_count as i64 * 1000 + a.updated_at as i64;
        let b_score = b.access_count as i64 * 1000 + b.updated_at as i64;
        b_score.cmp(&a_score)
    });

    Ok(facts)
}

#[tauri::command]
pub fn delete_user_fact(fact_id: String) -> Result<bool, String> {
    let facts_path = get_facts_path();
    if !facts_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&facts_path).map_err(|e| e.to_string())?;
    let mut facts: Vec<UserFact> = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let pos = facts.iter().position(|f| f.id == fact_id);
    if let Some(idx) = pos {
        facts[idx].status = FactStatus::Deleted;
        let content = serde_json::to_string_pretty(&facts).map_err(|e| e.to_string())?;
        fs::write(&facts_path, content).map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn get_facts_count() -> Result<i32, String> {
    let facts_path = get_facts_path();
    if !facts_path.exists() {
        return Ok(0);
    }
    let content = fs::read_to_string(&facts_path).map_err(|e| e.to_string())?;
    let facts: Vec<UserFact> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let active_count = facts.iter().filter(|f| f.status == FactStatus::Active).count() as i32;
    Ok(active_count)
}

/// 获取相关事实（用于注入 system prompt）
/// 策略：时间衰减 + 访问频率 + 关键词匹配
#[tauri::command]
pub fn get_relevant_facts(context: Option<String>, limit: Option<i32>) -> Result<Vec<UserFact>, String> {
    let facts = get_user_facts(None)?;
    let limit = limit.unwrap_or(5) as usize;
    let now = now_timestamp() as i64;
    let seven_days = 7 * 24 * 3600;

    // 候选：7天内事实，按访问次数降序
    let mut candidates: Vec<UserFact> = facts
        .into_iter()
        .filter(|f| now - f.created_at < seven_days)
        .collect();

    candidates.sort_by(|a, b| {
        let a_score = a.access_count as i64 * 1000 + (now - a.created_at) as i64;
        let b_score = b.access_count as i64 * 1000 + (now - b.created_at) as i64;
        b_score.cmp(&a_score)
    });

    // 关键词匹配（如有 context）
    if let Some(ref ctx) = context {
        if !ctx.is_empty() {
            let keywords: Vec<&str> = ctx.split_whitespace().collect();
            if !keywords.is_empty() {
                let matched: Vec<UserFact> = candidates
                    .iter()
                    .filter(|f| {
                        keywords.iter().any(|k| {
                            f.object.contains(k) ||
                            f.predicate.contains(k) ||
                            f.tags.iter().any(|t| t.contains(k))
                        })
                    })
                    .take(limit)
                    .cloned()
                    .collect();

                if !matched.is_empty() {
                    return Ok(matched);
                }
            }
        }
    }

    // 返回前 limit 条
    Ok(candidates.into_iter().take(limit).collect())
}