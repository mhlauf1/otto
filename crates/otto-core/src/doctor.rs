//! The Identity Doctor (SPEC §4.4): who is each workspace actually signed in as?
//!
//! Every check runs the real CLI inside the workspace's isolated environment, exactly
//! as an agent would. Anything not green blocks pushes (via the doctor-status file).

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;
use specta::Type;

use crate::env::{FORBIDDEN, HostEnv, IsolatedEnv};
use crate::hooks;
use crate::workspace::{Workspace, WorkspaceLayout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CheckStatus {
    Pass,
    /// Works, but something deserves attention. Doesn't block pushes.
    Warn,
    /// Not used by this workspace (e.g. no SSH key because it pushes over HTTPS).
    Skipped,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: String,
    pub label: String,
    pub status: CheckStatus,
    /// What we found, in plain language.
    pub detail: String,
    /// How to fix it, when it isn't green.
    pub fix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    pub workspace_id: String,
    pub checks: Vec<Check>,
    /// True when nothing failed.
    pub all_green: bool,
    /// True when every *identity* check passed (ADR-011). Pushes are blocked otherwise.
    pub push_allowed: bool,
}

/// Checks that decide who a push goes out as. Only these gate `pre-push` (ADR-011).
pub const PUSH_GATING: &[&str] = &["hooks", "git", "gh", "ssh"];

fn check(id: &str, label: &str, status: CheckStatus, detail: String, fix: Option<&str>) -> Check {
    Check {
        id: id.into(),
        label: label.into(),
        status,
        detail,
        fix: fix.map(Into::into),
    }
}

/// Result of running a CLI.
#[derive(Debug)]
pub enum Run {
    Done {
        success: bool,
        stdout: String,
        stderr: String,
    },
    NotInstalled,
    TimedOut,
    Failed(String),
}

/// Run `program` in `env` with a deadline. Output is read on threads so a chatty
/// command can't deadlock on a full pipe.
pub fn run(
    env: &IsolatedEnv,
    cwd: Option<&Path>,
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Run {
    let mut cmd = Command::new(program);
    env.apply(&mut cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Run::NotInstalled,
        Err(e) => return Run::Failed(e.to_string()),
    };
    let read = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = String::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_string(&mut buf);
            }
            buf
        })
    };
    let out = read(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let err = read(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Run::TimedOut;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(e) => return Run::Failed(e.to_string()),
        }
    };
    Run::Done {
        success: status.success(),
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
    }
}

const QUICK: Duration = Duration::from_secs(10);
const NETWORK: Duration = Duration::from_secs(20);

fn check_hooks(ws: &Workspace, layout: &WorkspaceLayout) -> Check {
    let dir = layout.hooks_dir();
    let current = |name: &str, expected: String| {
        std::fs::read_to_string(dir.join(name)).is_ok_and(|actual| actual == expected)
    };
    if current("pre-commit", hooks::pre_commit(ws, layout))
        && current("pre-push", hooks::pre_push(ws, layout))
    {
        check(
            "hooks",
            "Commit and push guardrails",
            CheckStatus::Pass,
            "Installed and current.".into(),
            None,
        )
    } else {
        check(
            "hooks",
            "Commit and push guardrails",
            CheckStatus::Fail,
            "The managed hooks are missing or were edited.".into(),
            Some("Relaunch Otto to reinstall them."),
        )
    }
}

fn check_git(ws: &Workspace, env: &IsolatedEnv) -> Check {
    let expected = ws.identity.git_email();
    let label = "Git author";
    match run(env, None, "git", &["var", "GIT_AUTHOR_IDENT"], QUICK) {
        Run::Done {
            success: true,
            stdout,
            ..
        } if stdout.contains(&format!("<{expected}>")) => check(
            "git",
            label,
            CheckStatus::Pass,
            format!("Commits as {expected}."),
            None,
        ),
        Run::Done { stdout, .. } => check(
            "git",
            label,
            CheckStatus::Fail,
            format!(
                "Git would commit as “{}”.",
                stdout.split('>').next().unwrap_or("").trim()
            ),
            Some("Relaunch Otto to rewrite this workspace's git config."),
        ),
        Run::NotInstalled => check(
            "git",
            label,
            CheckStatus::Fail,
            "Git isn't installed.".into(),
            Some("Run xcode-select --install."),
        ),
        other => check("git", label, CheckStatus::Fail, describe(&other), None),
    }
}

