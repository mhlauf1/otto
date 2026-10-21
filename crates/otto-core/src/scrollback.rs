//! A Shell session's recent output, kept so it comes back after a relaunch (M5a).
//! Agents redraw their own history when resumed, so only Shell sessions keep one.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use crate::workspace::WorkspaceLayout;

/// The most kept per session; older output falls off the top.
pub const MAX_BYTES: usize = 256 * 1024;

/// Output collected while a session runs. Cheap to clone; clones share one buffer.
#[derive(Debug, Clone, Default)]
pub struct Buffer(Arc<Mutex<Inner>>);

#[derive(Debug, Default)]
struct Inner {
    text: String,
    dirty: bool,
}

impl Buffer {
    /// Seed a buffer with what was saved before, so saving again keeps it.
    #[must_use]
    pub fn with(text: &str) -> Self {
        let buffer = Self::default();
        buffer.push(text);
        buffer.lock().dirty = false;
        buffer
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn push(&self, text: &str) {
        let mut inner = self.lock();
        inner.text.push_str(text);
        if inner.text.len() > MAX_BYTES {
            let mut cut = inner.text.len() - MAX_BYTES;
            while !inner.text.is_char_boundary(cut) {
                cut += 1;
            }
            inner.text.drain(..cut);
        }
        inner.dirty = true;
    }

    /// The text, if it changed since the last call.
    #[must_use]
    pub fn take_if_changed(&self) -> Option<String> {
        let mut inner = self.lock();
        if !inner.dirty {
            return None;
        }
        inner.dirty = false;
        Some(inner.text.clone())
    }
}

/// Where a session's scrollback is saved. `None` unless `id` is a UUID.
#[must_use]
pub fn path(layout: &WorkspaceLayout, id: &str) -> Option<PathBuf> {
    let id = uuid::Uuid::parse_str(id).ok()?;
    Some(layout.root().join("scrollback").join(format!("{id}.log")))
}

/// # Errors
/// Fails on I/O errors.
pub fn save(layout: &WorkspaceLayout, id: &str, text: &str) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let Some(file) = path(layout, id) else {
        return Ok(());
    };
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    let tmp = file.with_extension("log.tmp");
    fs::write(&tmp, text)?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
    fs::rename(tmp, file)
}

#[must_use]
pub fn load(layout: &WorkspaceLayout, id: &str) -> Option<String> {
    fs::read_to_string(path(layout, id)?).ok()
}

pub fn forget(layout: &WorkspaceLayout, id: &str) {
    if let Some(file) = path(layout, id) {
        let _ = fs::remove_file(file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{OttoPaths, WorkspaceId};

    #[test]
    fn keeps_only_the_tail_on_a_character_boundary() {
        let buffer = Buffer::default();
        buffer.push(&"é".repeat(MAX_BYTES));
        buffer.push("end");
        let text = buffer.take_if_changed().unwrap();
        assert!(text.len() <= MAX_BYTES);
        assert!(text.ends_with("end"));
        assert_eq!(buffer.take_if_changed(), None, "unchanged since last take");
    }

    #[test]
    fn saves_loads_and_forgets_by_uuid_only() {
        let dir = tempfile::tempdir().unwrap();
        let layout = OttoPaths::new(dir.path()).workspace(&WorkspaceId::parse("lauf").unwrap());
        let id = "7de656b9-96e1-4226-9fbe-c8d129813f94";
        save(&layout, id, "$ ls\nREADME.md\n").unwrap();
        assert_eq!(load(&layout, id).as_deref(), Some("$ ls\nREADME.md\n"));
        forget(&layout, id);
        assert_eq!(load(&layout, id), None);
        assert!(path(&layout, "../../etc/passwd").is_none());
        save(&layout, "../escape", "x").unwrap();
        assert!(!dir.path().join("escape.log").exists());
    }
}
