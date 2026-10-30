//! Claude (Anthropic Messages API).
//!
//! The API key never touches a file: it comes from `ANTHROPIC_API_KEY` or the
//! OS keychain (macOS Keychain, Windows Credential Manager). The chosen model
//! name is a preference and lives in config.toml.

use pc_core::compose::Prompt;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "PaperComposer";
const ACCOUNT: &str = "anthropic-api-key";
pub const DEFAULT_MODEL: &str = "claude-sonnet-5-5";
pub const MODELS: &[&str] = &["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"];

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum KeySource {
    Env,
    Keychain,
}

pub struct Claude {
    http: reqwest::Client,
    base: String,
    pub model: String,
}

#[derive(Serialize)]
pub struct ClaudeStatus {
    pub configured: bool,
    pub source: Option<KeySource>,
    pub model: String,
    pub models: &'static [&'static str],
}

fn keychain() -> Option<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).ok()
}

/// The key and where it came from. Env wins so a shell can override.
fn key() -> Option<(String, KeySource)> {
    if let Ok(k) = std::env::var("ANTHROPIC_API_KEY") {
        if !k.trim().is_empty() {
            return Some((k, KeySource::Env));
        }
    }
    keychain()?.get_password().ok().map(|k| (k, KeySource::Keychain))
}

/// Store (or with an empty string, remove) the key in the OS keychain.
pub fn save_key(k: &str) -> Result<(), String> {
    let entry = keychain().ok_or("keychain unavailable on this system")?;
    if k.trim().is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    } else {
        entry.set_password(k.trim()).map_err(|e| e.to_string())
    }
}

#[derive(Serialize)]
struct Req<'a> {
    model: &'a str,
    max_tokens: u32,
    temperature: f32,
    system: &'a str,
    messages: [Msg<'a>; 1],
}
#[derive(Serialize)]
struct Msg<'a> {
    role: &'a str,
    content: &'a str,
}
#[derive(Deserialize)]
struct Resp {
    content: Vec<Block>,
}
#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}
#[derive(Deserialize)]
struct ApiError {
    error: ApiErrorBody,
}
#[derive(Deserialize)]
struct ApiErrorBody {
    message: String,
}

impl Claude {
    pub fn new(model: Option<String>) -> Claude {
        Claude {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(180))
                .build()
                .expect("http client"),
            // Overridable for testing against a local stub.
            base: std::env::var("PAPERCOMPOSER_CLAUDE_BASE_URL").unwrap_or_else(|_| "https://api.anthropic.com".into()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.into()),
        }
    }

    pub fn status(&self) -> ClaudeStatus {
        let k = key();
        ClaudeStatus { configured: k.is_some(), source: k.map(|(_, s)| s), model: self.model.clone(), models: MODELS }
    }

    pub async fn complete(&self, prompt: &Prompt) -> Result<String, String> {
        let (k, _) = key().ok_or("Claude 尚未設定 API key")?;
        let body = Req {
            model: &self.model,
            max_tokens: 4096,
            temperature: 0.2,
            system: &prompt.system,
            messages: [Msg { role: "user", content: &prompt.user }],
        };
        let resp = self
            .http
            .post(format!("{}/v1/messages", self.base.trim_end_matches('/')))
            .header("x-api-key", k)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("無法連線到 Claude：{e}"))?;
        let status = resp.status();
        if !status.is_success() {
            let msg = resp.json::<ApiError>().await.map(|e| e.error.message).unwrap_or_default();
            return Err(format!("Claude 回傳 {status}：{msg}"));
        }
        let r: Resp = resp.json().await.map_err(|e| format!("Claude 回覆無法解析：{e}"))?;
        Ok(r.content.into_iter().filter(|b| b.kind == "text").map(|b| b.text).collect::<Vec<_>>().join(""))
    }
}
