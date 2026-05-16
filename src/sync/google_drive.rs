use crate::config::DriveSettings;
use chrono::Utc;
use serde::Deserialize;
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::Path,
    time::Duration,
};
use url::Url;

#[allow(dead_code)]
pub trait GoogleDriveClient {
    fn download_file(&self, file_id: &str, target_path: &Path) -> std::io::Result<()>;
    fn upload_to_folder(&self, local_path: &Path, folder_id: &str, file_name: &str) -> std::io::Result<()>;
    fn upload_file_to_folder(&self, local_path: &Path, folder_id: &str, file_name: &str, mime_type: &str) -> std::io::Result<()>;
    fn list_json_files(&self, folder_id: &str) -> std::io::Result<Vec<DriveFile>>;
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct DriveFile {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    pub modified_time: String,
}

pub struct GoogleDriveStub;

impl GoogleDriveClient for GoogleDriveStub {
    fn download_file(&self, _file_id: &str, _target_path: &Path) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Google Drive API is not configured yet",
        ))
    }

    fn upload_to_folder(&self, _local_path: &Path, _folder_id: &str, _file_name: &str) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Google Drive API is not configured yet",
        ))
    }

    fn upload_file_to_folder(&self, _local_path: &Path, _folder_id: &str, _file_name: &str, _mime_type: &str) -> std::io::Result<()> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Google Drive API is not configured yet",
        ))
    }

    fn list_json_files(&self, _folder_id: &str) -> std::io::Result<Vec<DriveFile>> {
        Ok(Vec::new())
    }
}

pub struct GoogleDriveRestClient {
    settings: DriveSettings,
}

impl GoogleDriveRestClient {
    pub fn new(settings: DriveSettings) -> Self {
        Self { settings }
    }

    fn access_token(&self) -> std::io::Result<String> {
        if self.settings.access_token.trim().is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Google Drive is not authenticated",
            ));
        }
        Ok(self.settings.access_token.clone())
    }
}

impl GoogleDriveClient for GoogleDriveRestClient {
    fn download_file(&self, file_id: &str, target_path: &Path) -> std::io::Result<()> {
        let token = self.access_token()?;
        let url = format!("https://www.googleapis.com/drive/v3/files/{file_id}?alt=media");
        let response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .call()
            .map_err(to_io_error)?;
        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(to_io_error)?;
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, bytes)
    }

    fn upload_to_folder(&self, local_path: &Path, folder_id: &str, file_name: &str) -> std::io::Result<()> {
        self.upload_file_to_folder(local_path, folder_id, file_name, "application/json; charset=UTF-8")
    }

    fn upload_file_to_folder(&self, local_path: &Path, folder_id: &str, file_name: &str, mime_type: &str) -> std::io::Result<()> {
        let token = self.access_token()?;
        let existing_id = self.find_file_id_in_folder(folder_id, file_name)?;
        let metadata = if existing_id.is_some() {
            serde_json::json!({ "name": file_name })
        } else {
            serde_json::json!({
                "name": file_name,
                "parents": [folder_id],
            })
        };
        let body = fs::read(local_path)?;
        let boundary = "papercomposer-boundary";
        let mut multipart = Vec::new();
        multipart.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{}\r\n--{boundary}\r\nContent-Type: {mime_type}\r\n\r\n",
                metadata
            )
            .as_bytes(),
        );
        multipart.extend_from_slice(&body);
        multipart.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let url = if let Some(file_id) = existing_id.as_ref() {
            format!(
                "https://www.googleapis.com/upload/drive/v3/files/{file_id}?uploadType=multipart"
            )
        } else {
            "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart".to_owned()
        };
        let request = if existing_id.is_some() {
            ureq::patch(&url)
        } else {
            ureq::post(&url)
        };
        request
            .set("Authorization", &format!("Bearer {token}"))
            .set(
                "Content-Type",
                &format!("multipart/related; boundary={boundary}"),
            )
            .send_bytes(&multipart)
            .map_err(to_io_error)?;
        Ok(())
    }

    fn list_json_files(&self, folder_id: &str) -> std::io::Result<Vec<DriveFile>> {
        let token = self.access_token()?;
        let query =
            format!("'{folder_id}' in parents and trashed = false and name contains '.json'");
        let response = ureq::get("https://www.googleapis.com/drive/v3/files")
            .set("Authorization", &format!("Bearer {token}"))
            .query("q", &query)
            .query("fields", "files(id,name,mimeType,modifiedTime)")
            .call()
            .map_err(to_io_error)?;
        let list: DriveFileList = response.into_json().map_err(to_io_error)?;
        Ok(list
            .files
            .into_iter()
            .map(|file| DriveFile {
                id: file.id,
                name: file.name,
                mime_type: file.mime_type,
                modified_time: file.modified_time,
            })
            .collect())
    }
}

