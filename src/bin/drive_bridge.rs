use paper_composer::{
    config::{self, DriveSettings},
    models::{ProjectData, ProjectEntry, Project},
    storage::{
        default_data_dir,
        load_project_data, load_workspace_settings, save_project_data, save_workspace_settings,
        scan_workspace_projects, project_workspace_dir, remove_project_workspace, WorkspaceSettings,
    },
    sync::google_drive::{
        GoogleDriveClient, GoogleDriveRestClient, refresh_access_token, run_local_oauth_flow,
    },
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    time::Duration,
};
use url::form_urlencoded;
use uuid::Uuid;

const BRIDGE_ADDR: &str = "127.0.0.1:53683";

#[derive(Deserialize)]
struct ConnectRequest {
    client_id: Option<String>,
    client_secret: Option<String>,
    scope: Option<String>,
    workspace_root: Option<String>,
}

#[derive(Deserialize)]
struct UploadProjectRequest {
    project_id: String,
    project_name: String,
    folder_id: String,
    local_path: String,
    project_data: ProjectData,
}

#[derive(Deserialize)]
struct AddProjectRequest {
    project_name: String,
    folder_id: String,
}

#[derive(Deserialize)]
struct LocalAddProjectRequest {
    project_name: String,
    local_path: String,
}

#[derive(Deserialize)]
struct DeleteProjectRequest {
    project_id: String,
    project_name: String,
    folder_id: String,
    local_path: String,
}

#[derive(Deserialize)]
struct SaveProjectRequest {
    project_id: String,
    project_name: String,
    folder_id: String,
    local_path: String,
    project_data: ProjectData,
}

#[derive(Deserialize)]
struct SyncProjectRequest {
    project_id: String,
    project_name: String,
    folder_id: String,
    local_path: String,
}

#[derive(Serialize)]
struct StatusResponse {
    ok: bool,
    configured: bool,
    authenticated: bool,
    message: String,
    settings_txt_path: Option<String>,
}

#[derive(Serialize)]
struct SettingsResponse {
    ok: bool,
    configured: bool,
    authenticated: bool,
    client_id: String,
    scope: String,
    has_client_secret: bool,
    workspace_root: String,
    message: String,
    settings_txt_path: String,
}

#[derive(Serialize)]
struct StartupSyncResponse {
    ok: bool,
    message: String,
    projects: Vec<ProjectEntry>,
    projects_data: serde_json::Value,
}

#[derive(Serialize)]
struct FolderSummary {
    ok: bool,
    message: String,
    total: usize,
    json_count: usize,
    pdf_count: usize,
    image_count: usize,
    other_count: usize,
}

#[derive(Serialize)]
struct DriveAsset {
    id: String,
    name: String,
    modified_time: String,
}

#[derive(Serialize)]
struct FolderIndexResponse {
    ok: bool,
    message: String,
    pdfs: Vec<DriveAsset>,
    jsons: Vec<JsonAsset>,
}

#[derive(Serialize)]
struct SyncProjectResponse {
    ok: bool,
    message: String,
    project_id: String,
    project_entry: ProjectEntry,
    project_data: ProjectData,
    pdfs: Vec<DriveAsset>,
    jsons: Vec<JsonAsset>,
}

fn project_entry(project_id: &str, project_name: &str, folder_id: &str, local_path: &str) -> ProjectEntry {
    ProjectEntry {
        project_id: project_id.to_owned(),
        project_name: project_name.to_owned(),
        folder_id: folder_id.to_owned(),
        local_path: local_path.to_owned(),
    }
}

#[derive(Serialize)]
struct JsonAsset {
    id: String,
    name: String,
    modified_time: String,
    content: serde_json::Value,
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind(BRIDGE_ADDR)?;
    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            let _ = handle_stream(&mut stream);
        }
    }
    Ok(())
}

fn handle_stream(stream: &mut TcpStream) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let request = read_http_request(stream)?;
    let (method, target) = request_line(&request).unwrap_or(("", ""));
    let (path, query) = split_target(target);

    if method == "OPTIONS" {
        return respond_text(stream, 204, "");
    }

    let body = request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or("");

    match (method, path) {
        ("GET", "/status") => status(stream),
        ("GET", "/settings") => settings(stream),
        ("POST", "/connect") => connect(stream, body),
        ("POST", "/add-project") => add_project(stream, body),
        ("POST", "/local-add-project") => local_add_project(stream, body),
        ("POST", "/delete-project") => delete_project(stream, body),
        ("POST", "/sync-project") => sync_project(stream, body),
        ("GET", "/local-folder-index") => {
            if let Err(err) = local_folder_index(stream, query) {
                respond_error(stream, err)
            } else {
                Ok(())
            }
        }
        ("POST", "/save-project") => save_project_local(stream, body),
        ("GET", "/colors") => colors_get(stream),
        ("POST", "/colors") => colors_post(stream, body),
        ("POST", "/open-file") => open_file(stream, body),
        ("GET", "/startup-sync") => startup_sync(stream),
        ("POST", "/upload-project") => upload_project(stream, body),
        ("GET", "/summary") => {
            if let Err(err) = summary(stream, query) {
                respond_error(stream, err)
            } else {
                Ok(())
            }
        }
        ("GET", "/folder-index") => {
            if let Err(err) = folder_index(stream, query) {
                respond_error(stream, err)
            } else {
                Ok(())
            }
        }
        _ => respond_json(
            stream,
            404,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: "Unknown endpoint".to_owned(),
                settings_txt_path: None,
            },
        ),
    }
}

