//! IPC commands. Keep these thin: translate the call, delegate to `otto-core`.

// Tauri deserializes every argument into an owned value and hands out `State` by value.
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;

use otto_core::agent::{AgentKind, Purpose};
use otto_core::agent_events;
use otto_core::claude_setup::{self, ClaudeSource, SetupItem};
use otto_core::doctor::{self, DoctorReport};
use otto_core::env::IsolatedEnv;
use otto_core::history::{self, PastConversation};
use otto_core::knowledge::{self, Area, KnowledgeEntry, KnowledgeError};
use otto_core::projects::{DiscoveredRepo, classify, find_repos};
use otto_core::provision::write_doctor_status;
use otto_core::pty::{PtyEvent, PtySession};
use otto_core::repo::{RemotePolicy, RepoRefusal, VerifiedRepo, verify_repo};
use otto_core::scrollback;
use otto_core::store::{Project, ProjectGroup, StoreError};
use otto_core::store::{SavedSession, SessionKind, SessionStatus};
use otto_core::workspace::Workspace;
use otto_core::workspace::WorkspaceLayout;
use otto_core::{AppInfo, AppInfoError};
use serde::Serialize;
use specta::Type;
use tauri::State;
use tauri::ipc::Channel;

use crate::sessions::SessionInfo;
use crate::state::AppState;

/// Errors any command can return, in words Mike can read.
#[derive(Debug, thiserror::Error, Serialize, Type)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum CommandError {
    #[error("{0}")]
    Invalid(String),
    #[error("There's no workspace called “{0}”.")]
    NotFound(String),
    /// A repo failed an isolation check. The message says which and why.
    #[error("{0}")]
    Refused(String),
    #[error("Something went wrong inside Otto: {0}")]
    Internal(String),
}

#[tauri::command]
#[specta::specta]
pub fn app_info() -> Result<AppInfo, AppInfoError> {
    AppInfo::current(env!("CARGO_PKG_VERSION"))
}

#[tauri::command]
#[specta::specta]
pub async fn list_workspaces(state: State<'_, AppState>) -> Result<Vec<Workspace>, CommandError> {
    state
        .store
        .workspaces()
        .await
        .map_err(|e| CommandError::Internal(e.to_string()))
}

/// Run the Identity Doctor for one workspace, record the verdict for `pre-push`,
/// and log it to the audit trail.
#[tauri::command]
#[specta::specta]
pub async fn run_identity_doctor(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<DoctorReport, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let layout = state.paths.workspace(&ws.id);
    let host = state.host.clone();
    let (ws2, layout2) = (ws.clone(), layout.clone());
    let report =
        tauri::async_runtime::spawn_blocking(move || doctor::diagnose(&ws2, &layout2, &host))
            .await
            .map_err(|e| CommandError::Internal(e.to_string()))?;
    write_doctor_status(&layout, report.push_allowed)
        .map_err(|e| CommandError::Internal(e.to_string()))?;
    let detail = serde_json::json!({
        "allGreen": report.all_green,
        "pushAllowed": report.push_allowed,
        "failing": report.checks.iter().filter(|c| c.status == doctor::CheckStatus::Fail).map(|c| &c.id).collect::<Vec<_>>(),
    });
    // Log the verdict only when it changes, so the audit trail doesn't grow with every check.
    let last = state
        .store
        .last_event_detail(&ws.id, "identity_doctor")
        .await
        .ok()
        .flatten();
    if last.as_deref() != Some(detail.to_string().as_str()) {
        let _ = state
            .store
            .record_event(Some(&ws.id), "identity_doctor", &detail)
            .await;
    }
    Ok(report)
}

