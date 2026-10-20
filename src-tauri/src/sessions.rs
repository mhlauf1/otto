//! Live terminal sessions, keyed by ID. Sessions are in memory for now; M5 persists
//! and resumes them.

use std::collections::HashMap;
use std::sync::Mutex;

use otto_core::agent::AgentKind;
use otto_core::pty::PtySession;
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
}

struct Live {
    info: SessionInfo,
    pty: PtySession,
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

    pub fn insert(&self, info: SessionInfo, pty: PtySession) {
        self.lock().insert(info.id.clone(), Live { info, pty });
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