fn status(stream: &mut TcpStream) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let settings = config::load_drive_settings(&data_dir);
    respond_json(
        stream,
        200,
        &StatusResponse {
            ok: true,
            configured: settings.configured(),
            authenticated: settings.authenticated(),
            message: "Bridge ready".to_owned(),
            settings_txt_path: Some(config::drive_settings_path(&data_dir).display().to_string()),
        },
    )
}

fn settings(stream: &mut TcpStream) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let settings = config::load_drive_settings(&data_dir);
    let workspace = load_workspace_settings(&data_dir);
    respond_json(
        stream,
        200,
        &SettingsResponse {
            ok: true,
            configured: settings.configured(),
            authenticated: settings.authenticated(),
            client_id: settings.client_id,
            scope: settings.scope,
            has_client_secret: !settings.client_secret.trim().is_empty(),
            workspace_root: workspace.workspace_root,
            message: "已讀取本機 Drive 設定".to_owned(),
            settings_txt_path: config::drive_settings_path(&data_dir).display().to_string(),
        },
    )
}

fn connect(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let req: ConnectRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(err) => {
            return respond_json(
                stream,
                400,
                &StatusResponse {
                    ok: false,
                    configured: false,
                    authenticated: false,
                    message: format!("Invalid request: {err}"),
                    settings_txt_path: None,
                },
            );
        }
    };

    let mut settings = config::load_drive_settings(&data_dir);
    if let Some(client_id) = req.client_id.filter(|v| !v.trim().is_empty()) {
        settings.client_id = client_id;
    }
    if let Some(client_secret) = req.client_secret.filter(|v| !v.trim().is_empty()) {
        settings.client_secret = client_secret;
    }
    if let Some(scope) = req.scope.filter(|v| !v.trim().is_empty()) {
        settings.scope = scope;
    }
    if let Some(workspace_root) = req.workspace_root.filter(|v| !v.trim().is_empty()) {
        save_workspace_settings(
            &data_dir,
            &WorkspaceSettings {
                workspace_root: workspace_root.trim().to_owned(),
            },
        )?;
    }

    if !settings.configured() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: "請填 Client ID 和 Client Secret".to_owned(),
                settings_txt_path: None,
            },
        );
    }

    let result = if settings.authenticated() {
        refresh_access_token(settings).or_else(|_| run_local_oauth_flow(config::load_drive_settings(&data_dir)))
    } else {
        run_local_oauth_flow(settings)
    };

    match result.and_then(|settings| config::save_drive_settings(&data_dir, &settings)) {
        Ok(_) => {
            let settings = config::load_drive_settings(&data_dir);
            respond_json(
                stream,
                200,
                &StatusResponse {
                    ok: true,
                    configured: settings.configured(),
                    authenticated: settings.authenticated(),
                    message: "Google Drive 已連線".to_owned(),
                    settings_txt_path: Some(config::drive_settings_path(&data_dir).display().to_string()),
                },
            )
        }
        Err(err) => respond_json(
            stream,
            500,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: false,
                message: format!("Google OAuth 失敗：{err}"),
                settings_txt_path: None,
            },
        ),
    }
}

