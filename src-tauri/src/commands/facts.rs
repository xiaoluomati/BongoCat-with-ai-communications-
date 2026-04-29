// User Facts Memory Commands
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

fn get_app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("BongoCat")
        .join("data")
}

fn get_data_dir() -> PathBuf { get_app_data_dir() }
fn get_facts_dir() -> PathBuf { get_data_dir().join("facts") }

fn ensure_facts_dir() -> Result<(), String> {
    fs::create_dir_all(get_facts_dir()).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Evidence {
    pub date: String,
    pub message_id: String,
    pub snippet: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserFact {
    pub id: String,
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub evidence: Vec<Evidence>,
    pub confidence: f32,
    pub tags: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub access_count: i32,
}

fn get_pending_path() -> PathBuf { get_facts_dir().join("pending_facts.jsonl") }
fn get_user_facts_path(user_id: &str) -> PathBuf { get_facts_dir().join(format!("{}_facts.json", user_id)) }

fn now_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Mutex for protecting concurrent writes to pending_facts.jsonl
static PENDING_FILE_MUTEX: Mutex<()> = Mutex::new(());

#[tauri::command]
pub fn save_user_fact(
    _user_id: String,
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
        tags,
        created_at: now,
        updated_at: now,
        access_count: 0,
    };

    let json = serde_json::to_string(&fact).map_err(|e| e.to_string())?;
    let jsonl = json + "\n";

    // Lock for concurrent write safety
    let _guard = PENDING_FILE_MUTEX.lock().map_err(|e| e.to_string())?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_pending_path())
        .map_err(|e| e.to_string())?;
    file.write_all(jsonl.as_bytes()).map_err(|e| e.to_string())?;

    Ok(id)
}

#[tauri::command]
pub fn merge_pending_facts(user_id: String) -> Result<i32, String> {
    ensure_facts_dir()?;

    let pending_path = get_pending_path();
    if !pending_path.exists() {
        return Ok(0);
    }

    let file = fs::File::open(&pending_path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);

    let mut all_facts: std::collections::HashMap<String, UserFact> = std::collections::HashMap::new();

    // Read existing merged facts
    let user_facts_path = get_user_facts_path(&user_id);
    if user_facts_path.exists() {
        let content = fs::read_to_string(&user_facts_path).map_err(|e| e.to_string())?;
        let existing: Vec<UserFact> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        for fact in existing {
            all_facts.insert(fact.id.clone(), fact);
        }
    }

    // Merge pending facts (skip duplicates by id)
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let pending_fact: UserFact = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        all_facts.insert(pending_fact.id.clone(), pending_fact);
    }

    // Deduplicate by subject+predicate+object triplet
    let mut seen: std::collections::HashSet<(String, String, String), _> = std::collections::HashSet::new();
    let merged: Vec<UserFact> = all_facts
        .into_values()
        .filter(|fact| {
            let key = (fact.subject.clone(), fact.predicate.clone(), fact.object.clone());
            seen.insert(key)
        })
        .collect();

    // Write merged facts back
    let content = serde_json::to_string_pretty(&merged).map_err(|e| e.to_string())?;
    fs::write(&user_facts_path, content).map_err(|e| e.to_string())?;

    // Clear pending file
    fs::write(&pending_path, "").map_err(|e| e.to_string())?;

    Ok(merged.len() as i32)
}

#[tauri::command]
pub fn get_user_facts(user_id: String) -> Result<Vec<UserFact>, String> {
    let user_facts_path = get_user_facts_path(&user_id);
    if !user_facts_path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&user_facts_path).map_err(|e| e.to_string())?;
    let facts: Vec<UserFact> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(facts)
}

#[tauri::command]
pub fn delete_user_fact(user_id: String, fact_id: String) -> Result<bool, String> {
    let user_facts_path = get_user_facts_path(&user_id);
    if !user_facts_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&user_facts_path).map_err(|e| e.to_string())?;
    let mut facts: Vec<UserFact> = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let original_len = facts.len();
    facts.retain(|f| f.id != fact_id);

    if facts.len() == original_len {
        return Ok(false);
    }

    let content = serde_json::to_string_pretty(&facts).map_err(|e| e.to_string())?;
    fs::write(&user_facts_path, content).map_err(|e| e.to_string())?;

    Ok(true)
}

#[tauri::command]
pub fn get_facts_count(user_id: String) -> Result<i32, String> {
    let user_facts_path = get_user_facts_path(&user_id);
    if !user_facts_path.exists() {
        return Ok(0);
    }
    let content = fs::read_to_string(&user_facts_path).map_err(|e| e.to_string())?;
    let facts: Vec<UserFact> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(facts.len() as i32)
}