fn check_gh(ws: &Workspace, layout: &WorkspaceLayout, env: &IsolatedEnv) -> Check {
    let label = "GitHub CLI";
    let user = ws.identity.github_user().as_str();
    let login_fix = format!(
        "Sign in inside this workspace: GH_CONFIG_DIR=\"{}\" gh auth login --insecure-storage --web (as {user}).",
        layout.gh_dir().display()
    );
    let hosts = std::fs::read_to_string(layout.gh_dir().join("hosts.yml")).unwrap_or_default();
    if hosts.trim().is_empty() {
        return check(
            "gh",
            label,
            CheckStatus::Fail,
            "Not signed in to GitHub in this workspace.".into(),
            Some(&login_fix),
        );
    }
    if !hosts.contains("oauth_token:") {
        return check(
            "gh",
            label,
            CheckStatus::Fail,
            "The token is in the shared macOS Keychain, where other workspaces can reach it."
                .into(),
            Some(&login_fix),
        );
    }
    match run(env, None, "gh", &["api", "user", "--jq", ".login"], NETWORK) {
        Run::Done {
            success: true,
            stdout,
            ..
        } if stdout.trim().eq_ignore_ascii_case(user) => check(
            "gh",
            label,
            CheckStatus::Pass,
            format!("Signed in as {user}."),
            None,
        ),
        Run::Done {
            success: true,
            stdout,
            ..
        } => check(
            "gh",
            label,
            CheckStatus::Fail,
            format!(
                "Signed in as {}, but this workspace is {user}.",
                stdout.trim()
            ),
            Some(&login_fix),
        ),
        Run::Done { stderr, .. } => check(
            "gh",
            label,
            CheckStatus::Fail,
            format!("GitHub didn't accept the token: {}", first_line(&stderr)),
            Some(&login_fix),
        ),
        Run::NotInstalled => check(
            "gh",
            label,
            CheckStatus::Fail,
            "The GitHub CLI isn't installed.".into(),
            Some("brew install gh"),
        ),
        other => check("gh", label, CheckStatus::Fail, describe(&other), None),
    }
}

fn check_ssh(ws: &Workspace, layout: &WorkspaceLayout, env: &IsolatedEnv) -> Check {
    let label = "SSH key";
    if !layout.ssh_key().exists() {
        return check(
            "ssh",
            label,
            CheckStatus::Skipped,
            "No SSH key. This workspace pushes over HTTPS.".into(),
            None,
        );
    }
    let Some(ssh) = env.get("GIT_SSH_COMMAND") else {
        return check(
            "ssh",
            label,
            CheckStatus::Fail,
            "SSH isn't configured.".into(),
            None,
        );
    };
    let probe = format!("{ssh} -o BatchMode=yes -o ConnectTimeout=10 -T git@github.com");
    let user = ws.identity.github_user().as_str();
    match run(env, None, "/bin/sh", &["-c", &probe], NETWORK) {
        // GitHub always exits 1 here; the greeting tells us who we are.
        Run::Done { stderr, .. }
            if stderr.to_ascii_lowercase().contains(&format!("hi {user}!")) =>
        {
            check(
                "ssh",
                label,
                CheckStatus::Pass,
                format!("GitHub recognises the key as {user}."),
                None,
            )
        }
        Run::Done { stderr, .. } => check(
            "ssh",
            label,
            CheckStatus::Fail,
            format!(
                "GitHub didn't recognise the key as {user}: {}",
                first_line(&stderr)
            ),
            Some("Add this workspace's public key to the right GitHub account."),
        ),
        other => check("ssh", label, CheckStatus::Fail, describe(&other), None),
    }
}

fn check_claude(env: &IsolatedEnv) -> Check {
    let label = "Claude Code";
    match run(env, None, "claude", &["auth", "status"], QUICK) {
        Run::Done { stdout, .. } => {
            let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_default();
            if json["loggedIn"].as_bool() == Some(true) {
                let who = json["email"]
                    .as_str()
                    .or(json["account"]["email"].as_str())
                    .unwrap_or("an account");
                check(
                    "claude",
                    label,
                    CheckStatus::Pass,
                    format!("Signed in as {who}."),
                    None,
                )
            } else {
                check(
                    "claude",
                    label,
                    CheckStatus::Fail,
                    "Not signed in in this workspace.".into(),
                    Some("Open a Claude session in this workspace and run /login."),
                )
            }
        }
        Run::NotInstalled => check(
            "claude",
            label,
            CheckStatus::Fail,
            "Claude Code isn't installed.".into(),
            Some("curl -fsSL https://claude.ai/install.sh | bash"),
        ),
        other => check("claude", label, CheckStatus::Fail, describe(&other), None),
    }
}