fn add_project(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let req: AddProjectRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(err) => {
            return respond_json(
                stream,
                400,
                &StatusResponse {
                    ok: false,
                    configured: false,
                    authenticated: false,
                    message: format!("Invalid request: {err}"),
                    settings_txt_path: None,
                },
            );
        }
    };

    if req.project_name.trim().is_empty() || req.folder_id.trim().is_empty() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: "專案名稱、Folder ID 不能為空".to_owned(),
                settings_txt_path: None,
            },
        );
    }

    let settings = config::load_drive_settings(&data_dir);
    if !settings.authenticated() {
        return respond_json(
            stream,
            403,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: false,
                message: "請先連線 Google Drive".to_owned(),
                settings_txt_path: None,
            },
        );
    }

    let workspace_settings = load_workspace_settings(&data_dir);
    if workspace_settings.workspace_root.trim().is_empty() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: true,
                message: "請先在 Google Drive 設定中填寫工作區總資料夾".to_owned(),
                settings_txt_path: None,
            },
        );
    }

    let workspace_root = PathBuf::from(workspace_settings.workspace_root.trim());
    if let Err(err) = fs::create_dir_all(&workspace_root) {
        return respond_json(
            stream,
            500,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: true,
                message: format!("無法建立工作區總資料夾: {err}"),
                settings_txt_path: None,
            },
        );
    }
    let project_id = Uuid::new_v4().to_string();
    let project_folder_name = sanitize_filename(req.project_name.trim());
    let local_path = workspace_root.join(&project_folder_name);
    if local_path.join("project.json").exists() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: true,
                message: format!("專案資料夾已存在: {}", local_path.display()),
                settings_txt_path: None,
            },
        );
    }
    if let Err(err) = fs::create_dir_all(&local_path) {
        return respond_json(
            stream,
            500,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: true,
                message: format!("無法建立本地專案資料夾 {}: {err}", local_path.display()),
                settings_txt_path: None,
            },
        );
    }
    if !local_path.is_dir() {
        return respond_json(
            stream,
            500,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: true,
                message: format!("本地專案資料夾建立後仍找不到：{}", local_path.display()),
                settings_txt_path: None,
            },
        );
    }
    let entry = ProjectEntry {
        project_id: project_id.clone(),
        project_name: req.project_name.clone(),
        folder_id: req.folder_id.clone(),
        local_path: local_path.display().to_string(),
    };

    let empty_data = ProjectData {
        project: Project {
            project_id: project_id.clone(),
            project_name: req.project_name.clone(),
            folder_id: req.folder_id.clone(),
            sections: vec![],
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        },
        papers: vec![],
        algorithms: vec![],
        images: vec![],
        tables: vec![],
        notes: vec![],
    };

    if let Err(err) = save_project_data(&data_dir, &entry, &empty_data) {
        return respond_json(
            stream,
            500,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: true,
                message: format!("無法保存項目數據: {err}"),
                settings_txt_path: None,
            },
        );
    }

    let mut pdfs = Vec::new();
    let mut jsons = Vec::new();
    let mut project_data = empty_data;
    let mut drive_warning: Option<String> = None;
    if let Ok(settings) = authenticated_settings(&data_dir) {
        let client = GoogleDriveRestClient::new(settings);
        match sync_project_workspace(&client, &data_dir, &entry) {
            Ok((synced_data, synced_pdfs, synced_jsons)) => {
                project_data = synced_data;
                pdfs = synced_pdfs;
                jsons = synced_jsons;
            }
            Err(err) => {
                drive_warning = Some(err.to_string());
            }
        }
    }

    let base_message = format!("已建立專案：{}（本地：{}）", req.project_name, local_path.display());
    let message = match drive_warning {
        Some(reason) => format!("{base_message}；Drive 同步失敗：{reason}"),
        None => base_message,
    };

    respond_json(
        stream,
        200,
        &SyncProjectResponse {
            ok: true,
            message,
            project_id,
            project_entry: entry,
            project_data,
            pdfs,
            jsons,
        },
    )
}

fn startup_sync(stream: &mut TcpStream) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let scanned = scan_workspace_projects(&data_dir);
    let mut projects = Vec::new();
    let mut projects_data = serde_json::json!({});

    let drive_client = authenticated_settings(&data_dir)
        .ok()
        .map(GoogleDriveRestClient::new);
    let mut failures = Vec::new();
    for (entry, local_data) in scanned {
        let data = if entry.folder_id.starts_with("local:") {
            local_data
        } else if let Some(client) = drive_client.as_ref() {
            match sync_project_workspace(client, &data_dir, &entry) {
                Ok((synced, _, _)) => synced,
                Err(err) => {
                    failures.push(format!("{}: {err}", entry.project_name));
                    local_data
                }
            }
        } else {
            local_data
        };
        projects_data[&entry.project_id] = serde_json::to_value(&data).unwrap_or_default();
        projects.push(entry);
    }
    let message = if failures.is_empty() {
        format!("已掃描並載入 {} 個專案", projects.len())
    } else {
        format!("已載入 {} 個專案；同步失敗：{}", projects.len(), failures.join("; "))
    };
    respond_json(
        stream,
        200,
        &StartupSyncResponse {
            ok: true,
            message,
            projects,
            projects_data,
        },
    )
}

