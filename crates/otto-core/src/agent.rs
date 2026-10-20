//! Which program a session runs, and with what arguments (SPEC §3.3).

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AgentKind {
    Claude,
    Codex,
    /// A plain zsh inside the workspace environment, for git and gh by hand.
    Shell,
}

/// Why the session is being opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    /// A working session. For Claude, Otto picks the session ID up front so it can resume later (M5).
    Work { claude_session_id: Option<String> },
    /// Run the CLI's own sign-in flow inside this workspace.
    SignIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl AgentKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
            Self::Shell => "Shell",
        }
    }

    pub fn launch(self, purpose: &Purpose) -> LaunchSpec {
        let (program, args): (&str, Vec<String>) = match (self, purpose) {
            (Self::Claude, Purpose::SignIn) => ("claude", vec!["auth".into(), "login".into()]),
            (
                Self::Claude,
                Purpose::Work {
                    claude_session_id: Some(id),
                },
            ) => ("claude", vec!["--session-id".into(), id.clone()]),
            (
                Self::Claude,
                Purpose::Work {
                    claude_session_id: None,
                },
            ) => ("claude", vec![]),
            (Self::Codex, Purpose::SignIn) => ("codex", vec!["login".into()]),
            (Self::Codex, Purpose::Work { .. }) => ("codex", vec![]),
            // Interactive login shell: reads $ZDOTDIR, which is inside the workspace.
            (Self::Shell, _) => ("/bin/zsh", vec!["-l".into()]),
        };
        LaunchSpec {
            program: program.to_owned(),
            args,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_sessions_get_a_fixed_id_for_resume() {
        let spec = AgentKind::Claude.launch(&Purpose::Work {
            claude_session_id: Some("abc".into()),
        });
        assert_eq!(spec.args, ["--session-id", "abc"]);
    }

    #[test]
    fn sign_in_uses_each_cli_s_own_flow() {
        assert_eq!(
            AgentKind::Claude.launch(&Purpose::SignIn).args,
            ["auth", "login"]
        );
        assert_eq!(AgentKind::Codex.launch(&Purpose::SignIn).args, ["login"]);
    }
}
