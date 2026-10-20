//! IPC commands. Keep these thin: translate the call, delegate to `otto-core`.

use std::path::PathBuf;

use otto_core::doctor::{self, DoctorReport};
use otto_core::env::IsolatedEnv;
use otto_core::provision::write_doctor_status;
use otto_core::repo::{RepoRefusal, VerifiedRepo, verify_repo};
use otto_core::workspace::Workspace;
use otto_core::{AppInfo, AppInfoError};
use serde::Serialize;
use specta::Type;
use tauri::State;

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
