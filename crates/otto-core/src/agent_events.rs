//! What each agent is doing, reported by the agents themselves (M5a).
//!
//! Claude Code and Codex both run a command when something happens: Claude through hooks
//! (`UserPromptSubmit`, `Stop`, `Notification`), Codex through `notify` after each turn.
//! Otto points both at one small script per workspace. The script drops the event into
//! that workspace's own inbox folder, named by the session's `OTTO_SESSION`. Otto reads
//! the inbox and updates the session, but only if it belongs to that workspace.
//!
//! Nothing here edits Mike's settings: Claude gets the hooks through `--settings`, and
//! Codex through `-c notify=…`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::agent::AgentKind;
use crate::env::shell_quote;
use crate::store::SessionStatus;
use crate::workspace::{Workspace, WorkspaceLayout};

/// Payloads bigger than this are ignored (a normal one is a few hundred bytes).
const MAX_EVENT_BYTES: u64 = 64 * 1024;
const MAX_MESSAGE: usize = 200;

/// Where this workspace's agents drop events.
#[must_use]
pub fn inbox_dir(layout: &WorkspaceLayout) -> PathBuf {
    layout.root().join("inbox")
}

#[must_use]
pub fn script_path(layout: &WorkspaceLayout) -> PathBuf {
    layout.hooks_dir().join("otto-agent-event")
}

#[must_use]
pub fn claude_settings_path(layout: &WorkspaceLayout) -> PathBuf {
    layout.hooks_dir().join("claude-settings.json")
}

/// The event script. It never fails or blocks the agent: anything unexpected just exits.
#[must_use]
pub fn script(workspace: &Workspace, layout: &WorkspaceLayout) -> String {
    format!(
        r#"#!/bin/sh
# Managed by Otto. Regenerated on every launch.
# Workspace: {name} ({id}). Agents run this when they start, finish or need Mike.
# It only ever writes into this workspace's inbox.
umask 077
case "${{OTTO_SESSION:-}}" in
  ""|*[!0-9a-f-]*) exit 0 ;;
esac
inbox={inbox}
[ -d "$inbox" ] || exit 0
source="${{1:-}}"
case "$source" in claude|codex) ;; *) exit 0 ;; esac
tmp=$(mktemp "$inbox/.$OTTO_SESSION.XXXXXX") || exit 0
if [ "$source" = codex ]; then
  printf '%s' "${{2:-}}" > "$tmp"
else
  head -c {max} > "$tmp"
fi
mv "$tmp" "$inbox/$OTTO_SESSION.$source.${{tmp##*.}}.json" 2>/dev/null || rm -f "$tmp"
exit 0
"#,
        name = workspace.name,
        id = workspace.id,
        inbox = shell_quote(&inbox_dir(layout).display().to_string()),
        max = MAX_EVENT_BYTES,
    )
}

/// Claude settings layered on with `--settings`: the three hooks Otto listens to.
#[must_use]
pub fn claude_settings(layout: &WorkspaceLayout) -> String {
    let command = format!(
        "{} claude",
        shell_quote(&script_path(layout).display().to_string())
    );
    let hook = serde_json::json!([{ "hooks": [{ "type": "command", "command": command }] }]);
    serde_json::json!({
        "hooks": {
            "UserPromptSubmit": hook,
            "Stop": hook,
            "Notification": hook,
        }
    })
    .to_string()
}

/// Extra arguments that make an agent report its events to Otto.
#[must_use]
pub fn launch_args(agent: AgentKind, layout: &WorkspaceLayout) -> Vec<String> {
    match agent {
        AgentKind::Claude => vec![
            "--settings".into(),
            claude_settings_path(layout).display().to_string(),
        ],
        AgentKind::Codex => vec!["-c".into(), codex_notify(layout)],
        AgentKind::Shell => Vec::new(),
    }
}

/// `notify=["<script>","codex"]`, as a TOML value for `codex -c`.
fn codex_notify(layout: &WorkspaceLayout) -> String {
    let toml_string = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    format!(
        "notify=[{},{}]",
        toml_string(&script_path(layout).display().to_string()),
        toml_string("codex")
    )
}

