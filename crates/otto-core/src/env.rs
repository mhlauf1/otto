//! The isolated environment every Otto-spawned process runs in (SPEC §4.1, ADR-008).
//!
//! `IsolatedEnv` is the *only* way Otto builds a child environment. It starts from
//! nothing (`env_clear`) and adds an allowlist. Forbidden variables can't be emitted:
//! `set` rejects them, and the isolation tests prove it end to end.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::workspace::{Workspace, WorkspaceLayout};

/// Variables that must never reach a child process, whatever the source.
pub const FORBIDDEN: &[&str] = &[
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
    "GH_HOST",
    "GH_REPO",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_PROFILE",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "OPENAI_API_KEY",
    "CODEX_API_KEY",
    "SSH_AUTH_SOCK",
    "SSH_AGENT_PID",
];

/// Prefixes that are forbidden unless Otto sets the exact variable itself.
const FORBIDDEN_PREFIXES: &[&str] = &["CLAUDE_CODE_USE_", "GIT_"];

/// The `GIT_*` variables Otto itself sets. Every other `GIT_*` is forbidden.
const OTTO_GIT_VARS: &[&str] = &[
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_NOSYSTEM",
    "GIT_AUTHOR_NAME",
    "GIT_AUTHOR_EMAIL",
    "GIT_COMMITTER_NAME",
    "GIT_COMMITTER_EMAIL",
    "GIT_TERMINAL_PROMPT",
    "GIT_ASKPASS",
    "GIT_SSH_COMMAND",
];

/// True if `key` must never appear in a child environment built by Otto.
pub fn is_forbidden(key: &str) -> bool {
    FORBIDDEN.contains(&key)
        || FORBIDDEN_PREFIXES.iter().any(|p| key.starts_with(p)) && !OTTO_GIT_VARS.contains(&key)
}

/// The few facts Otto takes from the machine it runs on. Everything else is decided by Otto.
#[derive(Debug, Clone)]
pub struct HostEnv {
    pub home: PathBuf,
    pub user: String,
    pub shell: String,
    pub lang: String,
    /// Directories searched for CLIs, in order. Built explicitly, never inherited.
    pub path_dirs: Vec<PathBuf>,
}

impl HostEnv {
    /// Snapshot the current user. `PATH` is rebuilt from well-known install locations
    /// rather than copied, because a Finder-launched app gets a minimal PATH and a
    /// terminal-launched one gets whatever the shell rc files added.
    pub fn detect() -> Self {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let home = var("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
        let user = var("USER").unwrap_or_default();
        let mut path_dirs = vec![
            home.join(".local/bin"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ];
        if let Some(node_bin) = newest_nvm_bin(&home) {
            path_dirs.push(node_bin);
        }
        path_dirs.push(home.join(".cargo/bin"));
        path_dirs.extend(["/usr/bin", "/bin", "/usr/sbin", "/sbin"].map(PathBuf::from));
        Self {
            home,
            user,
            shell: var("SHELL").unwrap_or_else(|| "/bin/zsh".to_owned()),
            lang: var("LANG").unwrap_or_else(|| "en_US.UTF-8".to_owned()),
            path_dirs,
        }
    }

    fn path_value(&self) -> String {
        let parts: Vec<String> = self
            .path_dirs
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        parts.join(":")
    }
}

/// The highest installed nvm Node version's `bin`, where npm-installed CLIs like Codex live.
fn newest_nvm_bin(home: &Path) -> Option<PathBuf> {
    let versions = std::fs::read_dir(home.join(".nvm/versions/node")).ok()?;
    let parse = |name: &str| -> Option<(u32, u32, u32)> {
        let mut it = name
            .strip_prefix('v')?
            .split('.')
            .map(|n| n.parse::<u32>().ok());
        Some((it.next()??, it.next()??, it.next()??))
    };
    versions
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            Some((parse(&name)?, e.path().join("bin")))
        })
        .max_by_key(|(v, _)| *v)
        .map(|(_, bin)| bin)
}

/// Quote a value for `sh` (used inside `GIT_SSH_COMMAND`, which git runs through a shell).
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("Otto refused to set {0}: it's on the forbidden list (SPEC §4.1).")]
pub struct ForbiddenVar(pub String);

/// A complete child environment for one workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsolatedEnv {
    vars: BTreeMap<String, String>,
}

