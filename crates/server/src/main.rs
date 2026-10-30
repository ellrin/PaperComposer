//! PaperComposer local server.
//!
//! Binds to 127.0.0.1 only. Finds the writing profile in this order:
//! 1. `PAPERCOMPOSER_PROFILE` environment variable
//! 2. `profile_path` in `<config dir>/PaperComposer/config.toml`
//! 3. the bundled `profile-template/` (demo mode)
//!
//! The config file lives in the OS config folder, never inside a repository,
//! so a personal profile path is never committed by accident.

use axum::{
    extract::{Query as AxQuery, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use pc_core::{check_text, search, Issue, Profile, Query};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

struct Loaded {
    profile: Option<Profile>,
    issues: Vec<Issue>,
    error: Option<String>,
    path: PathBuf,
    demo: bool,
}

type Shared = Arc<RwLock<Loaded>>;

#[derive(Deserialize, Default)]
struct Config {
    profile_path: Option<PathBuf>,
}

fn repo_root() -> PathBuf {
    // crates/server -> repository root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn resolve_profile_path() -> (PathBuf, bool) {
    if let Ok(p) = std::env::var("PAPERCOMPOSER_PROFILE") {
        return (PathBuf::from(p), false);
    }
    if let Some(cfg) = dirs::config_dir().map(|d| d.join("PaperComposer/config.toml")) {
        if let Ok(raw) = std::fs::read_to_string(&cfg) {
            if let Some(p) = toml::from_str::<Config>(&raw).unwrap_or_default().profile_path {
                return (p, false);
            }
        }
    }
    let demo = repo_root().join("profile-template");
    (demo.canonicalize().unwrap_or(demo), true)
}

fn load() -> Loaded {
    let (path, demo) = resolve_profile_path();
    match Profile::load(&path) {
        Ok((profile, issues)) => Loaded { profile: Some(profile), issues, error: None, path, demo },
        Err(e) => Loaded { profile: None, issues: vec![], error: Some(e), path, demo },
    }
}

#[derive(Serialize)]
struct ProfileInfo {
    name: Option<String>,
    path: String,
    demo: bool,
    error: Option<String>,
    phrases: usize,
    vocabulary: usize,
    rules: usize,
    sections: Vec<String>,
    functions: Vec<String>,
    issues: Vec<Issue>,
}

async fn profile_info(State(s): State<Shared>) -> Json<ProfileInfo> {
    let l = s.read().await;
    let (mut sections, mut functions) = (Vec::new(), Vec::new());
    if let Some(p) = &l.profile {
        for ph in &p.phrases {
            if !sections.contains(&ph.section) {
                sections.push(ph.section.clone());
            }
            if !functions.contains(&ph.function) {
                functions.push(ph.function.clone());
            }
        }
    }
    functions.sort();
    Json(ProfileInfo {
        name: l.profile.as_ref().map(|p| p.manifest.name.clone()),
        path: l.path.display().to_string(),
        demo: l.demo,
        error: l.error.clone(),
        phrases: l.profile.as_ref().map_or(0, |p| p.phrases.len()),
        vocabulary: l.profile.as_ref().map_or(0, |p| p.vocabulary.len()),
        rules: l.profile.as_ref().map_or(0, |p| p.rules.len()),
        sections,
        functions,
        issues: l.issues.clone(),
    })
}

/// Rule id -> message, so the UI can name soft rules that are still pending.
async fn rule_messages(State(s): State<Shared>) -> Json<std::collections::BTreeMap<String, String>> {
    let l = s.read().await;
    Json(
        l.profile
            .iter()
            .flat_map(|p| &p.rules)
            .map(|r| (r.id.clone(), if r.message.is_empty() { r.check.clone() } else { r.message.clone() }))
            .collect(),
    )
}

async fn reload(State(s): State<Shared>) -> impl IntoResponse {
    *s.write().await = load();
    StatusCode::NO_CONTENT
}

async fn phrases(State(s): State<Shared>, AxQuery(q): AxQuery<Query>) -> impl IntoResponse {
    let l = s.read().await;
    match &l.profile {
        Some(p) => Json(search(&p.phrases, &q)).into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, "profile not loaded").into_response(),
    }
}

#[derive(Deserialize)]
struct CheckBody {
    text: String,
    section: Option<String>,
}

async fn check(State(s): State<Shared>, Json(b): Json<CheckBody>) -> impl IntoResponse {
    let l = s.read().await;
    match &l.profile {
        Some(p) => Json(check_text(p, &b.text, b.section.as_deref())).into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, "profile not loaded").into_response(),
    }
}

#[tokio::main]
async fn main() {
    let state: Shared = Arc::new(RwLock::new(load()));
    {
        let l = state.read().await;
        let mode = if l.demo { " (demo profile)" } else { "" };
        match &l.error {
            Some(e) => eprintln!("profile: {} — {e}", l.path.display()),
            None => eprintln!("profile: {}{mode}, {} issue(s)", l.path.display(), l.issues.len()),
        }
    }

    let ui = std::env::var("PAPERCOMPOSER_UI")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo_root().join("ui"));

    let app = Router::new()
        .route("/api/profile", get(profile_info))
        .route("/api/profile/rules", get(rule_messages))
        .route("/api/profile/reload", post(reload))
        .route("/api/phrases", get(phrases))
        .route("/api/check", post(check))
        .fallback_service(ServeDir::new(ui))
        .with_state(state);

    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(7878);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("PaperComposer on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("server");
}
