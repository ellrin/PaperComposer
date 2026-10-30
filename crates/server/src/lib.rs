//! PaperComposer local server, used both by `cargo run -p pc-server` (browser
//! dev mode) and by the desktop app, which starts it in-process and points
//! its window at it.
//!
//! Binds to 127.0.0.1 only. Finds the writing profile in this order:
//! 1. `PAPERCOMPOSER_WORKSPACE` (private repo root; profile in `profile/`)
//! 2. `PAPERCOMPOSER_PROFILE` (a profile folder on its own)
//! 3. `workspace_path` or `profile_path` in `<config dir>/PaperComposer/config.toml`
//! 4. the bundled `workspace-template/` (demo mode)
//!
//! The config file lives in the OS config folder, never inside a repository,
//! so a personal profile path is never committed by accident.

pub mod engine;

use axum::{
    extract::{Query as AxQuery, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use engine::{Engine, EngineConfig};
use pc_core::compose::{self, ComposeRequest, Draft, DraftCheck, Prompt};
use pc_core::{check_text, search, Hit, Issue, Profile, Query};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

/// Where the UI and the demo profile live. The desktop app passes its bundled
/// resource folder; dev mode uses the repository.
#[derive(Clone)]
pub struct Paths {
    pub ui: PathBuf,
    pub demo_workspace: PathBuf,
}

impl Paths {
    pub fn from_repo() -> Paths {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = root.canonicalize().unwrap_or(root);
        Paths { ui: root.join("ui"), demo_workspace: root.join("workspace-template") }
    }
}

struct Loaded {
    profile: Option<Profile>,
    issues: Vec<Issue>,
    error: Option<String>,
    /// Profile folder.
    path: PathBuf,
    /// Workspace root (the private repository), when one is used.
    workspace: Option<PathBuf>,
    demo: bool,
}

struct AppState {
    loaded: RwLock<Loaded>,
    engine: Engine,
    paths: Paths,
}

type Shared = Arc<AppState>;

#[derive(Deserialize, Default)]
struct Config {
    /// Root of the private repository: profile/, publishers/, papers/.
    workspace_path: Option<PathBuf>,
    /// Profile folder on its own (when not using a workspace).
    profile_path: Option<PathBuf>,
    engine: Option<EngineConfig>,
}

fn read_config() -> Config {
    dirs::config_dir()
        .map(|d| d.join("PaperComposer/config.toml"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| toml::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Resolution order: PAPERCOMPOSER_WORKSPACE, PAPERCOMPOSER_PROFILE,
/// config workspace_path, config profile_path, bundled demo workspace.
fn locate(paths: &Paths) -> (PathBuf, Option<PathBuf>, bool) {
    let env = |k| std::env::var(k).ok().map(PathBuf::from);
    if let Some(ws) = env("PAPERCOMPOSER_WORKSPACE") {
        return (ws.join("profile"), Some(ws), false);
    }
    if let Some(p) = env("PAPERCOMPOSER_PROFILE") {
        return (p, None, false);
    }
    let cfg = read_config();
    if let Some(ws) = cfg.workspace_path {
        return (ws.join("profile"), Some(ws), false);
    }
    if let Some(p) = cfg.profile_path {
        return (p, None, false);
    }
    (paths.demo_workspace.join("profile"), Some(paths.demo_workspace.clone()), true)
}

fn load(paths: &Paths) -> Loaded {
    let (path, workspace, demo) = locate(paths);
    match Profile::load(&path) {
        Ok((profile, issues)) => Loaded { profile: Some(profile), issues, error: None, path, workspace, demo },
        Err(e) => Loaded { profile: None, issues: vec![], error: Some(e), path, workspace, demo },
    }
}

#[derive(Serialize)]
struct WorkspaceInfo {
    root: Option<String>,
    demo: bool,
    papers: Vec<String>,
    publishers: Vec<String>,
}

fn subdirs(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    out.sort();
    out
}

async fn workspace_info(State(s): State<Shared>) -> Json<WorkspaceInfo> {
    let l = s.loaded.read().await;
    let ws = l.workspace.as_deref();
    Json(WorkspaceInfo {
        root: ws.map(|w| w.display().to_string()),
        demo: l.demo,
        papers: ws.map(|w| subdirs(&w.join("papers"))).unwrap_or_default(),
        publishers: ws.map(|w| subdirs(&w.join("publishers"))).unwrap_or_default(),
    })
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
    let l = s.loaded.read().await;
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
    let l = s.loaded.read().await;
    Json(
        l.profile
            .iter()
            .flat_map(|p| &p.rules)
            .map(|r| (r.id.clone(), if r.message.is_empty() { r.check.clone() } else { r.message.clone() }))
            .collect(),
    )
}

async fn reload(State(s): State<Shared>) -> impl IntoResponse {
    *s.loaded.write().await = load(&s.paths);
    StatusCode::NO_CONTENT
}

async fn phrases(State(s): State<Shared>, AxQuery(q): AxQuery<Query>) -> impl IntoResponse {
    let l = s.loaded.read().await;
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
    let l = s.loaded.read().await;
    match &l.profile {
        Some(p) => Json(check_text(p, &b.text, b.section.as_deref())).into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, "profile not loaded").into_response(),
    }
}

async fn engine_status(State(s): State<Shared>) -> impl IntoResponse {
    Json(s.engine.status().await)
}

#[derive(Serialize)]
struct ComposeReply {
    /// Patterns offered to the engine (shown even when no engine is connected).
    references: Vec<Hit>,
    draft: Option<Draft>,
    check: Option<DraftCheck>,
    /// Set when no engine ran, or it failed.
    error: Option<String>,
    /// The exact prompt, for inspection.
    prompt: Prompt,
}

async fn compose_paragraph(State(s): State<Shared>, Json(req): Json<ComposeRequest>) -> impl IntoResponse {
    let (points, refs, prompt) = {
        let l = s.loaded.read().await;
        let Some(p) = &l.profile else {
            return (StatusCode::SERVICE_UNAVAILABLE, "profile not loaded").into_response();
        };
        let points = compose::parse_points(&req.points);
        let refs = compose::reference_phrases(p, &req, &points);
        let prompt = compose::build_prompt(p, &req, &points, &refs);
        (points, refs, prompt)
    };
    if points.is_empty() {
        return (StatusCode::BAD_REQUEST, "no points given").into_response();
    }
    if !s.engine.configured() {
        return Json(ComposeReply {
            references: refs,
            draft: None,
            check: None,
            error: Some("no engine".into()),
            prompt,
        })
        .into_response();
    }

    // One retry when the reply is not valid JSON or fails validation.
    let mut feedback: Option<String> = None;
    let mut last_err = None;
    for _ in 0..2 {
        let reply = match s.engine.complete(&prompt, feedback.as_deref()).await {
            Ok(r) => r,
            Err(e) => {
                last_err = Some(e);
                break;
            }
        };
        match compose::parse_draft(&reply) {
            Ok(mut draft) => {
                let l = s.loaded.read().await;
                let Some(p) = &l.profile else { break };
                let check = compose::check_draft(p, &req, &points, &refs, &mut draft);
                if check.ok() || feedback.is_some() {
                    return Json(ComposeReply { references: refs, draft: Some(draft), check: Some(check), error: None, prompt })
                        .into_response();
                }
                feedback = Some(format!("Your previous answer had problems: {}. Fix them.", check.problems.join("; ")));
            }
            Err(e) => {
                last_err = Some(format!("unreadable reply: {e}"));
                feedback = Some("Reply with the JSON object only.".into());
            }
        }
    }
    Json(ComposeReply { references: refs, draft: None, check: None, error: last_err, prompt }).into_response()
}

pub fn router(paths: Paths) -> Router {
    let engine = Engine::new(EngineConfig::resolve(read_config().engine));
    let state: Shared = Arc::new(AppState { loaded: RwLock::new(load(&paths)), engine, paths: paths.clone() });
    Router::new()
        .route("/api/workspace", get(workspace_info))
        .route("/api/profile", get(profile_info))
        .route("/api/profile/rules", get(rule_messages))
        .route("/api/profile/reload", post(reload))
        .route("/api/phrases", get(phrases))
        .route("/api/check", post(check))
        .route("/api/engine", get(engine_status))
        .route("/api/compose", post(compose_paragraph))
        .fallback_service(ServeDir::new(paths.ui))
        .with_state(state)
}

/// Bind on 127.0.0.1 (port 0 = any free port) and return the bound address
/// plus a future that serves until dropped.
pub async fn bind(
    paths: Paths,
    port: u16,
) -> std::io::Result<(SocketAddr, impl std::future::Future<Output = std::io::Result<()>>)> {
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?;
    let addr = listener.local_addr()?;
    let app = router(paths);
    Ok((addr, async move { axum::serve(listener, app).await }))
}
