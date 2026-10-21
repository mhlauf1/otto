//! Background work while Otto runs (M5a): pick up what agents report through their
//! workspace's inbox, tell the window, and save Shell scrollback now and then.

use std::time::Duration;

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
            let _ = SessionStatusChanged {
                workspace_id: ws.id.to_string(),
                id: event.session_id,
                status: event.status,
                message: event.message,
            }
            .emit(app);
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