/// Verify `path` for `ws` on a worker thread, logging refusals to the audit trail.
async fn verify(
    state: &AppState,
    ws: &Workspace,
    path: PathBuf,
    policy: RemotePolicy,
) -> Result<VerifiedRepo, CommandError> {
    let layout = state.paths.workspace(&ws.id);
    let env = IsolatedEnv::build(ws, &layout, &state.host);
    let ws2 = ws.clone();
    let verdict = tauri::async_runtime::spawn_blocking(move || {
        verify_repo(&ws2, &layout, &env, &path, policy)
    })
    .await
    .map_err(|e| CommandError::Internal(e.to_string()))?;
    verdict.map_err(|refusal| {
        let message = refusal.to_string();
        let store = state.store.clone();
        let id = ws.id.clone();
        tauri::async_runtime::spawn(async move {
            let detail = serde_json::json!({ "reason": message });
            let _ = store.record_event(Some(&id), "repo_refused", &detail).await;
        });
        CommandError::Refused(refusal.to_string())
    })
}

fn internal(e: impl std::fmt::Display) -> CommandError {
    CommandError::Internal(e.to_string())
}

/// Store refusals are Mike's to read; anything else is an internal failure.
fn store_error(e: StoreError) -> CommandError {
    match e {
        StoreError::Rejected(message) => CommandError::Refused(message),
        other => internal(other),
    }
}

/// Find git repos under `roots` (default: Desktop, Developer, Projects, code, dev, src)
/// and suggest a workspace for each from its GitHub owner.
#[tauri::command]
#[specta::specta]
pub async fn discover_projects(
    state: State<'_, AppState>,
    roots: Option<Vec<String>>,
) -> Result<Vec<DiscoveredRepo>, CommandError> {
    let home = state.host.home.clone();
    let roots: Vec<PathBuf> = roots.map_or_else(
        || {
            ["Desktop", "Developer", "Projects", "code", "dev", "src"]
                .iter()
                .map(|d| home.join(d))
                .filter(|p| p.is_dir())
                .collect()
        },
        |r| r.into_iter().map(PathBuf::from).collect(),
    );
    let workspaces = state.store.workspaces().await.map_err(internal)?;
    let host = state.host.clone();
    let mut found = tauri::async_runtime::spawn_blocking(move || {
        find_repos(&roots, 5)
            .iter()
            .map(|repo| classify(&host, repo, &workspaces))
            .collect::<Vec<_>>()
    })
    .await
    .map_err(internal)?;
    for repo in &mut found {
        repo.registered_to = state.store.repo_owner(&repo.path).await.map_err(internal)?;
    }
    Ok(found)
}

/// Classify one folder Mike picked by hand (outside the scanned locations).
#[tauri::command]
#[specta::specta]
pub async fn inspect_project(
    state: State<'_, AppState>,
    path: String,
) -> Result<DiscoveredRepo, CommandError> {
    let workspaces = state.store.workspaces().await.map_err(internal)?;
    let host = state.host.clone();
    let dir = PathBuf::from(&path);
    if !dir.join(".git").exists() {
        return Err(CommandError::Refused(format!(
            "{path} isn't a git repository. Run `git init` there first, or pick the repo's top folder."
        )));
    }
    let mut repo = tauri::async_runtime::spawn_blocking(move || classify(&host, &dir, &workspaces))
        .await
        .map_err(internal)?;
    repo.registered_to = state.store.repo_owner(&repo.path).await.map_err(internal)?;
    Ok(repo)
}

#[tauri::command]
#[specta::specta]
pub async fn list_projects(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Vec<Project>, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    state.store.projects(&ws.id).await.map_err(internal)
}

#[tauri::command]
#[specta::specta]
pub async fn list_project_groups(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Vec<ProjectGroup>, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    state
        .store
        .project_groups(&ws.id)
        .await
        .map_err(store_error)
}

#[tauri::command]
#[specta::specta]
pub async fn create_project_group(
    state: State<'_, AppState>,
    workspace_id: String,
    name: String,
) -> Result<ProjectGroup, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    state
        .store
        .create_project_group(&ws.id, &name)
        .await
        .map_err(store_error)
}

#[tauri::command]
#[specta::specta]
pub async fn rename_project_group(
    state: State<'_, AppState>,
    workspace_id: String,
    group_id: String,
    name: String,
) -> Result<(), CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    state
        .store
        .rename_project_group(&ws.id, &group_id, &name)
        .await
        .map_err(store_error)
}

