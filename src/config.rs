use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DriveSettings {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_port: u16,
    pub scope: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at_epoch: i64,
}

impl Default for DriveSettings {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_port: 53682,
            scope: "https://www.googleapis.com/auth/drive".to_owned(),
            access_token: String::new(),
            refresh_token: String::new(),
            expires_at_epoch: 0,
        }
    }
}

impl DriveSettings {
    pub fn configured(&self) -> bool {
        !self.client_id.trim().is_empty() && !self.client_secret.trim().is_empty()
    }

    pub fn authenticated(&self) -> bool {
        !self.access_token.trim().is_empty() || !self.refresh_token.trim().is_empty()
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/oauth", self.redirect_port)
    }
}

pub fn drive_settings_path(data_dir: &Path) -> PathBuf {
    data_dir.join("drive_settings.json")
}

pub fn load_drive_settings(data_dir: &Path) -> DriveSettings {
    let path = drive_settings_path(data_dir);
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_drive_settings(data_dir: &Path, settings: &DriveSettings) -> std::io::Result<()> {
    fs::create_dir_all(data_dir)?;
    let text = serde_json::to_string_pretty(settings)?;
    fs::write(drive_settings_path(data_dir), text)
}