impl GoogleDriveRestClient {
    pub fn trash_file(&self, file_id: &str) -> std::io::Result<()> {
        let token = self.access_token()?;
        let url = format!("https://www.googleapis.com/drive/v3/files/{file_id}");
        ureq::patch(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json; charset=UTF-8")
            .send_string(r#"{"trashed":true}"#)
            .map_err(to_io_error)?;
        Ok(())
    }

    pub fn trash_json_files_in_folder(&self, folder_id: &str) -> std::io::Result<usize> {
        let files = self.list_files_in_folder(folder_id)?;
        let mut trashed = 0;
        for file in files {
            let lower = file.name.to_lowercase();
            let is_json = lower.ends_with(".json") || file.mime_type == "application/json";
            if is_json {
                self.trash_file(&file.id)?;
                trashed += 1;
            }
        }
        Ok(trashed)
    }

    fn find_file_id_in_folder(&self, folder_id: &str, name: &str) -> std::io::Result<Option<String>> {
        let token = self.access_token()?;
        let escaped_name = name.replace('\'', "\\'");
        let query = format!(
            "'{}' in parents and trashed = false and name = '{}'",
            folder_id, escaped_name
        );
        let response = ureq::get("https://www.googleapis.com/drive/v3/files")
            .set("Authorization", &format!("Bearer {token}"))
            .query("q", &query)
            .query("fields", "files(id,name)")
            .call()
            .map_err(to_io_error)?;
        let list: DriveFileList = response.into_json().map_err(to_io_error)?;
        Ok(list.files.into_iter().next().map(|file| file.id))
    }

    pub fn list_files_in_folder(&self, folder_id: &str) -> std::io::Result<Vec<DriveFile>> {
        let token = self.access_token()?;
        let query = format!("'{folder_id}' in parents and trashed = false");
        let response = ureq::get("https://www.googleapis.com/drive/v3/files")
            .set("Authorization", &format!("Bearer {token}"))
            .query("q", &query)
            .query("fields", "files(id,name,mimeType,modifiedTime)")
            .call()
            .map_err(to_io_error)?;
        let list: DriveFileList = response.into_json().map_err(to_io_error)?;
        Ok(list
            .files
            .into_iter()
            .map(|file| DriveFile {
                id: file.id,
                name: file.name,
                mime_type: file.mime_type,
                modified_time: file.modified_time,
            })
            .collect())
    }
}

pub fn authorization_url(settings: &DriveSettings) -> String {
    let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth").unwrap();
    url.query_pairs_mut()
        .append_pair("client_id", &settings.client_id)
        .append_pair("redirect_uri", &settings.redirect_uri())
        .append_pair("response_type", "code")
        .append_pair("scope", &settings.scope)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent");
    url.to_string()
}

pub fn run_local_oauth_flow(mut settings: DriveSettings) -> std::io::Result<DriveSettings> {
    let listener = TcpListener::bind(("127.0.0.1", settings.redirect_port))?;
    listener.set_nonblocking(false)?;
    let auth_url = authorization_url(&settings);
    let _ = webbrowser::open(&auth_url);

    let (mut stream, _) = listener.accept()?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut request = [0; 4096];
    let n = stream.read(&mut request)?;
    let request = String::from_utf8_lossy(&request[..n]);
    let code = parse_oauth_code(&request).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "OAuth redirect did not contain a code",
        )
    })?;
    let token = exchange_code_for_token(&settings, &code)?;
    settings.access_token = token.access_token;
    if let Some(refresh_token) = token.refresh_token {
        settings.refresh_token = refresh_token;
    }
    settings.expires_at_epoch = Utc::now().timestamp() + token.expires_in.unwrap_or(3600);

    let html = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<html><body><h2>Paper Composer connected to Google Drive.</h2><p>You can close this window.</p></body></html>";
    stream.write_all(html.as_bytes())?;
    Ok(settings)
}

pub fn refresh_access_token(mut settings: DriveSettings) -> std::io::Result<DriveSettings> {
    if settings.refresh_token.trim().is_empty() {
        return Ok(settings);
    }
    let response = ureq::post("https://oauth2.googleapis.com/token")
        .send_form(&[
            ("client_id", settings.client_id.as_str()),
            ("client_secret", settings.client_secret.as_str()),
            ("refresh_token", settings.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
        ])
        .map_err(to_io_error)?;
    let token: TokenResponse = response.into_json().map_err(to_io_error)?;
    settings.access_token = token.access_token;
    settings.expires_at_epoch = Utc::now().timestamp() + token.expires_in.unwrap_or(3600);
    Ok(settings)
}

fn exchange_code_for_token(settings: &DriveSettings, code: &str) -> std::io::Result<TokenResponse> {
    let response = ureq::post("https://oauth2.googleapis.com/token")
        .send_form(&[
            ("code", code),
            ("client_id", settings.client_id.as_str()),
            ("client_secret", settings.client_secret.as_str()),
            ("redirect_uri", settings.redirect_uri().as_str()),
            ("grant_type", "authorization_code"),
        ])
        .map_err(to_io_error)?;
    response.into_json().map_err(to_io_error)
}

fn parse_oauth_code(request: &str) -> Option<String> {
    let first_line = request.lines().next()?;
    let path = first_line.split_whitespace().nth(1)?;
    let url = Url::parse(&format!("http://127.0.0.1{path}")).ok()?;
    url.query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, value)| value.into_owned())
}

fn to_io_error(err: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, err.to_string())
}

#[derive(Deserialize)]
struct DriveFileList {
    files: Vec<DriveFileResponse>,
}

#[derive(Deserialize)]
struct DriveFileResponse {
    id: String,
    name: String,
    #[serde(rename = "mimeType", default)]
    mime_type: String,
    #[serde(rename = "modifiedTime", default)]
    modified_time: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}