/// Delete a group; its projects stay in the workspace, ungrouped.
#[tauri::command]
#[specta::specta]
pub async fn delete_project_group(
    state: State<'_, AppState>,
    workspace_id: String,
    group_id: String,
) -> Result<(), CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    state
        .store
        .delete_project_group(&ws.id, &group_id)
        .await
        .map_err(store_error)
}

#[tauri::command]
#[specta::specta]
pub async fn reorder_project_groups(
    state: State<'_, AppState>,
    workspace_id: String,
    group_ids: Vec<String>,
) -> Result<(), CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    state
        .store
        .reorder_project_groups(&ws.id, &group_ids)
        .await
        .map_err(store_error)
}

/// Name a project (empty goes back to the folder name). Only Otto's label changes.
#[tauri::command]
#[specta::specta]
pub async fn rename_project(
    state: State<'_, AppState>,
    workspace_id: String,
    path: String,
    name: String,
) -> Result<(), CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    state
        .store
        .rename_project(&ws.id, &path, &name)
        .await
        .map_err(store_error)
}

/// Put a project in one of its workspace's groups (`None` for no group).
#[tauri::command]
#[specta::specta]
pub async fn move_project(
    state: State<'_, AppState>,
    workspace_id: String,
    path: String,
    group_id: Option<String>,
) -> Result<(), CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    state
        .store
        .move_project(&ws.id, &path, group_id.as_deref())
        .await
        .map_err(store_error)
}

/// Register a repo to a workspace (SPEC §4.2). `local_only` means Mike chose the
/// workspace for a repo with no remote; pushes stay locked until a matching remote exists.
#[tauri::command]
#[specta::specta]
pub async fn register_repo(
    state: State<'_, AppState>,
    workspace_id: String,
    path: String,
    local_only: bool,
) -> Result<VerifiedRepo, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let policy = if local_only {
        RemotePolicy::AllowLocalOnly
    } else {
        RemotePolicy::RequireGithubRemote
    };
    let repo = verify(&state, &ws, PathBuf::from(path), policy).await?;
    if let Some(owner) = state.store.repo_owner(&repo.path).await.map_err(internal)? {
        if owner != ws.id.as_str() {
            return Err(CommandError::Refused(
                RepoRefusal::BelongsToOtherWorkspace(owner).to_string(),
            ));
        }
        return Ok(repo);
    }
    state
        .store
        .register_repo(&ws.id, &repo)
        .await
        .map_err(internal)?;
    Ok(repo)
}

/// The folder a session runs in: a project registered to `ws`, or the home folder.
async fn session_cwd(
    state: &AppState,
    ws: &Workspace,
    project_path: Option<&str>,
) -> Result<Option<PathBuf>, CommandError> {
    let Some(path) = project_path else {
        return Ok(None);
    };
    let owner = state.store.repo_owner(path).await.map_err(internal)?;
    if owner.as_deref() != Some(ws.id.as_str()) {
        return Err(CommandError::Refused(format!(
            "{path} isn't a {} project. Import it into {} first.",
            ws.name, ws.name
        )));
    }
    Ok(Some(PathBuf::from(path)))
}

/// Where a work session runs: a project of `ws`, re-verified every time so a token or
/// foreign remote added since registration is caught (ADR-007), or the home folder.
async fn verified_cwd(
    state: &AppState,
    ws: &Workspace,
    project_path: Option<&str>,
) -> Result<(PathBuf, bool), CommandError> {
    match session_cwd(state, ws, project_path).await? {
        Some(path) => {
            let repo = verify(state, ws, path, RemotePolicy::AllowLocalOnly).await?;
            Ok((PathBuf::from(repo.path), true))
        }
        None => Ok((state.host.home.clone(), false)),
    }
}

/// Earlier Claude conversations in a project (or General), newest first. Read only from
/// this workspace's own Claude config, so another workspace's history can't appear.
#[tauri::command]
#[specta::specta]
pub async fn list_past_conversations(
    state: State<'_, AppState>,
    workspace_id: String,
    project_path: Option<String>,
) -> Result<Vec<PastConversation>, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let cwd = session_cwd(&state, &ws, project_path.as_deref())
        .await?
        .unwrap_or_else(|| state.host.home.clone());
    let claude_dir = state.paths.workspace(&ws.id).claude_dir();
    tauri::async_runtime::spawn_blocking(move || history::list(&claude_dir, &cwd, 10))
        .await
        .map_err(internal)
}

