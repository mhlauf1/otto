//! Background work while Otto runs (M5a): pick up what agents report through their
//! workspace's inbox, tell the window, and save Shell scrollback now and then.

use std::time::Duration;

use otto_core::agent::AgentKind;
use otto_core::agent_events;
use otto_core::scrollback;
use otto_core::store::SessionStatus;
use otto_core::workspace::WorkspaceId;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::state::AppState;

const TICK: Duration = Duration::from_millis(400);
/// Scrollback is saved every this many ticks (10 s), and always when Otto quits.
const SAVE_EVERY: u64 = 25;
/// An agent's screen needs a moment after launch before it takes typed input.
const WARM_UP: Duration = Duration::from_secs(3);
/// Between pasting a prompt and pressing Return, so the paste lands as one block.
const PASTE_SETTLE: Duration = Duration::from_millis(60);

/// An agent started, finished or needs Mike.
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusChanged {
    pub workspace_id: String,
    pub id: String,
    pub status: SessionStatus,
    /// What the agent said, e.g. "Claude needs your permission to use Bash".
    pub message: Option<String>,
}

/// A session's "Up next" list changed (added, removed or sent).
#[derive(Debug, Clone, Serialize, Deserialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct QueueChanged {
    pub workspace_id: String,
    pub session_id: String,
}

pub fn queue_changed(app: &AppHandle, workspace: &WorkspaceId, session_id: &str) {
    let _ = QueueChanged {
        workspace_id: workspace.to_string(),
        session_id: session_id.to_owned(),
    }
    .emit(app);
}

/// If the session's agent is waiting for its next prompt, send the first one queued (M5b).
/// Never for a Shell, never while the agent works or needs Mike, and never twice.
pub async fn send_next_if_idle(
    app: &AppHandle,
    state: &AppState,
    workspace: &WorkspaceId,
    session_id: &str,
) {
    let Some(info) = state.sessions.info(session_id) else {
        return;
    };
    if info.agent == AgentKind::Shell || info.workspace_id != workspace.as_str() {
        return;
    }
    if let Some(age) = state.sessions.age(session_id) {
        tokio::time::sleep(WARM_UP.saturating_sub(age)).await;
    }
    if !state
        .store
        .claim_idle(workspace, session_id)
        .await
        .unwrap_or(false)
    {
        return;
    }
    let Ok(Some(item)) = state.store.take_next(workspace, session_id).await else {
        // Nothing waiting after all: hand the session back.
        let _ = state
            .store
            .set_session_status(workspace, session_id, SessionStatus::Idle)
            .await;
        return;
    };
    // Bracketed paste, so a multi-line prompt arrives whole. Escape characters are dropped
    // so a prompt can't end the paste early or send control sequences.
    let text = item.prompt.replace('\x1b', "");
    let pasted = format!("\x1b[200~{text}\x1b[201~");
    let _ = state.sessions.with(session_id, |pty| pty.write(&pasted));
    tokio::time::sleep(PASTE_SETTLE).await;
    let _ = state.sessions.with(session_id, |pty| pty.write("\r"));
    let _ = SessionStatusChanged {
        workspace_id: workspace.to_string(),
        id: session_id.to_owned(),
        status: SessionStatus::Running,
        message: None,
    }
    .emit(app);
    queue_changed(app, workspace, session_id);
}

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut ticks: u64 = 0;
        loop {
            tokio::time::sleep(TICK).await;
            ticks = ticks.wrapping_add(1);
            let state = app.state::<AppState>();
            deliver_events(&app, &state).await;
            if ticks.is_multiple_of(SAVE_EVERY) {
                save_scrollback(&state);
            }
        }
    });
}

async fn deliver_events(app: &AppHandle, state: &AppState) {
    let Ok(workspaces) = state.store.workspaces().await else {
        return;
    };
    for ws in workspaces {
        let layout = state.paths.workspace(&ws.id);
        let events = tauri::async_runtime::spawn_blocking(move || agent_events::drain(&layout))
            .await
            .unwrap_or_default();
        for event in events {
            // Only a session of *this* workspace can be updated from its inbox.
            let applied = state
                .store
                .set_session_status(&ws.id, &event.session_id, event.status)
                .await
                .unwrap_or(false);
            if !applied {
                tracing::debug!(workspace = %ws.id, "ignored an event for an unknown session");
                continue;
            }
            if let Some(thread) = &event.cli_session_id {
                let _ = state
                    .store
                    .set_cli_session_id(&ws.id, &event.session_id, thread)
                    .await;
            }
            let finished = event.status == SessionStatus::Idle;
            let id = event.session_id.clone();
            let _ = SessionStatusChanged {
                workspace_id: ws.id.to_string(),
                id: event.session_id,
                status: event.status,
                message: event.message,
            }
            .emit(app);
            if finished {
                send_next_if_idle(app, state, &ws.id, &id).await;
            }
        }
    }
}

/// Write Shell scrollback that changed since it was last saved.
pub fn save_scrollback(state: &AppState) {
    for (workspace, id, text) in state.sessions.changed_scrollback() {
        let Ok(ws) = WorkspaceId::parse(&workspace) else {
            continue;
        };
        if let Err(err) = scrollback::save(&state.paths.workspace(&ws), &id, &text) {
            tracing::warn!(%err, "couldn't save scrollback");
        }
    }
}