/// Write the script and Claude's settings file, and create the inbox.
///
/// # Errors
/// Fails on I/O errors.
pub fn provision(workspace: &Workspace, layout: &WorkspaceLayout) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let inbox = inbox_dir(layout);
    fs::create_dir_all(&inbox)?;
    fs::set_permissions(&inbox, fs::Permissions::from_mode(0o700))?;
    let script_file = script_path(layout);
    fs::write(&script_file, script(workspace, layout))?;
    fs::set_permissions(&script_file, fs::Permissions::from_mode(0o700))?;
    let settings = claude_settings_path(layout);
    fs::write(&settings, claude_settings(layout))?;
    fs::set_permissions(&settings, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

/// One thing an agent reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEvent {
    /// Otto's session ID, from the file name (a UUID, checked).
    pub session_id: String,
    pub status: SessionStatus,
    /// What to show Mike, e.g. "Claude needs your permission to use Bash".
    pub message: Option<String>,
    /// Codex's thread ID, so the session can be resumed later.
    pub cli_session_id: Option<String>,
}

/// Take every event waiting in the inbox, oldest first. Files are removed as they're
/// read; anything malformed is dropped.
#[must_use]
pub fn drain(layout: &WorkspaceLayout) -> Vec<AgentEvent> {
    let Ok(read) = fs::read_dir(inbox_dir(layout)) else {
        return Vec::new();
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = read
        .filter_map(Result::ok)
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            !name.starts_with('.') && name.ends_with(".json")
        })
        .filter_map(|e| {
            let modified = e.metadata().ok()?.modified().ok()?;
            Some((modified, e.path()))
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .filter_map(|(_, path)| {
            let event = parse_file(&path);
            let _ = fs::remove_file(&path);
            event
        })
        .collect()
}

fn parse_file(path: &Path) -> Option<AgentEvent> {
    let name = path.file_name()?.to_str()?;
    let mut parts = name.split('.');
    let session_id = uuid::Uuid::parse_str(parts.next()?).ok()?.to_string();
    let source = parts.next()?;
    if fs::metadata(path).ok()?.len() > MAX_EVENT_BYTES {
        return None;
    }
    let payload: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    let text = |key: &str| {
        payload
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(|s| shorten(s, MAX_MESSAGE))
    };
    let (status, message, cli_session_id) = match source {
        "claude" => match payload.get("hook_event_name")?.as_str()? {
            "UserPromptSubmit" => (SessionStatus::Running, None, None),
            "Stop" => (SessionStatus::Idle, None, None),
            "Notification" => (SessionStatus::Waiting, text("message"), None),
            _ => return None,
        },
        "codex" => match payload.get("type")?.as_str()? {
            "agent-turn-complete" => (
                SessionStatus::Idle,
                text("last-assistant-message"),
                payload
                    .get("thread-id")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|t| uuid::Uuid::parse_str(t).ok())
                    .map(|t| t.to_string()),
            ),
            _ => return None,
        },
        _ => return None,
    };
    Some(AgentEvent {
        session_id,
        status,
        message,
        cli_session_id,
    })
}