/// Open an agent (or shell) in a real terminal inside the workspace's isolated
/// environment, optionally inside one of its projects. Output streams to `on_event`.
/// With `sign_in`, runs the CLI's own login flow. With `resume_id`, continues an earlier
/// Claude conversation from this workspace and folder.
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn open_session(
    state: State<'_, AppState>,
    workspace_id: String,
    agent: AgentKind,
    sign_in: bool,
    project_path: Option<String>,
    resume_id: Option<String>,
    cols: u16,
    rows: u16,
    on_event: Channel<PtyEvent>,
) -> Result<SessionInfo, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let layout = state.paths.workspace(&ws.id);

    let (cwd, in_project) = verified_cwd(&state, &ws, project_path.as_deref()).await?;

    // A resumed conversation must be one of *this* workspace's, started in *this* folder.
    let resumed = match resume_id {
        Some(id) if agent == AgentKind::Claude && !sign_in => {
            let claude_dir = layout.claude_dir();
            let dir = cwd.clone();
            let past = tauri::async_runtime::spawn_blocking(move || {
                history::list(&claude_dir, &dir, usize::MAX)
            })
            .await
            .map_err(internal)?;
            let found = past.into_iter().find(|c| c.id == id).ok_or_else(|| {
                CommandError::Refused(format!(
                    "That conversation isn't one of {}'s in this project.",
                    ws.name
                ))
            })?;
            if state.sessions.contains(&found.id) {
                return Err(CommandError::Refused(
                    "That conversation is already open.".into(),
                ));
            }
            Some(found)
        }
        Some(_) => {
            return Err(CommandError::Invalid(
                "Only Claude Code conversations can be resumed.".into(),
            ));
        }
        None => None,
    };

    // Notes saved outside Otto (for example by Ask) reach new sessions too.
    refresh_instructions(&state, &ws, &layout);
    let id = resumed
        .as_ref()
        .map_or_else(|| uuid::Uuid::new_v4().to_string(), |c| c.id.clone());
    let purpose = match (&resumed, sign_in) {
        (_, true) => Purpose::SignIn,
        (Some(c), false) => Purpose::Resume {
            claude_session_id: c.id.clone(),
        },
        (None, false) => Purpose::Work {
            claude_session_id: (agent == AgentKind::Claude).then(|| id.clone()),
        },
    };
    let mut spec = agent.launch(&purpose);
    if agent == AgentKind::Claude && !sign_in {
        // Its own workspace's notes and About Mike, so the index in CLAUDE.md can be followed.
        add_dirs(&mut spec.args, &state.knowledge.session_dirs(&ws.id));
    }
    // The sidebar shows a session under its project, so the title is just the agent.
    let title = match (&resumed, sign_in) {
        (_, true) => format!("Sign in · {}", agent.label()),
        (Some(c), false) => c.title.clone(),
        (None, false) => agent.label().to_owned(),
    };
    let info = SessionInfo {
        id,
        workspace_id: ws.id.to_string(),
        agent,
        title,
        sign_in,
        project_path: in_project.then(|| cwd.display().to_string()),
    };
    let detail = serde_json::json!({
        "agent": agent,
        "signIn": sign_in,
        "project": info.project_path,
        "resumed": resumed.is_some(),
    });
    spawn_session(
        &state,
        &ws,
        &layout,
        spec,
        &cwd,
        info,
        SessionKind::Work,
        &detail,
        cols,
        rows,
        on_event,
    )
    .await
}

fn add_dirs(args: &mut Vec<String>, dirs: &[PathBuf]) {
    for dir in dirs {
        args.push("--add-dir".into());
        args.push(dir.display().to_string());
    }
}

