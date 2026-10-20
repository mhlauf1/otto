//! Real terminals (PTYs) for agent sessions. Every PTY is spawned from an
//! [`IsolatedEnv`], the same way as every other child process (SPEC §4.1).

use std::io::{Read, Write};
use std::path::Path;

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use specta::Type;

use crate::agent::LaunchSpec;
use crate::env::IsolatedEnv;

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("Couldn't open a terminal: {0}")]
    Open(String),
    #[error("Couldn't start {program}: {reason}")]
    Spawn { program: String, reason: String },
    #[error("The session has ended.")]
    Closed,
}

/// What a session sends to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PtyEvent {
    /// Terminal output, always valid UTF-8 (characters split across reads are held back).
    Data { text: String },
    /// The program exited. `code` is `None` if it was killed by a signal.
    Exit { code: Option<u32> },
}

/// Decodes a byte stream to UTF-8 without breaking characters that span reads.
#[derive(Debug, Default)]
pub struct Utf8Stream {
    pending: Vec<u8>,
}

impl Utf8Stream {
    pub fn push(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        let mut out = String::new();
        loop {
            match std::str::from_utf8(&self.pending) {
                Ok(s) => {
                    out.push_str(s);
                    self.pending.clear();
                    return out;
                }
                Err(e) => {
                    let valid = e.valid_up_to();
                    out.push_str(&String::from_utf8_lossy(&self.pending[..valid]));
                    match e.error_len() {
                        // Incomplete character at the end: keep it for the next read.
                        None => {
                            self.pending.drain(..valid);
                            return out;
                        }
                        // Genuinely invalid bytes: replace them and carry on.
                        Some(len) => {
                            out.push(char::REPLACEMENT_CHARACTER);
                            self.pending.drain(..valid + len);
                        }
                    }
                }
            }
        }
    }
}

/// A running terminal session. Dropping it kills the program.
pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

impl std::fmt::Debug for PtySession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PtySession")
    }
}

impl PtySession {
    /// Start `spec` in a new PTY inside `env`, in `cwd`. Output and the exit event are
    /// delivered to `on_event` from a background thread.
    ///
    /// # Errors
    /// Fails if the PTY can't be opened or the program can't start (e.g. not installed).
    pub fn spawn(
        env: &IsolatedEnv,
        spec: &LaunchSpec,
        cwd: &Path,
        cols: u16,
        rows: u16,
        mut on_event: impl FnMut(PtyEvent) + Send + 'static,
    ) -> Result<Self, PtyError> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Open(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&spec.program);
        cmd.args(&spec.args);
        cmd.cwd(cwd);
        cmd.env_clear();
        for (k, v) in env.vars() {
            cmd.env(k, v);
        }
        let spawn_err = |e: &dyn std::fmt::Display| PtyError::Spawn {
            program: spec.program.clone(),
            reason: e.to_string(),
        };
        let mut child = pair.slave.spawn_command(cmd).map_err(|e| spawn_err(&e))?;
        drop(pair.slave);

        let killer = child.clone_killer();
        let mut reader = pair.master.try_clone_reader().map_err(|e| spawn_err(&e))?;
        let writer = pair.master.take_writer().map_err(|e| spawn_err(&e))?;

        std::thread::Builder::new()
            .name(format!("pty-{}", spec.program))
            .spawn(move || {
                let mut decoder = Utf8Stream::default();
                let mut buf = [0u8; 16 * 1024];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            let text = decoder.push(&buf[..n]);
                            if !text.is_empty() {
                                on_event(PtyEvent::Data { text });
                            }
                        }
                    }
                }
                let code = child.wait().ok().map(|s| s.exit_code());
                on_event(PtyEvent::Exit { code });
            })
            .map_err(|e| spawn_err(&e))?;

        Ok(Self {
            master: pair.master,
            writer,
            killer,
        })
    }

    /// Send keystrokes (or pasted / dictated text) to the program.
    ///
    /// # Errors
    /// Fails if the program has exited.
    pub fn write(&mut self, data: &str) -> Result<(), PtyError> {
        self.writer
            .write_all(data.as_bytes())
            .map_err(|_| PtyError::Closed)?;
        self.writer.flush().map_err(|_| PtyError::Closed)
    }

    /// # Errors
    /// Fails if the PTY is gone.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), PtyError> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|_| PtyError::Closed)
    }

    pub fn kill(&mut self) {
        let _ = self.killer.kill();
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        self.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_split_across_reads_is_reassembled() {
        let mut d = Utf8Stream::default();
        let bytes = "→ ok ✓".as_bytes();
        let (a, b) = bytes.split_at(2); // splits the 3-byte arrow
        let first = d.push(a);
        let second = d.push(b);
        assert_eq!(format!("{first}{second}"), "→ ok ✓");
        assert_eq!(first, "");
    }

    #[test]
    fn invalid_bytes_become_replacement_characters() {
        let mut d = Utf8Stream::default();
        assert_eq!(d.push(b"a\xffb"), "a\u{FFFD}b");
    }
}