fn upload_project(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let req: UploadProjectRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(err) => {
            return respond_json(
                stream,
                400,
                &StatusResponse {
                    ok: false,
                    configured: false,
                    authenticated: false,
                    message: format!("Invalid request: {err}"),
                    settings_txt_path: None,
                },
            );
        }
    };

    let entry = project_entry(&req.project_id, &req.project_name, &req.folder_id, &req.local_path);
    if req.project_id.trim().is_empty() || req.folder_id.trim().is_empty() || req.local_path.trim().is_empty() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: false,
                message: "缺少 project_id、folder_id 或 local_path".to_owned(),
                settings_txt_path: None,
            },
        );
    }
    {
        save_project_data(&data_dir, &entry, &req.project_data)?;
        let mut settings = config::load_drive_settings(&data_dir);
        if settings.access_token.is_empty() {
            return respond_json(
                stream,
                403,
                &StatusResponse {
                    ok: false,
                    configured: true,
                    authenticated: false,
                    message: "請先連線 Google Drive".to_owned(),
                    settings_txt_path: None,
                },
            );
        }

        if let Ok(refreshed) = refresh_access_token(settings.clone()) {
            config::save_drive_settings(&data_dir, &refreshed).ok();
            settings = refreshed;
        }

        let client = GoogleDriveRestClient::new(settings);
        if let Err(err) = upload_workspace_to_drive(&client, &data_dir, &entry) {
            return respond_json(
                stream,
                500,
                &StatusResponse {
                    ok: false,
                    configured: true,
                    authenticated: true,
                    message: format!("上傳失敗: {err}"),
                    settings_txt_path: None,
                },
            );
        }
    }

    respond_json(
        stream,
        200,
        &StatusResponse {
            ok: true,
            configured: true,
            authenticated: true,
            message: "已儲存本地資料夾並上傳 Drive".to_owned(),
            settings_txt_path: Some(config::drive_settings_path(&data_dir).display().to_string()),
        },
    )
}

fn summary(stream: &mut TcpStream, query: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let folder_id = query_param(query, "folder_id").ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "folder_id 參數缺少",
        )
    })?;

    let settings = authenticated_settings(&data_dir)?;
    let client = GoogleDriveRestClient::new(settings);
    let files = client.list_files_in_folder(&folder_id)?;

    let json_count = files
        .iter()
        .filter(|file| file.name.ends_with(".json") || file.mime_type == "application/json")
        .count();
    let pdf_count = files
        .iter()
        .filter(|file| file.name.ends_with(".pdf") || file.mime_type == "application/pdf")
        .count();
    let image_count = files
        .iter()
        .filter(|file| file.mime_type.starts_with("image/"))
        .count();
    let other_count = files
        .len()
        .saturating_sub(json_count + pdf_count + image_count);

    let message = if json_count == 0 {
        format!("資料夾內：PDF {pdf_count}、JSON 0、影像 {image_count}；尚無 project.json")
    } else {
        format!("資料夾內：JSON {json_count}、PDF {pdf_count}、影像 {image_count}")
    };

    respond_json(
        stream,
        200,
        &FolderSummary {
            ok: true,
            message,
            total: files.len(),
            json_count,
            pdf_count,
            image_count,
            other_count,
        },
    )
}

fn folder_index(stream: &mut TcpStream, query: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let folder_id = query_param(query, "folder_id").ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "folder_id 參數缺少",
        )
    })?;

    let settings = authenticated_settings(&data_dir)?;
    let client = GoogleDriveRestClient::new(settings);
    let files = client.list_files_in_folder(&folder_id)?;

    let pdfs = files
        .iter()
        .filter(|file| file.name.to_lowercase().ends_with(".pdf") || file.mime_type == "application/pdf")
        .map(|file| DriveAsset {
            id: file.id.clone(),
            name: file.name.clone(),
            modified_time: file.modified_time.clone(),
        })
        .collect::<Vec<_>>();

    let json_files: Vec<_> = files
        .into_iter()
        .filter(|file| file.name.to_lowercase().ends_with(".json") || file.mime_type == "application/json")
        .collect();

    let mut jsons = Vec::new();
    for file in json_files {
        let temp_path = data_dir.join(format!("{}.json", sanitize_filename(&file.name)));
        if let Ok(_) = client.download_file(&file.id, &temp_path) {
            let content = fs::read_to_string(&temp_path)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            jsons.push(JsonAsset {
                id: file.id,
                name: file.name,
                modified_time: file.modified_time,
                content,
            });
            fs::remove_file(&temp_path).ok();
        }
    }

    respond_json(
        stream,
        200,
        &FolderIndexResponse {
            ok: true,
            message: format!("已索引資料夾：PDF {} 個，JSON {} 個", pdfs.len(), jsons.len()),
            pdfs,
            jsons,
        },
    )
}