/// Start `spec` in a PTY inside the workspace's isolated environment, remember it, and
/// log it. Work sessions also get `OTTO_SESSION` and the hook arguments that report their
/// status (M5a); Shell sessions keep their scrollback.
#[allow(clippy::too_many_arguments)]
async fn spawn_session(
    state: &AppState,
    ws: &Workspace,
    layout: &WorkspaceLayout,
    mut spec: otto_core::agent::LaunchSpec,
    cwd: &std::path::Path,
    info: SessionInfo,
    kind: SessionKind,
    detail: &serde_json::Value,
    cols: u16,
    rows: u16,
    on_event: Channel<PtyEvent>,
) -> Result<SessionInfo, CommandError> {
    let mut env = IsolatedEnv::build(ws, layout, &state.host);
    if !info.sign_in {
        env.set("OTTO_SESSION", &info.id).map_err(internal)?;
        spec.args
            .extend(agent_events::launch_args(info.agent, layout));
    }
    let buffer = (info.agent == AgentKind::Shell && !info.sign_in).then(|| {
        let saved = scrollback::load(layout, &info.id).unwrap_or_default();
        if !saved.is_empty() {
            let _ = on_event.send(PtyEvent::Data {
                text: format!(
                    "{saved}\r\n\x1b[2m[Otto: restored from the last session]\x1b[0m\r\n"
                ),
            });
        }
        scrollback::Buffer::with(&saved)
    });
    let output = buffer.clone();
    let ended = SessionEnd {
        store: state.store.clone(),
        quitting: state.quitting.clone(),
        layout: layout.clone(),
        id: info.id.clone(),
        remembered: !info.sign_in,
    };
    let pty = PtySession::spawn(&env, &spec, cwd, cols.max(20), rows.max(5), move |event| {
        match &event {
            PtyEvent::Data { text } => {
                if let Some(b) = &output {
                    b.push(text);
                }
            }
            PtyEvent::Exit { .. } => ended.run(),
        }
        let _ = on_event.send(event);
    })
    .map_err(|e| CommandError::Invalid(e.to_string()))?;
    if !info.sign_in {
        let saved = SavedSession {
            id: info.id.clone(),
            workspace_id: ws.id.to_string(),
            agent: info.agent,
            project_path: info.project_path.clone(),
            title: info.title.clone(),
            kind,
            cli_session_id: None,
            status: SessionStatus::Idle,
            last_active_at: String::new(),
        };
        state.store.save_session(&saved).await.map_err(internal)?;
    }
    state.sessions.insert(info.clone(), pty, buffer);
    let _ = state
        .store
        .record_event(Some(&ws.id), "session_opened", detail)
        .await;
    Ok(info)
}

/// When a session's program exits on its own (or Mike closes it), it's no longer
/// remembered for the next launch. While Otto itself is quitting nothing changes, so
/// open sessions come back paused.
struct SessionEnd {
    store: otto_core::store::Store,
    quitting: std::sync::Arc<std::sync::atomic::AtomicBool>,
    layout: WorkspaceLayout,
    id: String,
    remembered: bool,
}

impl SessionEnd {
    fn run(&self) {
        if !self.remembered || self.quitting.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let (store, layout, id) = (self.store.clone(), self.layout.clone(), self.id.clone());
        tauri::async_runtime::spawn(async move {
            let _ = store.close_saved_session(&id).await;
            scrollback::forget(&layout, &id);
        });
    }
}

/// Rebuild the workspace's generated CLAUDE.md and AGENTS.md. A failure is logged, never
/// fatal: a session without fresh notes is better than no session.
fn refresh_instructions(state: &AppState, ws: &Workspace, layout: &WorkspaceLayout) {
    if let Err(err) = knowledge::write_instructions(&state.knowledge, ws, layout) {
        tracing::warn!(workspace = %ws.id, %err, "couldn't refresh instruction files");
    }
}

fn knowledge_error(e: KnowledgeError) -> CommandError {
    match e {
        KnowledgeError::Io(io) => internal(io),
        other => CommandError::Refused(other.to_string()),
    }
}