fn check_codex(env: &IsolatedEnv) -> Check {
    let label = "Codex";
    match run(env, None, "codex", &["login", "status"], QUICK) {
        Run::Done {
            success: true,
            stdout,
            stderr,
        } => {
            let msg = first_line(if stdout.trim().is_empty() {
                &stderr
            } else {
                &stdout
            });
            check("codex", label, CheckStatus::Pass, msg, None)
        }
        Run::Done { .. } => check(
            "codex",
            label,
            CheckStatus::Fail,
            "Not signed in in this workspace.".into(),
            Some("Open a Codex session in this workspace and sign in."),
        ),
        Run::NotInstalled => check(
            "codex",
            label,
            CheckStatus::Fail,
            "The Codex CLI isn't installed.".into(),
            Some("npm install -g @openai/codex"),
        ),
        other => check("codex", label, CheckStatus::Fail, describe(&other), None),
    }
}

/// Claude's updater, run inside a workspace before 2026-10-20, could re-point the shared
/// `claude` command into Otto's data folder. Warn, because Mike's own terminal then
/// depends on Otto's files.
fn check_claude_install(host: &HostEnv, layout: &WorkspaceLayout) -> Check {
    let label = "Claude install";
    let otto_root = layout.root().parent().and_then(std::path::Path::parent);
    let real = crate::housekeeping::claude_program(host);
    match (real, otto_root) {
        (Some(real), Some(root))
            if root.canonicalize().is_ok_and(|root| real.starts_with(root)) =>
        {
            check(
                "claudeInstall",
                label,
                CheckStatus::Warn,
                format!(
                    "Your `claude` command runs a copy inside Otto's data folder ({}).",
                    real.display()
                ),
                Some(
                    "In a normal Terminal (outside Otto), run `claude install` to put Claude back in its usual place.",
                ),
            )
        }
        (Some(real), _) => check(
            "claudeInstall",
            label,
            CheckStatus::Pass,
            format!("Sessions use {}.", real.display()),
            None,
        ),
        (None, _) => check(
            "claudeInstall",
            label,
            CheckStatus::Skipped,
            "Claude Code isn't installed.".into(),
            None,
        ),
    }
}

/// ADR-008 residual risk R1: agent shells source the real rc files. Warn if they export secrets.
fn check_rc_files(host: &HostEnv) -> Check {
    let label = "Shell startup files";
    let mut hits = Vec::new();
    for name in [
        ".zshenv",
        ".zprofile",
        ".zshrc",
        ".bash_profile",
        ".bashrc",
        ".profile",
    ] {
        let Ok(text) = std::fs::read_to_string(host.home.join(name)) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            for var in FORBIDDEN {
                if line.contains(&format!("{var}=")) {
                    hits.push(format!("~/{name} sets {var}"));
                }
            }
        }
    }
    if hits.is_empty() {
        check(
            "rc",
            label,
            CheckStatus::Pass,
            "No tokens or agent sockets exported.".into(),
            None,
        )
    } else {
        hits.dedup();
        check(
            "rc",
            label,
            CheckStatus::Warn,
            hits.join("; "),
            Some(
                "Move these into a tool-specific config. Agent shells read these files in every workspace.",
            ),
        )
    }
}

fn first_line(s: &str) -> String {
    s.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("no output")
        .trim()
        .to_owned()
}

fn describe(run: &Run) -> String {
    match run {
        Run::TimedOut => "Timed out. Check your network connection.".into(),
        Run::Failed(e) => format!("Couldn't run the check: {e}"),
        Run::NotInstalled => "Not installed.".into(),
        Run::Done { stderr, .. } => first_line(stderr),
    }
}

/// Run every check for one workspace. Blocking: call it from a worker thread.
pub fn diagnose(ws: &Workspace, layout: &WorkspaceLayout, host: &HostEnv) -> DoctorReport {
    let env = IsolatedEnv::build(ws, layout, host);
    let checks = vec![
        check_hooks(ws, layout),
        check_git(ws, &env),
        check_gh(ws, layout, &env),
        check_ssh(ws, layout, &env),
        check_claude(&env),
        check_codex(&env),
        check_rc_files(host),
        check_claude_install(host, layout),
    ];
    let all_green = checks.iter().all(|c| c.status != CheckStatus::Fail);
    let push_allowed = checks
        .iter()
        .filter(|c| PUSH_GATING.contains(&c.id.as_str()))
        .all(|c| c.status != CheckStatus::Fail);
    DoctorReport {
        workspace_id: ws.id.to_string(),
        checks,
        all_green,
        push_allowed,
    }
}