fn local_add_project(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let req: LocalAddProjectRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(err) => {
            return respond_json(
                stream,
                400,
                &StatusResponse {
                    ok: false,
                    configured: false,
                    authenticated: false,
                    message: format!("Invalid request: {err}"),
                    settings_txt_path: None,
                },
            );
        }
    };

    if req.project_name.trim().is_empty() || req.local_path.trim().is_empty() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: "專案名稱和本地路徑不能為空".to_owned(),
                settings_txt_path: None,
            },
        );
    }

    let folder_path = PathBuf::from(req.local_path.trim());
    if !folder_path.is_dir() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: format!("路徑不是資料夾: {}", folder_path.display()),
                settings_txt_path: None,
            },
        );
    }

    let project_id = Uuid::new_v4().to_string();
    let folder_marker = format!("local:{}", folder_path.display());
    let entry = ProjectEntry {
        project_id: project_id.clone(),
        project_name: req.project_name.clone(),
        folder_id: folder_marker.clone(),
        local_path: folder_path.display().to_string(),
    };

    let empty_data = ProjectData {
        project: Project {
            project_id: project_id.clone(),
            project_name: req.project_name.clone(),
            folder_id: folder_marker,
            sections: vec![],
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        },
        papers: vec![],
        algorithms: vec![],
        images: vec![],
        tables: vec![],
        notes: vec![],
    };

    if let Err(err) = save_project_data(&data_dir, &entry, &empty_data) {
        return respond_json(
            stream,
            500,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: format!("無法保存項目數據: {err}"),
                settings_txt_path: None,
            },
        );
    }

    respond_json(
        stream,
        200,
        &StatusResponse {
            ok: true,
            configured: false,
            authenticated: false,
            message: format!("已建立本地專案: {}", req.project_name),
            settings_txt_path: None,
        },
    )
}

fn delete_project(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let req: DeleteProjectRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(err) => {
            return respond_json(
                stream,
                400,
                &StatusResponse {
                    ok: false,
                    configured: false,
                    authenticated: false,
                    message: format!("Invalid request: {err}"),
                    settings_txt_path: None,
                },
            );
        }
    };

    let entry = project_entry(&req.project_id, &req.project_name, &req.folder_id, &req.local_path);
    if entry.project_id.trim().is_empty() || entry.local_path.trim().is_empty() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: "缺少 project_id 或 local_path".to_owned(),
                settings_txt_path: None,
            },
        );
    }
    if !entry.folder_id.starts_with("local:") && !entry.folder_id.trim().is_empty() {
        let settings = match authenticated_settings(&data_dir) {
            Ok(settings) => settings,
            Err(err) => {
                return respond_json(
                    stream,
                    403,
                    &StatusResponse {
                        ok: false,
                        configured: true,
                        authenticated: false,
                        message: format!("移除專案需要先連線 Google Drive: {err}"),
                        settings_txt_path: None,
                    },
                );
            }
        };
        if let Err(err) = write_project_tombstone(&data_dir, &entry)
            .and_then(|path| {
                let client = GoogleDriveRestClient::new(settings);
                client.upload_file_to_folder(
                    &path,
                    &entry.folder_id,
                    "project.json",
                    "application/json; charset=UTF-8",
                )
            })
        {
            return respond_json(
                stream,
                500,
                &StatusResponse {
                    ok: false,
                    configured: true,
                    authenticated: true,
                    message: format!("移除專案失敗，空 JSON 尚未覆蓋 Drive: {err}"),
                    settings_txt_path: None,
                },
            );
        }
    }
    if let Err(err) = remove_project_workspace(&data_dir, &entry) {
        return respond_json(
            stream,
            500,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: format!("移除本地專案資料夾失敗: {err}"),
                settings_txt_path: None,
            },
        );
    }

    respond_json(
        stream,
        200,
        &StatusResponse {
            ok: true,
            configured: false,
            authenticated: false,
            message: "已移除本地專案資料夾，Drive PDF 保留，空 JSON 已覆蓋 Drive".to_owned(),
            settings_txt_path: None,
        },
    )
}

fn local_folder_index(stream: &mut TcpStream, query: &str) -> std::io::Result<()> {
    let path = query_param(query, "path").ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "path 參數缺少")
    })?;
    let folder = PathBuf::from(path.trim());
    if !folder.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("路徑不是資料夾: {}", folder.display()),
        ));
    }

    let mut pdfs = Vec::new();
    let mut jsons = Vec::new();
    for entry in fs::read_dir(&folder)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_internal_workspace_file(&name) {
            continue;
        }
        let lower = name.to_lowercase();
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| {
                chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        if is_local_pdf_file(&path, &name) {
            pdfs.push(DriveAsset {
                id: path.to_string_lossy().into_owned(),
                name,
                modified_time: modified,
            });
        } else if lower.ends_with(".json") {
            let content = fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            jsons.push(JsonAsset {
                id: path.to_string_lossy().into_owned(),
                name,
                modified_time: modified,
                content,
            });
        }
    }

    respond_json(
        stream,
        200,
        &FolderIndexResponse {
            ok: true,
            message: format!("本地資料夾：PDF {} 個，JSON {} 個", pdfs.len(), jsons.len()),
            pdfs,
            jsons,
        },
    )
}