/// "Ask": Claude Code in the workspace's notes folder, able to open its projects and About
/// Mike, and `private/` only with `include_private` (SPEC §7.5).
#[tauri::command]
#[specta::specta]
pub async fn open_ask(
    state: State<'_, AppState>,
    workspace_id: String,
    include_private: bool,
    cols: u16,
    rows: u16,
    on_event: Channel<PtyEvent>,
) -> Result<SessionInfo, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let layout = state.paths.workspace(&ws.id);
    refresh_instructions(&state, &ws, &layout);
    let projects: Vec<PathBuf> = state
        .store
        .projects(&ws.id)
        .await
        .map_err(internal)?
        .into_iter()
        .map(|p| PathBuf::from(p.path))
        .filter(|p| p.is_dir())
        .collect();
    let cwd = state.knowledge.area_dir(&ws.id, Area::Notes);
    let id = uuid::Uuid::new_v4().to_string();
    let mut spec = AgentKind::Claude.launch(&Purpose::Work {
        claude_session_id: Some(id.clone()),
    });
    add_dirs(
        &mut spec.args,
        &state.knowledge.ask_dirs(&ws.id, &projects, include_private),
    );
    let info = SessionInfo {
        id,
        workspace_id: ws.id.to_string(),
        agent: AgentKind::Claude,
        title: if include_private {
            format!("Ask {} · with private", ws.name)
        } else {
            format!("Ask {}", ws.name)
        },
        sign_in: false,
        project_path: None,
    };
    let detail =
        serde_json::json!({ "agent": "claude", "ask": true, "includePrivate": include_private });
    let kind = if include_private {
        SessionKind::AskPrivate
    } else {
        SessionKind::Ask
    };
    spawn_session(
        &state, &ws, &layout, spec, &cwd, info, kind, &detail, cols, rows, on_event,
    )
    .await
}

/// Everything in the workspace's knowledge, plus About Mike.
#[tauri::command]
#[specta::specta]
pub async fn list_knowledge(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Vec<KnowledgeEntry>, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let k = state.knowledge.clone();
    tauri::async_runtime::spawn_blocking(move || knowledge::list(&k, &ws.id))
        .await
        .map_err(internal)
}

#[tauri::command]
#[specta::specta]
pub async fn read_note(
    state: State<'_, AppState>,
    workspace_id: String,
    area: Area,
    path: String,
) -> Result<String, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    knowledge::read_note(&state.knowledge, &ws.id, area, &path).map_err(knowledge_error)
}

/// Save a note, then rebuild the workspace's instruction files so new sessions see it.
#[tauri::command]
#[specta::specta]
pub async fn save_note(
    state: State<'_, AppState>,
    workspace_id: String,
    area: Area,
    path: String,
    content: String,
) -> Result<(), CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    knowledge::save_note(&state.knowledge, &ws.id, area, &path, &content)
        .map_err(knowledge_error)?;
    if area == Area::Global {
        // About Mike is shared, so every workspace's files change.
        for other in state.store.workspaces().await.map_err(internal)? {
            refresh_instructions(&state, &other, &state.paths.workspace(&other.id));
        }
    } else {
        refresh_instructions(&state, &ws, &state.paths.workspace(&ws.id));
    }
    Ok(())
}

/// Create a note from a title; returns its path.
#[tauri::command]
#[specta::specta]
pub async fn create_note(
    state: State<'_, AppState>,
    workspace_id: String,
    area: Area,
    title: String,
) -> Result<String, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let path =
        knowledge::create_note(&state.knowledge, &ws.id, area, &title).map_err(knowledge_error)?;
    refresh_instructions(&state, &ws, &state.paths.workspace(&ws.id));
    Ok(path)
}

/// An image from the workspace's knowledge, as a `data:` URL.
#[tauri::command]
#[specta::specta]
pub async fn read_knowledge_image(
    state: State<'_, AppState>,
    workspace_id: String,
    area: Area,
    path: String,
) -> Result<String, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let k = state.knowledge.clone();
    tauri::async_runtime::spawn_blocking(move || knowledge::image_data_url(&k, &ws.id, area, &path))
        .await
        .map_err(internal)?
        .map_err(knowledge_error)
}