impl IsolatedEnv {
    /// Build the environment for `workspace`. Pure: reads nothing from the process environment.
    pub fn build(workspace: &Workspace, layout: &WorkspaceLayout, host: &HostEnv) -> Self {
        let mut env = Self {
            vars: BTreeMap::new(),
        };
        let p = |path: PathBuf| path.display().to_string();
        let id = &workspace.identity;

        // Base: the real HOME (ADR-008), explicit PATH, terminal and locale.
        env.put("HOME", p(host.home.clone()));
        env.put("USER", host.user.clone());
        env.put("LOGNAME", host.user.clone());
        env.put("SHELL", host.shell.clone());
        env.put("PATH", host.path_value());
        env.put("LANG", host.lang.clone());
        env.put("TERM", "xterm-256color".to_owned());
        env.put("COLORTERM", "truecolor".to_owned());
        env.put("TMPDIR", p(layout.tmp_dir()));

        // Which workspace this is. The hooks fail closed if it's missing or wrong.
        env.put("OTTO_WORKSPACE", workspace.id.to_string());

        // Agent CLIs.
        env.put("CLAUDE_CONFIG_DIR", p(layout.claude_dir()));
        env.put("ANTHROPIC_CONFIG_DIR", p(layout.anthropic_dir()));
        env.put("CLAUDE_CODE_TMPDIR", p(layout.tmp_dir()));
        env.put("CODEX_HOME", p(layout.codex_dir()));
        // One Claude install, updated by Mike's own terminal. Left on, each workspace's
        // updater downloaded its own copy (~500 MB each) and re-pointed ~/.local/bin/claude
        // into Otto's data folder (found 2026-10-20).
        env.put("DISABLE_AUTOUPDATER", "1".to_owned());

        // GitHub CLI: token lives in <ws>/gh/hosts.yml (ADR-005).
        env.put("GH_CONFIG_DIR", p(layout.gh_dir()));
        env.put("GH_PROMPT_DISABLED", "1".to_owned());
        env.put("GH_NO_UPDATE_NOTIFIER", "1".to_owned());

        // Git: only the workspace config, and identity pinned by env (beats repo-local config).
        env.put("GIT_CONFIG_GLOBAL", p(layout.gitconfig()));
        env.put("GIT_CONFIG_NOSYSTEM", "1".to_owned());
        env.put("GIT_AUTHOR_NAME", id.git_name().to_owned());
        env.put("GIT_AUTHOR_EMAIL", id.git_email().to_owned());
        env.put("GIT_COMMITTER_NAME", id.git_name().to_owned());
        env.put("GIT_COMMITTER_EMAIL", id.git_email().to_owned());
        env.put("GIT_TERMINAL_PROMPT", "0".to_owned());
        // Git falls back to SSH_ASKPASS for HTTPS passwords. Never let it: HTTPS
        // credentials come only from gh, and the SSH passphrase must never leave ssh.
        env.put("GIT_ASKPASS", "/usr/bin/false".to_owned());

        // SSH: ignore ~/.ssh/config and any agent; this workspace's key and known_hosts only (ADR-006).
        let ssh = [
            "ssh -F none".to_owned(),
            format!("-i {}", shell_quote(&p(layout.ssh_key()))),
            "-o IdentitiesOnly=yes -o IdentityAgent=none".to_owned(),
            format!(
                "-o UserKnownHostsFile={}",
                shell_quote(&p(layout.known_hosts()))
            ),
            "-o StrictHostKeyChecking=accept-new -o ControlMaster=no -o ControlPath=none"
                .to_owned(),
        ];
        env.put("GIT_SSH_COMMAND", ssh.join(" "));
        env.put("SSH_ASKPASS", p(layout.askpass()));
        env.put("SSH_ASKPASS_REQUIRE", "force".to_owned());

        // XDG and shell rc files stay inside the workspace.
        env.put("XDG_CONFIG_HOME", p(layout.xdg_dir("config")));
        env.put("XDG_DATA_HOME", p(layout.xdg_dir("data")));
        env.put("XDG_STATE_HOME", p(layout.xdg_dir("state")));
        env.put("XDG_CACHE_HOME", p(layout.xdg_dir("cache")));
        env.put("ZDOTDIR", p(layout.zsh_dir()));

        env
    }

    /// Internal setter for Otto's own keys. These are compile-time constants, and the
    /// tests check every one against `is_forbidden`.
    fn put(&mut self, key: &str, value: String) {
        debug_assert!(!is_forbidden(key), "Otto tried to set forbidden var {key}");
        self.vars.insert(key.to_owned(), value);
    }