fn save_project_local(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let req: SaveProjectRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(err) => {
            return respond_json(
                stream,
                400,
                &StatusResponse {
                    ok: false,
                    configured: false,
                    authenticated: false,
                    message: format!("Invalid request: {err}"),
                    settings_txt_path: None,
                },
            );
        }
    };

    let entry = project_entry(&req.project_id, &req.project_name, &req.folder_id, &req.local_path);
    if entry.project_id.trim().is_empty() || entry.local_path.trim().is_empty() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: "缺少 project_id 或 local_path".to_owned(),
                settings_txt_path: None,
            },
        );
    }

    if let Err(err) = save_project_data(&data_dir, &entry, &req.project_data) {
        return respond_json(
            stream,
            500,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: format!("無法保存: {err}"),
                settings_txt_path: None,
            },
        );
    }

    respond_json(
        stream,
        200,
        &StatusResponse {
            ok: true,
            configured: false,
            authenticated: false,
            message: "已存到本地專案資料夾".to_owned(),
            settings_txt_path: None,
        },
    )
}

fn sync_project(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let req: SyncProjectRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(err) => {
            return respond_json(
                stream,
                400,
                &StatusResponse {
                    ok: false,
                    configured: false,
                    authenticated: false,
                    message: format!("Invalid request: {err}"),
                    settings_txt_path: None,
                },
            );
        }
    };

    let entry = project_entry(&req.project_id, &req.project_name, &req.folder_id, &req.local_path);
    if entry.project_id.trim().is_empty() || entry.local_path.trim().is_empty() {
        return respond_json(
            stream,
            400,
            &StatusResponse {
                ok: false,
                configured: false,
                authenticated: false,
                message: "缺少 project_id 或 local_path".to_owned(),
                settings_txt_path: None,
            },
        );
    }

    if entry.folder_id.starts_with("local:") {
        let data = load_project_data(&data_dir, &entry).unwrap_or_else(|| empty_project_data(&entry));
        save_project_data(&data_dir, &entry, &data)?;
        let (pdfs, jsons) = collect_local_assets(&project_workspace_dir(&data_dir, &entry))?;
        return respond_json(
            stream,
            200,
            &SyncProjectResponse {
                ok: true,
                message: format!("已讀取本地資料夾：PDF {} 個，JSON {} 個", pdfs.len(), jsons.len()),
                project_id: entry.project_id.clone(),
                project_entry: entry,
                project_data: data,
                pdfs,
                jsons,
            },
        );
    }

    let settings = authenticated_settings(&data_dir)?;
    let client = GoogleDriveRestClient::new(settings);
    match sync_project_workspace(&client, &data_dir, &entry) {
        Ok((project_data, pdfs, jsons)) => respond_json(
            stream,
            200,
            &SyncProjectResponse {
                ok: true,
                message: format!("同步完成：PDF {} 個，JSON {} 個", pdfs.len(), jsons.len()),
                project_id: entry.project_id.clone(),
                project_entry: entry,
                project_data,
                pdfs,
                jsons,
            },
        ),
        Err(err) => respond_json(
            stream,
            500,
            &StatusResponse {
                ok: false,
                configured: true,
                authenticated: true,
                message: format!("同步失敗: {err}"),
                settings_txt_path: None,
            },
        ),
    }
}

fn authenticated_settings(data_dir: &PathBuf) -> std::io::Result<DriveSettings> {
    let mut settings = config::load_drive_settings(data_dir);
    if !settings.authenticated() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "請先連線 Google Drive",
        ));
    }
    settings = refresh_access_token(settings)?;
    config::save_drive_settings(data_dir, &settings).ok();
    Ok(settings)
}

