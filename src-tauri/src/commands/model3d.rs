//! 3D Model Management Commands

use crate::commands::config::get_app_data_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

fn get_models_dir() -> std::path::PathBuf {
    get_app_data_dir().join("models_3d")
}

fn get_index_path() -> std::path::PathBuf {
    get_models_dir().join("index.json")
}

fn ensure_dirs() -> Result<(), String> {
    fs::create_dir_all(&get_models_dir()).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model3DInfo {
    pub id: String,
    pub name: String,
    pub pmx_path: String,
    pub motions: HashMap<String, String>,
    pub added_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    models: Vec<Model3DInfo>,
}

fn load_index() -> Result<Vec<Model3DInfo>, String> {
    let path = get_index_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let index: IndexFile = serde_json::from_str(&content).unwrap_or(IndexFile { models: vec![] });
    Ok(index.models)
}

fn save_index(models: &[Model3DInfo]) -> Result<(), String> {
    ensure_dirs()?;
    let index = IndexFile { models: models.to_vec() };
    let content = serde_json::to_string_pretty(&index).map_err(|e| e.to_string())?;
    fs::write(get_index_path(), content).map_err(|e| e.to_string())
}

/// List all imported 3D models
#[tauri::command]
pub fn list_3d_models() -> Result<Vec<Model3DInfo>, String> {
    load_index()
}

/// Import a PMX model by copying it into the app data directory.
/// source_path: path to the .pmx file selected by the user.
/// name: display name for the model.
#[tauri::command]
pub fn add_3d_model(name: String, source_path: String) -> Result<Model3DInfo, String> {
    ensure_dirs()?;

    let id = uuid::Uuid::new_v4().to_string();
    let model_dir = get_models_dir().join(&id);
    fs::create_dir_all(&model_dir).map_err(|e| e.to_string())?;

    // Copy PMX file
    let pmx_name = format!("model.pmx");
    let pmx_dest = model_dir.join(&pmx_name);
    fs::copy(&source_path, &pmx_dest).map_err(|e| e.to_string())?;

    // Copy texture folder if it exists alongside the PMX
    let source_dir = std::path::Path::new(&source_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));
    for tex_dir_name in &["tex", "textures", "texture"] {
        let tex_src = source_dir.join(tex_dir_name);
        if tex_src.exists() && tex_src.is_dir() {
            let tex_dest = model_dir.join(tex_dir_name);
            copy_dir_recursive(&tex_src, &tex_dest)?;
            break;
        }
    }

    let model = Model3DInfo {
        id: id.clone(),
        name,
        pmx_path: pmx_dest.to_string_lossy().to_string(),
        motions: HashMap::new(),
        added_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    let mut models = load_index()?;
    models.push(model.clone());
    save_index(&models)?;

    Ok(model)
}

/// Remove a 3D model
#[tauri::command]
pub fn remove_3d_model(id: String) -> Result<(), String> {
    let model_dir = get_models_dir().join(&id);
    if model_dir.exists() {
        fs::remove_dir_all(&model_dir).map_err(|e| e.to_string())?;
    }
    let mut models = load_index()?;
    models.retain(|m| m.id != id);
    save_index(&models)?;
    Ok(())
}

/// Add a VMD motion to a model
#[tauri::command]
pub fn add_model_motion(model_id: String, motion_name: String, vmd_path: String) -> Result<(), String> {
    let model_dir = get_models_dir().join(&model_id);
    let motions_dir = model_dir.join("motions");
    fs::create_dir_all(&motions_dir).map_err(|e| e.to_string())?;

    let ext = std::path::Path::new(&vmd_path)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let dest_name = format!("{}.{}", motion_name, ext);
    let dest = motions_dir.join(&dest_name);
    fs::copy(&vmd_path, &dest).map_err(|e| e.to_string())?;

    let mut models = load_index()?;
    if let Some(model) = models.iter_mut().find(|m| m.id == model_id) {
        model.motions.insert(motion_name, dest.to_string_lossy().to_string());
    }
    save_index(&models)?;
    Ok(())
}

/// Copy directory recursively
fn copy_dir_recursive(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