    /// Add an extra variable, for example a session ID.
    ///
    /// # Errors
    /// Refuses anything on the forbidden list.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), ForbiddenVar> {
        if is_forbidden(key) {
            return Err(ForbiddenVar(key.to_owned()));
        }
        self.vars.insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.vars.get(key).map(String::as_str)
    }

    pub fn vars(&self) -> &BTreeMap<String, String> {
        &self.vars
    }

    /// Replace `cmd`'s entire environment with this one. Anything set on `cmd` before is dropped.
    pub fn apply<'c>(&self, cmd: &'c mut Command) -> &'c mut Command {
        cmd.env_clear().envs(&self.vars)
    }

    /// The same environment as `(OsString, OsString)` pairs, for PTY builders (M2).
    pub fn to_os_pairs(&self) -> Vec<(OsString, OsString)> {
        self.vars
            .iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{GithubOwner, Identity, OttoPaths, Tint, WorkspaceId};

    fn fixture() -> (Workspace, WorkspaceLayout, HostEnv) {
        let ws = Workspace::new(
            WorkspaceId::parse("lauf").unwrap(),
            "Lauf",
            Tint::Jade,
            Identity::new("Mike", "mhlauf1@gmail.com", "mhlauf1").unwrap(),
            vec![GithubOwner::parse("mhlauf1").unwrap()],
        )
        .unwrap();
        let layout = OttoPaths::new("/tmp/Otto Test").workspace(&ws.id);
        let host = HostEnv {
            home: "/Users/m".into(),
            user: "m".into(),
            shell: "/bin/zsh".into(),
            lang: "en_US.UTF-8".into(),
            path_dirs: vec!["/usr/bin".into(), "/bin".into()],
        };
        (ws, layout, host)
    }

    #[test]
    fn never_emits_a_forbidden_variable() {
        let (ws, layout, host) = fixture();
        let env = IsolatedEnv::build(&ws, &layout, &host);
        for key in env.vars().keys() {
            assert!(!is_forbidden(key), "{key} is forbidden");
        }
    }

    #[test]
    fn forbids_tokens_agents_and_unknown_git_vars() {
        for key in FORBIDDEN {
            assert!(is_forbidden(key));
        }
        assert!(is_forbidden("GIT_CONFIG_PARAMETERS"));
        assert!(is_forbidden("GIT_EXEC_PATH"));
        assert!(is_forbidden("GIT_DIR"));
        assert!(is_forbidden("CLAUDE_CODE_USE_BEDROCK"));
        assert!(!is_forbidden("GIT_AUTHOR_EMAIL"));
        assert!(!is_forbidden("CLAUDE_CONFIG_DIR"));
    }

    #[test]
    fn set_refuses_forbidden_keys() {
        let (ws, layout, host) = fixture();
        let mut env = IsolatedEnv::build(&ws, &layout, &host);
        assert_eq!(
            env.set("GH_TOKEN", "x"),
            Err(ForbiddenVar("GH_TOKEN".into()))
        );
        assert!(env.set("OTTO_SESSION", "abc").is_ok());
    }

    #[test]
    fn pins_identity_and_config_to_the_workspace() {
        let (ws, layout, host) = fixture();
        let env = IsolatedEnv::build(&ws, &layout, &host);
        assert_eq!(env.get("GIT_AUTHOR_EMAIL"), Some("mhlauf1@gmail.com"));
        assert_eq!(env.get("GIT_CONFIG_NOSYSTEM"), Some("1"));
        assert_eq!(
            env.get("CLAUDE_CONFIG_DIR"),
            Some("/tmp/Otto Test/workspaces/lauf/claude")
        );
        assert_eq!(env.get("OTTO_WORKSPACE"), Some("lauf"));
        let ssh = env.get("GIT_SSH_COMMAND").unwrap();
        assert!(ssh.starts_with("ssh -F none "));
        assert!(ssh.contains("-i '/tmp/Otto Test/workspaces/lauf/ssh/id_ed25519'"));
    }

    #[test]
    fn claude_never_updates_itself_inside_a_workspace() {
        let (ws, layout, host) = fixture();
        let env = IsolatedEnv::build(&ws, &layout, &host);
        assert_eq!(env.get("DISABLE_AUTOUPDATER"), Some("1"));
    }

    #[test]
    fn shell_quote_survives_quotes() {
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }
}