fn sync_project_workspace(
    client: &GoogleDriveRestClient,
    data_dir: &PathBuf,
    entry: &ProjectEntry,
) -> std::io::Result<(ProjectData, Vec<DriveAsset>, Vec<JsonAsset>)> {
    let workspace = project_workspace_dir(data_dir, entry);
    fs::create_dir_all(&workspace)?;

    let files = client.list_files_in_folder(&entry.folder_id)?;
    if let Some(project_json) = files.iter().find(|file| file.name == "project.json") {
        let probe = workspace.join(".paper_composer_project_probe.json");
        client.download_file(&project_json.id, &probe)?;
        if is_project_tombstone(&probe) {
            fs::remove_file(&probe).ok();
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Drive project.json 是已刪除標記，略過同步",
            ));
        }
        fs::remove_file(&probe).ok();
    }
    for file in &files {
        let lower = file.name.to_lowercase();
        let is_pdf = lower.ends_with(".pdf") || file.mime_type == "application/pdf";
        let is_json = lower.ends_with(".json") || file.mime_type == "application/json";
        if !is_pdf && !is_json {
            continue;
        }
        let target = workspace.join(sanitize_filename(&file.name));
        if is_pdf && target.exists() {
            continue;
        }
        if target.exists() && target.file_name().and_then(|name| name.to_str()) == Some("project.json") {
            continue;
        }
        client.download_file(&file.id, &target)?;
    }

    let mut data = load_project_data(data_dir, entry).unwrap_or_else(|| empty_project_data(entry));
    data.project.project_id = entry.project_id.clone();
    data.project.project_name = entry.project_name.clone();
    data.project.folder_id = entry.folder_id.clone();
    save_project_data(data_dir, entry, &data)?;

    upload_workspace_to_drive(client, data_dir, entry)?;

    let (pdfs, jsons) = collect_local_assets(&workspace)?;
    Ok((data, pdfs, jsons))
}

fn upload_workspace_to_drive(
    client: &GoogleDriveRestClient,
    data_dir: &PathBuf,
    entry: &ProjectEntry,
) -> std::io::Result<()> {
    let workspace = project_workspace_dir(data_dir, entry);
    if !workspace.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("找不到本地專案資料夾: {}", workspace.display()),
        ));
    }
    let existing: std::collections::HashSet<String> = client
        .list_files_in_folder(&entry.folder_id)?
        .into_iter()
        .map(|f| f.name.to_lowercase())
        .collect();
    for item in fs::read_dir(&workspace)? {
        let item = item?;
        let path = item.path();
        if !path.is_file() {
            continue;
        }
        let name = item.file_name().to_string_lossy().into_owned();
        if is_internal_workspace_file(&name) {
            continue;
        }
        let lower = name.to_lowercase();
        if lower == "project.json" {
            client.upload_file_to_folder(&path, &entry.folder_id, &name, "application/json; charset=UTF-8")?;
            continue;
        }
        if existing.contains(&lower) {
            continue;
        }
        if lower.ends_with(".json") {
            client.upload_file_to_folder(&path, &entry.folder_id, &name, "application/json; charset=UTF-8")?;
        } else if is_local_pdf_file(&path, &name) {
            client.upload_file_to_folder(&path, &entry.folder_id, &name, "application/pdf")?;
        }
    }
    Ok(())
}

fn write_project_tombstone(data_dir: &PathBuf, entry: &ProjectEntry) -> std::io::Result<PathBuf> {
    let workspace = project_workspace_dir(data_dir, entry);
    if workspace.exists() {
        for item in fs::read_dir(&workspace)? {
            let path = item?.path();
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
    } else {
        fs::create_dir_all(&workspace)?;
    }
    let path = workspace.join("project.json");
    let tombstone = serde_json::json!({
        "paper_composer_deleted": true,
        "project_id": entry.project_id,
        "project_name": entry.project_name,
        "folder_id": entry.folder_id,
        "deleted_at": Utc::now().to_rfc3339()
    });
    fs::write(&path, serde_json::to_string_pretty(&tombstone)?)?;
    Ok(path)
}

fn is_project_tombstone(path: &PathBuf) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.get("paper_composer_deleted").and_then(|deleted| deleted.as_bool()))
        .unwrap_or(false)
}

fn empty_project_data(entry: &ProjectEntry) -> ProjectData {
    let now = Utc::now().to_rfc3339();
    ProjectData {
        project: Project {
            project_id: entry.project_id.clone(),
            project_name: entry.project_name.clone(),
            folder_id: entry.folder_id.clone(),
            sections: vec![],
            created_at: now.clone(),
            updated_at: now,
        },
        papers: vec![],
        algorithms: vec![],
        images: vec![],
        tables: vec![],
        notes: vec![],
    }
}