/// Copy images Mike picked into the workspace's assets. Returns their asset paths.
#[tauri::command]
#[specta::specta]
pub async fn add_knowledge_assets(
    state: State<'_, AppState>,
    workspace_id: String,
    files: Vec<String>,
) -> Result<Vec<String>, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let k = state.knowledge.clone();
    tauri::async_runtime::spawn_blocking(move || {
        files
            .iter()
            .map(|f| knowledge::add_asset(&k, &ws.id, std::path::Path::new(f)))
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(internal)?
    .map_err(knowledge_error)
}

/// Keystrokes, paste and dictation all arrive here.
#[tauri::command]
#[specta::specta]
pub fn write_session(
    state: State<'_, AppState>,
    id: String,
    data: String,
) -> Result<(), CommandError> {
    state
        .sessions
        .with(&id, |pty| pty.write(&data))
        .ok_or(CommandError::NotFound(id))?
        .map_err(|e| CommandError::Invalid(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub fn resize_session(
    state: State<'_, AppState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), CommandError> {
    state
        .sessions
        .with(&id, |pty| pty.resize(cols.max(20), rows.max(5)))
        .ok_or(CommandError::NotFound(id))?
        .map_err(|e| CommandError::Invalid(e.to_string()))
}

/// Close a session for good: kill it, and don't bring it back on the next launch.
#[tauri::command]
#[specta::specta]
pub async fn close_session(state: State<'_, AppState>, id: String) -> Result<bool, CommandError> {
    let workspace = state
        .sessions
        .list()
        .into_iter()
        .find(|s| s.id == id)
        .map(|s| s.workspace_id);
    let closed = state.sessions.close(&id);
    state
        .store
        .close_saved_session(&id)
        .await
        .map_err(internal)?;
    if let Some(ws) = workspace.and_then(|w| otto_core::workspace::WorkspaceId::parse(&w).ok()) {
        scrollback::forget(&state.paths.workspace(&ws), &id);
    }
    Ok(closed)
}

/// Sessions that were open when Otto last quit and aren't running now: shown paused.
#[tauri::command]
#[specta::specta]
pub async fn list_paused_sessions(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Vec<SavedSession>, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    Ok(state
        .store
        .open_sessions(&ws.id)
        .await
        .map_err(internal)?
        .into_iter()
        .filter(|s| !state.sessions.contains(&s.id))
        .collect())
}

/// Stop remembering a paused session without resuming it.
#[tauri::command]
#[specta::specta]
pub async fn dismiss_paused_session(
    state: State<'_, AppState>,
    workspace_id: String,
    id: String,
) -> Result<(), CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    if state
        .store
        .session(&ws.id, &id)
        .await
        .map_err(internal)?
        .is_none()
    {
        return Err(CommandError::NotFound(id));
    }
    state
        .store
        .close_saved_session(&id)
        .await
        .map_err(internal)?;
    scrollback::forget(&state.paths.workspace(&ws.id), &id);
    Ok(())
}

/// The Claude launch for a paused session: `--resume` if it has a conversation, else a
/// fresh start with the same ID (it was opened but never used).
async fn claude_resume_spec(
    layout: &WorkspaceLayout,
    cwd: &std::path::Path,
    id: &str,
) -> Result<otto_core::agent::LaunchSpec, CommandError> {
    let (claude_dir, dir, sid) = (layout.claude_dir(), cwd.to_path_buf(), id.to_owned());
    let has_history = tauri::async_runtime::spawn_blocking(move || {
        history::list(&claude_dir, &dir, usize::MAX)
            .iter()
            .any(|c| c.id == sid)
    })
    .await
    .map_err(internal)?;
    let purpose = if has_history {
        Purpose::Resume {
            claude_session_id: id.to_owned(),
        }
    } else {
        Purpose::Work {
            claude_session_id: Some(id.to_owned()),
        }
    };
    Ok(AgentKind::Claude.launch(&purpose))
}

/// Pick a paused session back up the way it was opened: Claude with `--resume`, Codex with
/// `codex resume`, Shell with its saved scrollback. Projects are re-verified.
#[tauri::command]
#[specta::specta]
pub async fn resume_paused_session(
    state: State<'_, AppState>,
    workspace_id: String,
    id: String,
    cols: u16,
    rows: u16,
    on_event: Channel<PtyEvent>,
) -> Result<SessionInfo, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let layout = state.paths.workspace(&ws.id);
    let saved = state
        .store
        .session(&ws.id, &id)
        .await
        .map_err(internal)?
        .ok_or_else(|| CommandError::NotFound(id.clone()))?;
    if state.sessions.contains(&saved.id) {
        return Err(CommandError::Refused(
            "That session is already open.".into(),
        ));
    }
    refresh_instructions(&state, &ws, &layout);

    let ask = matches!(saved.kind, SessionKind::Ask | SessionKind::AskPrivate);
    let (cwd, in_project) = if ask {
        (state.knowledge.area_dir(&ws.id, Area::Notes), false)
    } else {
        verified_cwd(&state, &ws, saved.project_path.as_deref()).await?
    };
    let work = Purpose::Work {
        claude_session_id: None,
    };
    let mut spec = match saved.agent {
        AgentKind::Claude => claude_resume_spec(&layout, &cwd, &saved.id).await?,
        AgentKind::Codex => {
            let mut spec = AgentKind::Codex.launch(&work);
            if let Some(thread) = &saved.cli_session_id {
                spec.args = vec!["resume".into(), thread.clone()];
            }
            spec
        }
        AgentKind::Shell => AgentKind::Shell.launch(&work),
    };
    if saved.agent == AgentKind::Claude {
        let dirs = if ask {
            let projects: Vec<PathBuf> = state
                .store
                .projects(&ws.id)
                .await
                .map_err(internal)?
                .into_iter()
                .map(|p| PathBuf::from(p.path))
                .filter(|p| p.is_dir())
                .collect();
            state
                .knowledge
                .ask_dirs(&ws.id, &projects, saved.kind == SessionKind::AskPrivate)
        } else {
            state.knowledge.session_dirs(&ws.id)
        };
        add_dirs(&mut spec.args, &dirs);
    }
    let info = SessionInfo {
        id: saved.id.clone(),
        workspace_id: ws.id.to_string(),
        agent: saved.agent,
        title: saved.title.clone(),
        sign_in: false,
        project_path: in_project.then(|| cwd.display().to_string()),
    };
    let detail = serde_json::json!({
        "agent": saved.agent,
        "resumedPaused": true,
        "project": info.project_path,
    });
    spawn_session(
        &state, &ws, &layout, spec, &cwd, info, saved.kind, &detail, cols, rows, on_event,
    )
    .await
}

/// Sessions still running (for example after the window reloads).
#[tauri::command]
#[specta::specta]
pub fn list_sessions(state: State<'_, AppState>) -> Vec<SessionInfo> {
    state.sessions.list()
}

/// What can be copied from Mike's normal `~/.claude` setup into a workspace.
#[tauri::command]
#[specta::specta]
pub fn claude_setup_inventory(state: State<'_, AppState>) -> Result<Vec<SetupItem>, CommandError> {
    claude_setup::inventory(&ClaudeSource::for_home(&state.host.home))
        .map_err(|e| CommandError::Invalid(e.to_string()))
}

/// Copy the chosen setup items into one workspace's Claude config. Copies only.
#[tauri::command]
#[specta::specta]
pub async fn import_claude_setup(
    state: State<'_, AppState>,
    workspace_id: String,
    item_ids: Vec<String>,
) -> Result<Vec<String>, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let layout = state.paths.workspace(&ws.id);
    let source = ClaudeSource::for_home(&state.host.home);
    let ids = item_ids.clone();
    let done =
        tauri::async_runtime::spawn_blocking(move || claude_setup::import(&source, &layout, &ids))
            .await
            .map_err(internal)?
            .map_err(|e| CommandError::Invalid(e.to_string()))?;
    refresh_instructions(&state, &ws, &state.paths.workspace(&ws.id));
    let detail = serde_json::json!({ "items": done });
    let _ = state
        .store
        .record_event(Some(&ws.id), "claude_setup_imported", &detail)
        .await;
    Ok(done)
}
