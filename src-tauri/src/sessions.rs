//! Live terminal sessions, keyed by ID. The database remembers them across launches
//! (`otto_core::store`, M5a); this is what's running right now.

use std::collections::HashMap;
use std::sync::Mutex;

use otto_core::agent::AgentKind;
use otto_core::pty::PtySession;
use otto_core::scrollback;
use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub id: String,
    pub workspace_id: String,
    pub agent: AgentKind,
    pub title: String,
    pub sign_in: bool,
    /// The project the session runs in; `None` means General (the home folder).
    pub project_path: Option<String>,
}

struct Live {
    info: SessionInfo,
    pty: PtySession,
    /// Shell sessions only: their recent output, saved for the next launch.
    scrollback: Option<scrollback::Buffer>,
}

#[derive(Default)]
pub struct Sessions {
    live: Mutex<HashMap<String, Live>>,
}

impl Sessions {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Live>> {
        // A panic while holding the lock can't leave the map half-updated, so recover.
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn insert(
        &self,
        info: SessionInfo,
        pty: PtySession,
        scrollback: Option<scrollback::Buffer>,
    ) {
        self.lock().insert(
            info.id.clone(),
            Live {
                info,
                pty,
                scrollback,
            },
        );
    }

    /// Scrollback that changed since it was last saved: (workspace, session, text).
    pub fn changed_scrollback(&self) -> Vec<(String, String, String)> {
        self.lock()
            .values()
            .filter_map(|l| {
                let text = l.scrollback.as_ref()?.take_if_changed()?;
                Some((l.info.workspace_id.clone(), l.info.id.clone(), text))
            })
            .collect()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.lock().contains_key(id)
    }

    pub fn with<T>(&self, id: &str, f: impl FnOnce(&mut PtySession) -> T) -> Option<T> {
        self.lock().get_mut(id).map(|l| f(&mut l.pty))
    }

    /// Remove and kill a session. Dropping the PTY kills the program.
    pub fn close(&self, id: &str) -> bool {
        self.lock().remove(id).is_some()
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let mut v: Vec<_> = self.lock().values().map(|l| l.info.clone()).collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }
}
