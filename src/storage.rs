use crate::models::{ProjectData, ProjectEntry};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WorkspaceSettings {
    pub workspace_root: String,
}

pub fn default_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("PAPER_COMPOSER_DATA_DIR") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("user_data")
}

pub fn workspace_settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("workspace_settings.json")
}

pub fn load_workspace_settings(data_dir: &Path) -> WorkspaceSettings {
    fs::read_to_string(workspace_settings_path(data_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_workspace_settings(data_dir: &Path, settings: &WorkspaceSettings) -> std::io::Result<()> {
    fs::create_dir_all(data_dir)?;
    fs::write(workspace_settings_path(data_dir), serde_json::to_string_pretty(settings)?)
}

pub fn scan_workspace_projects(data_dir: &Path) -> Vec<(ProjectEntry, ProjectData)> {
    let settings = load_workspace_settings(data_dir);
    let root = PathBuf::from(settings.workspace_root.trim());
    let mut projects = Vec::new();
    if !root.is_dir() {
        return projects;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return projects;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let project_path = path.join("project.json");
        let Some(data) = fs::read_to_string(project_path)
            .ok()
            .and_then(|text| serde_json::from_str::<ProjectData>(&text).ok())
        else {
            continue;
        };
        projects.push((
            ProjectEntry {
                project_id: data.project.project_id.clone(),
                project_name: data.project.project_name.clone(),
                folder_id: data.project.folder_id.clone(),
                local_path: path.display().to_string(),
            },
            data,
        ));
    }
    projects.sort_by(|a, b| a.0.project_name.cmp(&b.0.project_name));
    projects
}

pub fn project_workspace_dir(_data_dir: &Path, entry: &ProjectEntry) -> PathBuf {
    PathBuf::from(entry.local_path.trim())
}

pub fn project_json_path(data_dir: &Path, entry: &ProjectEntry) -> PathBuf {
    project_workspace_dir(data_dir, entry).join("project.json")
}

pub fn load_project_data(data_dir: &Path, entry: &ProjectEntry) -> Option<ProjectData> {
    let path = project_json_path(data_dir, entry);
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

pub fn save_project_data(
    data_dir: &Path,
    entry: &ProjectEntry,
    data: &ProjectData,
) -> std::io::Result<()> {
    let path = project_json_path(data_dir, entry);
    fs::create_dir_all(path.parent().unwrap())?;
    let text = serde_json::to_string_pretty(data)?;
    fs::write(path, text)
}

pub fn remove_project_workspace(data_dir: &Path, entry: &ProjectEntry) -> std::io::Result<()> {
    let path = project_workspace_dir(data_dir, entry);
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}
