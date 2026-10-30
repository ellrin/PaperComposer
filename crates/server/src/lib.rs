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

pub mod claude;
pub mod engine;

use axum::{
    extract::{Path as AxPath, Query as AxQuery, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post, put},
    Router,
};
use claude::Claude;
use engine::{Engine, EngineConfig};
use pc_core::compose::{self, ComposeRequest, Draft, DraftCheck, Prompt};
use pc_core::paper::{self, Section};
use pc_core::table::{self, Table};
use pc_core::{check_text, decompose, search, Hit, Issue, Profile, Query};
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
    claude: RwLock<Claude>,
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
    claude: Option<ClaudeConfig>,
}

#[derive(Deserialize, Default)]
struct ClaudeConfig {
    model: Option<String>,
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("PaperComposer/config.toml"))
}

fn read_config() -> Config {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|raw| toml::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Set one `[table] key = value` in config.toml, keeping everything else.
fn write_config_value(table: &str, key: &str, value: &str) -> Result<(), String> {
    let path = config_path().ok_or("no config folder on this system")?;
    let mut doc: toml::Table = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or_default();
    let section = doc
        .entry(table.to_string())
        .or_insert_with(|| toml::Value::Table(Default::default()));
    if let toml::Value::Table(t) = section {
        t.insert(key.to_string(), toml::Value::String(value.to_string()));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, toml::to_string(&doc).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// The demo workspace is copied into the app's data folder on first use, so
/// trying the app never edits the bundled (or repository) copy.
fn demo_workspace(paths: &Paths) -> PathBuf {
    let Some(target) = dirs::data_dir().map(|d| d.join("PaperComposer/demo-workspace")) else {
        return paths.demo_workspace.clone();
    };
    if !target.exists() && copy_dir(&paths.demo_workspace, &target).is_err() {
        return paths.demo_workspace.clone();
    }
    target
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
    let ws = demo_workspace(paths);
    (ws.join("profile"), Some(ws), true)
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

// ── Papers ─────────────────────────────────────────────

fn bad(msg: &str) -> axum::response::Response {
    (StatusCode::BAD_REQUEST, msg.to_string()).into_response()
}

async fn paper_dir(s: &Shared, name: &str) -> Result<PathBuf, axum::response::Response> {
    if !paper::safe_name(name) {
        return Err(bad("invalid paper name"));
    }
    let l = s.loaded.read().await;
    let ws = l.workspace.clone().ok_or_else(|| bad("no workspace"))?;
    let dir = ws.join("papers").join(name);
    if !dir.is_dir() {
        return Err((StatusCode::NOT_FOUND, "paper not found").into_response());
    }
    Ok(dir)
}

async fn get_paper(State(s): State<Shared>, AxPath(name): AxPath<String>) -> impl IntoResponse {
    match paper_dir(&s, &name).await {
        Ok(dir) => Json(paper::load_paper(&dir)).into_response(),
        Err(r) => r,
    }
}

#[derive(Deserialize)]
struct SectionBody {
    title: String,
    paragraphs: Vec<String>,
}

async fn put_section(
    State(s): State<Shared>,
    AxPath((name, file)): AxPath<(String, String)>,
    Json(b): Json<SectionBody>,
) -> impl IntoResponse {
    if !paper::safe_name(&file) || !file.ends_with(".tex") {
        return bad("invalid section file");
    }
    let dir = match paper_dir(&s, &name).await {
        Ok(d) => d,
        Err(r) => return r,
    };
    let section = Section { file: file.clone(), title: b.title, paragraphs: b.paragraphs };
    let path = dir.join("sections").join(&file);
    if let Err(e) = std::fs::create_dir_all(dir.join("sections")).and_then(|_| paper::write_atomic(&path, &paper::render_section(&section))) {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }
    // Return what is now on disk, re-read, so the UI shows the saved form.
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    Json(paper::parse_section(&file, &text)).into_response()
}

// ── Database tables ────────────────────────────────────

async fn get_table(State(s): State<Shared>, AxPath(name): AxPath<String>) -> impl IntoResponse {
    let dir = s.loaded.read().await.path.clone();
    match table::read_table(&dir, &name) {
        Ok(t) => Json(t).into_response(),
        Err(e) => bad(&e),
    }
}

async fn put_table(State(s): State<Shared>, AxPath(name): AxPath<String>, Json(t): Json<Table>) -> impl IntoResponse {
    let dir = s.loaded.read().await.path.clone();
    if let Err(e) = table::write_table(&dir, &name, &t) {
        return bad(&e);
    }
    let fresh = load(&s.paths);
    let issues: Vec<Issue> = fresh.issues.iter().filter(|i| i.file == name).cloned().collect();
    *s.loaded.write().await = fresh;
    Json(issues).into_response()
}

// ── Claude ─────────────────────────────────────────────

async fn claude_status(State(s): State<Shared>) -> impl IntoResponse {
    Json(s.claude.read().await.status())
}

#[derive(Deserialize)]
struct ClaudeSettings {
    /// Some("") removes the stored key; None leaves it.
    key: Option<String>,
    model: Option<String>,
}

async fn claude_settings(State(s): State<Shared>, Json(b): Json<ClaudeSettings>) -> impl IntoResponse {
    if let Some(k) = &b.key {
        if let Err(e) = claude::save_key(k) {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("無法存入系統金鑰圈：{e}")).into_response();
        }
    }
    if let Some(m) = b.model.filter(|m| claude::MODELS.contains(&m.as_str())) {
        if let Err(e) = write_config_value("claude", "model", &m) {
            return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
        }
        s.claude.write().await.model = m;
    }
    Json(s.claude.read().await.status()).into_response()
}

#[derive(Deserialize)]
struct DecomposeBody {
    text: String,
    section: Option<String>,
}

async fn decompose_text(State(s): State<Shared>, Json(b): Json<DecomposeBody>) -> impl IntoResponse {
    if b.text.trim().is_empty() {
        return bad("no text");
    }
    let prompt = {
        let l = s.loaded.read().await;
        let Some(p) = &l.profile else { return bad("profile not loaded") };
        decompose::build_prompt(p, &b.text, b.section.as_deref())
    };
    // Claude when configured, otherwise the local engine.
    let reply = {
        let c = s.claude.read().await;
        if c.status().configured {
            c.complete(&prompt).await
        } else if s.engine.configured() {
            s.engine.complete(&prompt, None).await
        } else {
            Err("需要先在設定中連接 Claude 或本機模型".into())
        }
    };
    let reply = match reply {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY, e).into_response(),
    };
    let l = s.loaded.read().await;
    let Some(p) = &l.profile else { return bad("profile not loaded") };
    match decompose::parse_proposals(p, &reply) {
        Ok(props) => Json(props).into_response(),
        Err(e) => (StatusCode::BAD_GATEWAY, format!("回覆無法拆解：{e}")).into_response(),
    }
}

pub fn router(paths: Paths) -> Router {
    let cfg = read_config();
    let engine = Engine::new(EngineConfig::resolve(cfg.engine));
    let claude = Claude::new(cfg.claude.and_then(|c| c.model));
    let state: Shared = Arc::new(AppState {
        loaded: RwLock::new(load(&paths)),
        engine,
        claude: RwLock::new(claude),
        paths: paths.clone(),
    });
    Router::new()
        .route("/api/papers/{name}", get(get_paper))
        .route("/api/papers/{name}/sections/{file}", put(put_section))
        .route("/api/tables/{name}", get(get_table).put(put_table))
        .route("/api/claude", get(claude_status))
        .route("/api/settings/claude", post(claude_settings))
        .route("/api/decompose", post(decompose_text))
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