fn collect_local_assets(workspace: &PathBuf) -> std::io::Result<(Vec<DriveAsset>, Vec<JsonAsset>)> {
    let mut pdfs = Vec::new();
    let mut jsons = Vec::new();
    if !workspace.is_dir() {
        return Ok((pdfs, jsons));
    }

    for entry in fs::read_dir(workspace)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_internal_workspace_file(&name) {
            continue;
        }
        let lower = name.to_lowercase();
        let modified = file_modified_rfc3339(&path);
        if is_local_pdf_file(&path, &name) {
            pdfs.push(DriveAsset {
                id: path.to_string_lossy().into_owned(),
                name,
                modified_time: modified,
            });
        } else if lower.ends_with(".json") {
            let content = fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            jsons.push(JsonAsset {
                id: path.to_string_lossy().into_owned(),
                name,
                modified_time: modified,
                content,
            });
        }
    }
    pdfs.sort_by(|a, b| a.name.cmp(&b.name));
    jsons.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((pdfs, jsons))
}

fn file_modified_rfc3339(path: &PathBuf) -> String {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .and_then(|d| chrono::DateTime::<Utc>::from_timestamp(d.as_secs() as i64, 0))
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_default()
}

fn is_internal_workspace_file(name: &str) -> bool {
    name.starts_with(".paper_composer_")
}

fn is_local_pdf_file(path: &PathBuf, name: &str) -> bool {
    if name.to_lowercase().ends_with(".pdf") {
        return true;
    }
    let mut header = [0_u8; 5];
    fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map(|_| &header == b"%PDF-")
        .unwrap_or(false)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect()
}

fn query_param(query: &str, key: &str) -> Option<String> {
    form_urlencoded::parse(query.as_bytes())
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

fn request_line(request: &str) -> Option<(&str, &str)> {
    let mut parts = request.lines().next()?.split_whitespace();
    let method = parts.next()?;
    let target = parts.next()?;
    Some((method, target))
}

fn split_target(target: &str) -> (&str, &str) {
    target.split_once('?').unwrap_or((target, ""))
}

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buffer = [0; 8192];
    let mut bytes = Vec::new();
    loop {
        let n = stream.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..n]);
        if let Some(header_end) = find_header_end(&bytes) {
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            let total = header_end + 4 + content_length;
            while bytes.len() < total {
                let n = stream.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..n]);
            }
            break;
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn open_file(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    #[derive(Deserialize)]
    struct Req {
        path: String,
    }
    let req: Req = match serde_json::from_str(body) {
        Ok(r) => r,
        Err(err) => {
            return respond_json(
                stream,
                400,
                &serde_json::json!({"ok": false, "message": err.to_string()}),
            );
        }
    };
    let path = std::path::PathBuf::from(req.path.trim());
    if !path.exists() {
        return respond_json(
            stream,
            404,
            &serde_json::json!({"ok": false, "message": format!("檔案不存在：{}", path.display())}),
        );
    }
    let result = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(&path).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/C", "start", "", &path.to_string_lossy()]).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(&path).spawn()
    };
    match result {
        Ok(_) => respond_json(stream, 200, &serde_json::json!({"ok": true, "message": "已開啟"})),
        Err(err) => respond_json(
            stream,
            500,
            &serde_json::json!({"ok": false, "message": err.to_string()}),
        ),
    }
}

fn colors_get(stream: &mut TcpStream) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    let path = data_dir.join("color.json");
    let body = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
    respond(stream, 200, "application/json; charset=utf-8", &body)
}

fn colors_post(stream: &mut TcpStream, body: &str) -> std::io::Result<()> {
    let data_dir = default_data_dir();
    if let Err(err) = fs::create_dir_all(&data_dir) {
        return respond_error(stream, err);
    }
    let value: serde_json::Value = serde_json::from_str(body)
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    let pretty = match serde_json::to_string_pretty(&value) {
        Ok(s) => s,
        Err(err) => return respond_error(stream, std::io::Error::new(std::io::ErrorKind::Other, err)),
    };
    if let Err(err) = fs::write(data_dir.join("color.json"), pretty) {
        return respond_error(stream, err);
    }
    respond_json(stream, 200, &serde_json::json!({"ok": true, "message": "Colors saved"}))
}

fn respond_json<T: Serialize>(stream: &mut TcpStream, status: u16, value: &T) -> std::io::Result<()> {
    let body = serde_json::to_string(value)?;
    respond(stream, status, "application/json; charset=utf-8", &body)
}

fn respond_text(stream: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    respond(stream, status, "text/plain; charset=utf-8", body)
}

fn respond_error(stream: &mut TcpStream, err: std::io::Error) -> std::io::Result<()> {
    respond_json(
        stream,
        500,
        &StatusResponse {
            ok: false,
            configured: false,
            authenticated: false,
            message: err.to_string(),
            settings_txt_path: None,
        },
    )
}

fn respond(stream: &mut TcpStream, status: u16, content_type: &str, body: &str) -> std::io::Result<()> {
    let status_text = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\n\r\n{body}",
        body.as_bytes().len()
    );
    stream.write_all(response.as_bytes())
}