fn shorten(text: &str, max: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= max {
        return one_line;
    }
    let cut: String = one_line.chars().take(max - 1).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::default_workspaces;
    use crate::workspace::OttoPaths;
    use std::process::{Command, Stdio};

    const SESSION: &str = "7de656b9-96e1-4226-9fbe-c8d129813f94";

    fn setup() -> (tempfile::TempDir, Workspace, WorkspaceLayout) {
        let dir = tempfile::tempdir().unwrap();
        let ws = default_workspaces().unwrap().remove(1);
        let layout = OttoPaths::new(dir.path().join("Otto Support")).workspace(&ws.id);
        fs::create_dir_all(layout.hooks_dir()).unwrap();
        provision(&ws, &layout).unwrap();
        (dir, ws, layout)
    }

    /// Run the real script the way an agent would.
    fn run_script(layout: &WorkspaceLayout, session: &str, args: &[&str], stdin: &str) {
        let mut child = Command::new(script_path(layout))
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("OTTO_SESSION", session)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        std::io::Write::write_all(child.stdin.as_mut().unwrap(), stdin.as_bytes()).unwrap();
        assert!(
            child.wait().unwrap().success(),
            "the script never fails the agent"
        );
    }

    #[test]
    fn claude_hooks_become_statuses() {
        let (_dir, _ws, layout) = setup();
        for (event, extra) in [
            ("UserPromptSubmit", ""),
            (
                "Notification",
                r#","message":"Claude needs your permission to use Bash""#,
            ),
            ("Stop", ""),
        ] {
            run_script(
                &layout,
                SESSION,
                &["claude"],
                &format!(r#"{{"hook_event_name":"{event}","session_id":"x"{extra}}}"#),
            );
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        let events = drain(&layout);
        let statuses: Vec<_> = events.iter().map(|e| e.status).collect();
        assert_eq!(
            statuses,
            [
                SessionStatus::Running,
                SessionStatus::Waiting,
                SessionStatus::Idle
            ]
        );
        assert_eq!(events[0].session_id, SESSION);
        assert_eq!(
            events[1].message.as_deref(),
            Some("Claude needs your permission to use Bash")
        );
        assert_eq!(drain(&layout), [], "events are taken once");
    }

    #[test]
    fn codex_turns_report_idle_and_their_thread() {
        let (_dir, _ws, layout) = setup();
        let thread = "2fabc426-502e-41bd-99a6-3d6942ba4f8f";
        run_script(
            &layout,
            SESSION,
            &[
                "codex",
                &format!(
                    r#"{{"type":"agent-turn-complete","thread-id":"{thread}","last-assistant-message":"Done: PR #8 merged."}}"#
                ),
            ],
            "",
        );
        let events = drain(&layout);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, SessionStatus::Idle);
        assert_eq!(events[0].cli_session_id.as_deref(), Some(thread));
        assert_eq!(events[0].message.as_deref(), Some("Done: PR #8 merged."));
    }

    #[test]
    fn the_script_ignores_bad_session_ids_and_unknown_sources() {
        let (_dir, _ws, layout) = setup();
        for session in ["", "../../evil", "not a uuid; rm -rf"] {
            run_script(
                &layout,
                session,
                &["claude"],
                r#"{"hook_event_name":"Stop"}"#,
            );
        }
        run_script(
            &layout,
            SESSION,
            &["bogus"],
            r#"{"hook_event_name":"Stop"}"#,
        );
        assert_eq!(fs::read_dir(inbox_dir(&layout)).unwrap().count(), 0);
    }

    #[test]
    fn malformed_files_are_dropped() {
        let (_dir, _ws, layout) = setup();
        let inbox = inbox_dir(&layout);
        fs::write(inbox.join(format!("{SESSION}.claude.a.json")), "not json").unwrap();
        fs::write(
            inbox.join("nope.claude.b.json"),
            r#"{"hook_event_name":"Stop"}"#,
        )
        .unwrap();
        fs::write(
            inbox.join(format!("{SESSION}.claude.c.json")),
            r#"{"hook_event_name":"PreToolUse"}"#,
        )
        .unwrap();
        assert_eq!(drain(&layout), []);
        assert_eq!(fs::read_dir(&inbox).unwrap().count(), 0);
    }

    #[test]
    fn launch_args_point_each_cli_at_the_script_without_touching_settings() {
        let (_dir, _ws, layout) = setup();
        let claude = launch_args(AgentKind::Claude, &layout);
        assert_eq!(claude[0], "--settings");
        let settings: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&claude[1]).unwrap()).unwrap();
        let command = settings["hooks"]["Stop"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        assert!(command.ends_with(" claude"));
        assert!(command.contains("otto-agent-event"));
        let codex = launch_args(AgentKind::Codex, &layout);
        assert_eq!(codex[0], "-c");
        assert!(codex[1].starts_with("notify=[\"") && codex[1].ends_with(",\"codex\"]"));
        assert_eq!(launch_args(AgentKind::Shell, &layout), Vec::<String>::new());
    }
}
