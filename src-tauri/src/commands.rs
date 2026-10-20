//! IPC commands. Keep these thin: translate the call, delegate to `otto-core`.

// Tauri deserializes every argument into an owned value and hands out `State` by value.
#![allow(clippy::needless_pass_by_value)]

use std::path::PathBuf;

use otto_core::agent::{AgentKind, Purpose};
use otto_core::doctor::{self, DoctorReport};
use otto_core::env::IsolatedEnv;
use otto_core::provision::write_doctor_status;
use otto_core::pty::{PtyEvent, PtySession};
use otto_core::repo::{RepoRefusal, VerifiedRepo, verify_repo};
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

/// Check and register a repo for a workspace (SPEC §4.2).
#[tauri::command]
#[specta::specta]
pub async fn register_repo(
    state: State<'_, AppState>,
    workspace_id: String,
    path: String,
) -> Result<VerifiedRepo, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let layout = state.paths.workspace(&ws.id);
    let env = IsolatedEnv::build(&ws, &layout, &state.host);
    let (ws2, layout2, path2) = (ws.clone(), layout.clone(), PathBuf::from(path));
    let verdict =
        tauri::async_runtime::spawn_blocking(move || verify_repo(&ws2, &layout2, &env, &path2))
            .await
            .map_err(|e| CommandError::Internal(e.to_string()))?;
    let repo = match verdict {
        Ok(repo) => repo,
        Err(refusal) => {
            let detail = serde_json::json!({ "reason": refusal.to_string() });
            let _ = state
                .store
                .record_event(Some(&ws.id), "repo_refused", &detail)
                .await;
            return Err(CommandError::Refused(refusal.to_string()));
        }
    };
    if let Some(owner) = state
        .store
        .repo_owner(&repo.path)
        .await
        .map_err(|e| CommandError::Internal(e.to_string()))?
    {
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
        .map_err(|e| CommandError::Internal(e.to_string()))?;
    Ok(repo)
}

/// Open an agent (or shell) in a real terminal inside the workspace's isolated
/// environment. Output streams to `on_event`. With `sign_in`, runs the CLI's own login flow.
#[tauri::command]
#[specta::specta]
pub async fn open_session(
    state: State<'_, AppState>,
    workspace_id: String,
    agent: AgentKind,
    sign_in: bool,
    cols: u16,
    rows: u16,
    on_event: Channel<PtyEvent>,
) -> Result<SessionInfo, CommandError> {
    let ws = state.workspace(&workspace_id).await?;
    let layout = state.paths.workspace(&ws.id);
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
    let pty = PtySession::spawn(
        &env,
        &spec,
        &state.host.home,
        cols.max(20),
        rows.max(5),
        move |event| {
            let _ = on_event.send(event);
        },
    )
    .map_err(|e| CommandError::Invalid(e.to_string()))?;
    let title = if sign_in {
        format!("Sign in · {}", agent.label())
    } else {
        agent.label().to_owned()
    };
    let info = SessionInfo {
        id,
        workspace_id: ws.id.to_string(),
        agent,
        title,
        sign_in,
    };
    state.sessions.insert(info.clone(), pty);
    let detail = serde_json::json!({ "agent": agent, "signIn": sign_in });
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
