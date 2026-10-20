//! IPC commands. Keep these thin: translate the call, delegate to `otto-core`.

// Tauri deserializes every argument into an owned value and hands out `State` by value.
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;

use otto_core::agent::{AgentKind, Purpose};
use otto_core::claude_setup::{self, ClaudeSource, SetupItem};
use otto_core::doctor::{self, DoctorReport};
use otto_core::env::IsolatedEnv;
use otto_core::projects::{DiscoveredRepo, classify, find_repos};
use otto_core::provision::write_doctor_status;
use otto_core::pty::{PtyEvent, PtySession};
use otto_core::repo::{RemotePolicy, RepoRefusal, VerifiedRepo, verify_repo};
use otto_core::store::Project;
use otto_core::workspace::Workspace;
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
    let _ = state
        .store
        .record_event(Some(&ws.id), "identity_doctor", &detail)
        .await;
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

/// Open an agent (or shell) in a real terminal inside the workspace's isolated
/// environment, optionally inside one of its projects. Output streams to `on_event`.
/// With `sign_in`, runs the CLI's own login flow.
#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn open_session(
    state: State<'_, AppState>,
    workspace_id: String,
    agent: AgentKind,
    sign_in: bool,
    project_path: Option<String>,
    cols: u16,
    rows: u16,
    on_event: Channel<PtyEvent>,
) -> Result<SessionInfo, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let layout = state.paths.workspace(&ws.id);

    // A project must be registered to *this* workspace, and is re-verified every time:
    // a token or foreign remote added since registration is caught here (ADR-007).
    let (cwd, project) = match project_path {
        Some(path) => {
            let owner = state.store.repo_owner(&path).await.map_err(internal)?;
            if owner.as_deref() != Some(ws.id.as_str()) {
                return Err(CommandError::Refused(format!(
                    "{path} isn't a {} project. Import it into {} first.",
                    ws.name, ws.name
                )));
            }
            let repo = verify(
                &state,
                &ws,
                PathBuf::from(&path),
                RemotePolicy::AllowLocalOnly,
            )
            .await?;
            let name = PathBuf::from(&repo.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            (PathBuf::from(repo.path), name)
        }
        None => (state.host.home.clone(), None),
    };

    let env = IsolatedEnv::build(&ws, &layout, &state.host);
    let id = uuid::Uuid::new_v4().to_string();
    let purpose = if sign_in {
        Purpose::SignIn
    } else {
        Purpose::Work {
            claude_session_id: (agent == AgentKind::Claude).then(|| id.clone()),
        }
    };
    let spec = agent.launch(&purpose);
    let pty = PtySession::spawn(&env, &spec, &cwd, cols.max(20), rows.max(5), move |event| {
        let _ = on_event.send(event);
    })
    .map_err(|e| CommandError::Invalid(e.to_string()))?;
    let title = match (sign_in, &project) {
        (true, _) => format!("Sign in · {}", agent.label()),
        (false, Some(name)) => format!("{} · {name}", agent.label()),
        (false, None) => agent.label().to_owned(),
    };
    let info = SessionInfo {
        id,
        workspace_id: ws.id.to_string(),
        agent,
        title,
        sign_in,
        project_path: project.is_some().then(|| cwd.display().to_string()),
    };
    state.sessions.insert(info.clone(), pty);
    let detail =
        serde_json::json!({ "agent": agent, "signIn": sign_in, "project": info.project_path });
    let _ = state
        .store
        .record_event(Some(&ws.id), "session_opened", &detail)
        .await;
    Ok(info)
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

#[tauri::command]
#[specta::specta]
pub fn close_session(state: State<'_, AppState>, id: String) -> bool {
    state.sessions.close(&id)
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
    let detail = serde_json::json!({ "items": done });
    let _ = state
        .store
        .record_event(Some(&ws.id), "claude_setup_imported", &detail)
        .await;
    Ok(done)
}
