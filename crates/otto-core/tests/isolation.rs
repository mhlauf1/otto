//! SPEC §4.6 isolation suite. The build can't ship if any of these fail.
//!
//! "GitHub" is simulated by local bare repos: each test repo's remote is a real
//! `https://github.com/<owner>/…` URL, rewritten by a repo-local `insteadOf` to a bare
//! repo on disk. The hooks see the real GitHub URL; the bytes go to the bare repo.
//! No test may touch the network.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use otto_core::doctor::{self, CheckStatus};
use otto_core::env::{FORBIDDEN, HostEnv, IsolatedEnv, is_forbidden};
use otto_core::provision::{provision, write_doctor_status};
use otto_core::repo::{RemotePolicy, RepoRefusal, verify_repo};
use otto_core::store::default_workspaces;
use otto_core::workspace::{OttoPaths, Workspace, WorkspaceLayout};

struct World {
    dir: tempfile::TempDir,
    paths: OttoPaths,
    host: HostEnv,
    playbook: Workspace,
    lauf: Workspace,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        // A space in the path, like the real "Application Support".
        let paths = OttoPaths::new(dir.path().join("Otto Support"));
        let fakebin = dir.path().join("fakebin");
        fs::create_dir_all(&fakebin).unwrap();
        let mut host = HostEnv::detect();
        host.path_dirs.insert(0, fakebin);
        let mut ws = default_workspaces().unwrap().into_iter();
        let (playbook, lauf) = (ws.next().unwrap(), ws.next().unwrap());
        let world = Self {
            dir,
            paths,
            host,
            playbook,
            lauf,
        };
        for w in [&world.playbook, &world.lauf] {
            provision(w, &world.layout(w)).unwrap();
            write_doctor_status(&world.layout(w), true).unwrap();
        }
        world
    }

    fn layout(&self, ws: &Workspace) -> WorkspaceLayout {
        self.paths.workspace(&ws.id)
    }

    fn env(&self, ws: &Workspace) -> IsolatedEnv {
        IsolatedEnv::build(ws, &self.layout(ws), &self.host)
    }

    /// Put an executable script on the fake PATH (shadows the real CLI).
    fn fake_cli(&self, name: &str, body: &str) {
        let path = self.dir.path().join("fakebin").join(name);
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// A repo whose `origin` is `https://github.com/<owner>/project.git`, backed by a bare repo.
    fn repo(&self, ws: &Workspace, owner: &str) -> (PathBuf, PathBuf) {
        let base = self.dir.path().join(format!("{}-{owner}", ws.id));
        let bare = base.join("remote.git");
        let work = base.join("work");
        fs::create_dir_all(&work).unwrap();
        let env = self.env(ws);
        git_ok(&env, &base, &["init", "-q", "--bare", "remote.git"]);
        git_ok(&env, &work, &["init", "-q"]);
        let url = format!("https://github.com/{owner}/project.git");
        git_ok(&env, &work, &["remote", "add", "origin", &url]);
        let rewrite = format!("url.{}.insteadOf", bare.display());
        git_ok(&env, &work, &["config", &rewrite, &url]);
        fs::write(work.join("README.md"), "hi\n").unwrap();
        git_ok(&env, &work, &["add", "README.md"]);
        git_ok(&env, &work, &["commit", "-q", "-m", "first"]);
        (work, bare)
    }
}

fn git(env: &IsolatedEnv, cwd: &Path, args: &[&str]) -> Output {
    env.apply(Command::new("git").current_dir(cwd).args(args))
        .output()
        .unwrap()
}

fn git_ok(env: &IsolatedEnv, cwd: &Path, args: &[&str]) -> Output {
    let out = git(env, cwd, args);
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        stderr(&out)
    );
    out
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn has_branch(bare: &Path, env: &IsolatedEnv) -> bool {
    let out = git(env, bare, &["show-ref", "--heads"]);
    !String::from_utf8_lossy(&out.stdout).trim().is_empty()
}

// ── Test 1: pushing a Lauf-owned remote from Playbook is blocked ───────────

#[test]
fn t1_playbook_cannot_push_to_a_lauf_owned_remote() {
    let w = World::new();
    let env = w.env(&w.playbook);
    let (work, bare) = w.repo(&w.playbook, "mhlauf1");
    let out = git(&env, &work, &["push", "-q", "origin", "HEAD:main"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("github.com/mhlauf1"),
        "{}",
        stderr(&out)
    );
    assert!(!has_branch(&bare, &env), "nothing may reach the remote");
}

#[test]
fn t1_control_pushing_to_an_allowlisted_owner_works() {
    let w = World::new();
    let env = w.env(&w.playbook);
    let (work, bare) = w.repo(&w.playbook, "mikelauf");
    git_ok(&env, &work, &["push", "-q", "origin", "HEAD:main"]);
    assert!(has_branch(&bare, &env));
}

#[test]
fn t1_an_insteadof_rewrite_cannot_redirect_a_push_to_another_owner() {
    // Git contacts the remote before `pre-push` runs, so a live redirect would need the
    // network. Instead, call the hook exactly as git does: remote name, then the
    // *effective* (rewritten) URL. The configured URL is allowed; the effective one isn't.
    let w = World::new();
    let env = w.env(&w.playbook);
    let (work, _) = w.repo(&w.playbook, "mikelauf");
    let hook = w.layout(&w.playbook).hooks_dir().join("pre-push");
    let out = env
        .apply(Command::new(&hook).current_dir(&work))
        .args(["origin", "https://github.com/mhlauf1/project.git"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("github.com/mhlauf1"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn t1_git_never_offers_the_ssh_passphrase_helper_as_an_https_password() {
    // No gh login in this workspace: git must fail cleanly, not fall back to SSH_ASKPASS.
    let w = World::new();
    let env = w.env(&w.playbook);
    let marker = w.dir.path().join("askpass-was-called");
    w.fake_cli(
        "probe-askpass",
        &format!("touch '{}'; echo secret", marker.display()),
    );
    let mut env = env;
    env.set(
        "SSH_ASKPASS",
        &w.dir
            .path()
            .join("fakebin/probe-askpass")
            .display()
            .to_string(),
    )
    .unwrap();
    let mut child = env
        .apply(
            Command::new("git")
                .current_dir(w.dir.path())
                .args(["credential", "fill"]),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"protocol=https\nhost=github.com\npath=mikelauf/project.git\n\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(!out.status.success(), "git must not invent a credential");
    assert!(!String::from_utf8_lossy(&out.stdout).contains("secret"));
    assert!(
        !marker.exists(),
        "git called the SSH askpass helper for an HTTPS password"
    );
}

#[test]
fn askpass_answers_only_its_own_key_prompt() {
    let w = World::new();
    let layout = w.layout(&w.playbook);
    let askpass = layout.askpass();
    for prompt in [
        "Password for 'https://mikelauf@github.com': ",
        "Username for 'https://github.com': ",
    ] {
        let out = Command::new(&askpass).arg(prompt).output().unwrap();
        assert!(!out.status.success(), "answered {prompt}");
        assert_eq!(out.stdout, b"", "leaked an answer");
    }
}

#[test]
fn t1_pushes_are_blocked_while_the_identity_doctor_is_red() {
    let w = World::new();
    let env = w.env(&w.playbook);
    let (work, bare) = w.repo(&w.playbook, "mikelauf");
    write_doctor_status(&w.layout(&w.playbook), false).unwrap();
    let out = git(&env, &work, &["push", "-q", "origin", "HEAD:main"]);
    assert!(!out.status.success());
    assert!(stderr(&out).contains("Identity Doctor"), "{}", stderr(&out));
    assert!(!has_branch(&bare, &env));
}

#[test]
fn t1_hooks_fail_closed_outside_their_workspace() {
    let w = World::new();
    let env = w.env(&w.playbook);
    let (work, bare) = w.repo(&w.playbook, "mikelauf");
    let mut cmd = Command::new("git");
    env.apply(&mut cmd).env_remove("OTTO_WORKSPACE");
    let out = cmd
        .current_dir(&work)
        .args(["push", "-q", "origin", "HEAD:main"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("running as 'unknown'"),
        "{}",
        stderr(&out)
    );
    assert!(!has_branch(&bare, &env));
}

// ── Test 2: committing with the wrong email is blocked ─────────────────────

#[test]
fn t2_commit_with_a_foreign_author_is_blocked() {
    let w = World::new();
    let env = w.env(&w.playbook);
    let (work, _) = w.repo(&w.playbook, "mikelauf");
    fs::write(work.join("a.txt"), "a").unwrap();
    git_ok(&env, &work, &["add", "a.txt"]);
    let out = git(
        &env,
        &work,
        &[
            "commit",
            "-q",
            "-m",
            "x",
            "--author=Mike <mhlauf1@gmail.com>",
        ],
    );
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("<mhlauf1@gmail.com>"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn t2_repo_local_email_cannot_override_the_workspace_identity() {
    let w = World::new();
    let env = w.env(&w.playbook);
    let (work, _) = w.repo(&w.playbook, "mikelauf");
    git_ok(&env, &work, &["config", "user.email", "mhlauf1@gmail.com"]);
    git_ok(&env, &work, &["commit", "-q", "--allow-empty", "-m", "x"]);
    let log = git_ok(&env, &work, &["log", "-1", "--format=%ae"]);
    assert_eq!(
        String::from_utf8_lossy(&log.stdout).trim(),
        "michael.laufersweiler@playbookexp.com"
    );
}

#[test]
fn t2_a_foreign_commit_made_with_no_verify_still_cannot_be_pushed() {
    let w = World::new();
    let env = w.env(&w.playbook);
    let (work, bare) = w.repo(&w.playbook, "mikelauf");
    git_ok(
        &env,
        &work,
        &[
            "commit",
            "-q",
            "--no-verify",
            "--allow-empty",
            "-m",
            "x",
            "--author=M <mhlauf1@gmail.com>",
        ],
    );
    let out = git(&env, &work, &["push", "-q", "origin", "HEAD:main"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("commit by <mhlauf1@gmail.com>"),
        "{}",
        stderr(&out)
    );
    assert!(!has_branch(&bare, &env));
}

// ── Test 3: spawned processes contain no stripped variables ────────────────

fn child_env(env: &IsolatedEnv, program: &str, args: &[&str]) -> Vec<(String, String)> {
    let mut cmd = Command::new(program);
    // Pollute the command first: apply() must wipe all of this.
    for var in FORBIDDEN {
        cmd.env(var, "leaked");
    }
    cmd.env("GIT_CONFIG_PARAMETERS", "'user.email'='leak@x.com'")
        .env("CLAUDE_CODE_USE_BEDROCK", "1");
    let out = env.apply(&mut cmd).args(args).output().unwrap();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned())))
        .collect()
}

#[test]
fn t3_spawned_processes_contain_no_stripped_variables() {
    let w = World::new();
    for ws in [&w.playbook, &w.lauf] {
        let env = w.env(ws);
        for (program, args) in [
            ("/usr/bin/env", vec![]),
            ("/bin/sh", vec!["-c", "env"]),
            ("/bin/zsh", vec!["-c", "env"]),
        ] {
            let vars = child_env(&env, program, &args);
            assert_ne!(vars.len(), 0, "{program} printed no environment");
            for (k, v) in &vars {
                assert!(!is_forbidden(k), "{program} saw forbidden {k}");
                assert_ne!(v, "leaked", "{program} saw a leaked value in {k}");
            }
        }
    }
}

// ── Test 4: two workspaces at once never read each other's config ──────────

fn write_gh_login(layout: &WorkspaceLayout, user: &str, token: &str) {
    let hosts = format!(
        "github.com:\n    users:\n        {user}:\n            oauth_token: {token}\n    git_protocol: https\n    oauth_token: {token}\n    user: {user}\n"
    );
    fs::write(layout.gh_dir().join("hosts.yml"), hosts).unwrap();
}

#[test]
fn t4_concurrent_workspaces_only_read_their_own_git_config() {
    let w = World::new();
    let handles: Vec<_> = [&w.playbook, &w.lauf]
        .into_iter()
        .map(|ws| {
            let env = w.env(ws);
            let own = w.layout(ws).root().display().to_string();
            let other_ws = if ws.id == w.playbook.id {
                &w.lauf
            } else {
                &w.playbook
            };
            let other = w.layout(other_ws).root().display().to_string();
            let cwd = w.dir.path().to_path_buf();
            std::thread::spawn(move || {
                for _ in 0..20 {
                    let all = git(&env, &cwd, &["config", "--list", "--show-origin"]);
                    let text = String::from_utf8_lossy(&all.stdout).into_owned();
                    assert!(text.contains(&own), "own config not read: {text}");
                    assert!(!text.contains(&other), "read the other workspace: {text}");
                    for line in text.lines() {
                        assert!(
                            line.starts_with(&format!("file:{own}")),
                            "unexpected config source: {line}"
                        );
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn t4_each_workspace_only_ever_presents_its_own_github_credential() {
    // The structural guarantee behind ADR-007: even with --no-verify, git can only
    // hand GitHub the token that belongs to this workspace. Uses the real gh.
    let w = World::new();
    write_gh_login(&w.layout(&w.playbook), "mikelauf", "fake-playbook-token");
    write_gh_login(&w.layout(&w.lauf), "mhlauf1", "fake-lauf-token");

    let handles: Vec<_> = [
        (&w.playbook, "fake-playbook-token"),
        (&w.lauf, "fake-lauf-token"),
    ]
    .into_iter()
    .map(|(ws, expected)| {
        let env = w.env(ws);
        let cwd = w.dir.path().to_path_buf();
        std::thread::spawn(move || {
            for _ in 0..5 {
                let token = env
                    .apply(Command::new("gh").args(["auth", "token"]))
                    .output()
                    .expect("the GitHub CLI must be installed for the isolation suite");
                assert_eq!(String::from_utf8_lossy(&token.stdout).trim(), expected);

                // What git would send to github.com for *any* owner's repo.
                for path in ["mikelauf/project.git", "mhlauf1/project.git"] {
                    let mut child = env
                        .apply(
                            Command::new("git")
                                .current_dir(&cwd)
                                .args(["credential", "fill"]),
                        )
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .unwrap();
                    let req = format!("protocol=https\nhost=github.com\npath={path}\n\n");
                    child
                        .stdin
                        .take()
                        .unwrap()
                        .write_all(req.as_bytes())
                        .unwrap();
                    let out = child.wait_with_output().unwrap();
                    let text = String::from_utf8_lossy(&out.stdout);
                    assert!(
                        text.contains(&format!("password={expected}")),
                        "{path}: {text} {}",
                        stderr(&out)
                    );
                }
            }
        })
    })
    .collect();
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn t4_workspace_environments_share_no_paths() {
    let w = World::new();
    let (a, b) = (w.env(&w.playbook), w.env(&w.lauf));
    let root_a = w.layout(&w.playbook).root().display().to_string();
    let root_b = w.layout(&w.lauf).root().display().to_string();
    for (key, value) in a.vars() {
        assert!(
            !value.contains(&root_b),
            "{key} in Playbook points into Lauf"
        );
    }
    for (key, value) in b.vars() {
        assert!(
            !value.contains(&root_a),
            "{key} in Lauf points into Playbook"
        );
    }
}

// ── Repo registration (SPEC §4.2) ──────────────────────────────────────────

#[test]
fn repos_only_open_in_the_workspace_that_owns_their_remote() {
    let w = World::new();
    let (work, _) = w.repo(&w.lauf, "mhlauf1");
    let ok = verify_repo(
        &w.lauf,
        &w.layout(&w.lauf),
        &w.env(&w.lauf),
        &work,
        RemotePolicy::RequireGithubRemote,
    )
    .unwrap();
    assert_eq!(
        ok.remote_url.as_deref(),
        Some("https://github.com/mhlauf1/project.git")
    );
    let refused = verify_repo(
        &w.playbook,
        &w.layout(&w.playbook),
        &w.env(&w.playbook),
        &work,
        RemotePolicy::RequireGithubRemote,
    )
    .unwrap_err();
    assert_eq!(
        refused,
        RepoRefusal::ForeignOwner {
            remote: "origin".into(),
            owner: "mhlauf1".into()
        }
    );
}

#[test]
fn repos_that_override_guardrails_are_refused() {
    let w = World::new();
    let env = w.env(&w.lauf);
    let (work, _) = w.repo(&w.lauf, "mhlauf1");
    git_ok(&env, &work, &["config", "core.hooksPath", "/dev/null"]);
    let refused = verify_repo(
        &w.lauf,
        &w.layout(&w.lauf),
        &env,
        &work,
        RemotePolicy::RequireGithubRemote,
    )
    .unwrap_err();
    assert_eq!(refused, RepoRefusal::HooksOverridden);
}

// ── Identity Doctor ────────────────────────────────────────────────────────

#[test]
fn doctor_reports_the_signed_in_github_user_and_blocks_on_mismatch() {
    let w = World::new();
    let layout = w.layout(&w.lauf);
    write_gh_login(&layout, "mhlauf1", "fake");
    w.fake_cli(
        "claude",
        r#"echo '{"loggedIn":true,"email":"mhlauf1@gmail.com"}'"#,
    );
    w.fake_cli("codex", "echo 'Logged in using ChatGPT'");

    w.fake_cli("gh", "echo mhlauf1");
    let report = doctor::diagnose(&w.lauf, &layout, &w.host);
    let status = |id: &str| report.checks.iter().find(|c| c.id == id).unwrap().status;
    assert_eq!(status("gh"), CheckStatus::Pass);
    assert_eq!(status("git"), CheckStatus::Pass);
    assert_eq!(status("hooks"), CheckStatus::Pass);
    assert_eq!(status("claude"), CheckStatus::Pass);
    assert_eq!(status("codex"), CheckStatus::Pass);
    assert_eq!(status("ssh"), CheckStatus::Skipped);

    // gh is signed in as the *other* identity: red, and pushes blocked.
    w.fake_cli("gh", "echo mikelauf");
    let report = doctor::diagnose(&w.lauf, &layout, &w.host);
    let gh = report.checks.iter().find(|c| c.id == "gh").unwrap();
    assert_eq!(gh.status, CheckStatus::Fail);
    assert!(gh.detail.contains("mikelauf"));
    assert!(!report.all_green);
    assert!(!report.push_allowed);
}

#[test]
fn doctor_agent_logins_show_red_but_do_not_gate_pushes() {
    // ADR-011: a missing Codex login is a problem, but not an identity problem.
    let w = World::new();
    let layout = w.layout(&w.lauf);
    write_gh_login(&layout, "mhlauf1", "fake");
    w.fake_cli("gh", "echo mhlauf1");
    w.fake_cli("claude", r#"echo '{"loggedIn":false}'; exit 1"#);
    w.fake_cli("codex", "exit 1");
    let report = doctor::diagnose(&w.lauf, &layout, &w.host);
    assert!(!report.all_green);
    assert!(report.push_allowed);
}

#[test]
fn doctor_flags_a_token_left_in_the_shared_keychain() {
    let w = World::new();
    let layout = w.layout(&w.lauf);
    fs::write(
        layout.gh_dir().join("hosts.yml"),
        "github.com:\n    user: mhlauf1\n    git_protocol: https\n",
    )
    .unwrap();
    let report = doctor::diagnose(&w.lauf, &layout, &w.host);
    let gh = report.checks.iter().find(|c| c.id == "gh").unwrap();
    assert_eq!(gh.status, CheckStatus::Fail);
    assert!(gh.detail.contains("Keychain"));
}

// ── Terminal sessions (M2) ─────────────────────────────────────────────────

fn run_in_pty(
    env: &IsolatedEnv,
    spec: &otto_core::agent::LaunchSpec,
    cwd: &Path,
) -> (String, Option<u32>) {
    use otto_core::pty::{PtyEvent, PtySession};
    let (tx, rx) = std::sync::mpsc::channel();
    let _session = PtySession::spawn(env, spec, cwd, 200, 50, move |e| {
        let _ = tx.send(e);
    })
    .unwrap();
    let mut out = String::new();
    loop {
        match rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("pty timed out")
        {
            PtyEvent::Data { text } => out.push_str(&text),
            PtyEvent::Exit { code } => return (out, code),
        }
    }
}

#[test]
fn t3_agent_sessions_in_a_pty_get_only_the_isolated_environment() {
    use otto_core::agent::{AgentKind, Purpose};
    let w = World::new();
    // Stand-in for the real CLI: prints the environment it was given.
    w.fake_cli("claude", "echo \"session-args: $*\"; /usr/bin/env");
    let env = w.env(&w.lauf);
    let spec = AgentKind::Claude.launch(&Purpose::Work {
        claude_session_id: Some("s-1".into()),
    });
    let (out, code) = run_in_pty(&env, &spec, w.dir.path());
    assert_eq!(code, Some(0));
    assert!(out.contains("session-args: --session-id s-1"), "{out}");
    assert!(out.contains("OTTO_WORKSPACE=lauf"));
    assert!(out.contains(&format!(
        "CLAUDE_CONFIG_DIR={}",
        w.layout(&w.lauf).claude_dir().display()
    )));
    for line in out.lines() {
        if let Some((key, _)) = line.trim_end_matches('\r').split_once('=') {
            assert!(!is_forbidden(key), "PTY session saw forbidden {key}");
        }
    }
}

#[test]
fn t4_two_pty_sessions_run_side_by_side_as_different_identities() {
    use otto_core::agent::{AgentKind, Purpose};
    let w = World::new();
    w.fake_cli(
        "claude",
        "echo \"who=$GIT_AUTHOR_EMAIL cfg=$CLAUDE_CONFIG_DIR\"; sleep 0.3",
    );
    let spec = AgentKind::Claude.launch(&Purpose::Work {
        claude_session_id: None,
    });
    let a = {
        let env = w.env(&w.playbook);
        let spec = spec.clone();
        let cwd = w.dir.path().to_path_buf();
        std::thread::spawn(move || run_in_pty(&env, &spec, &cwd).0)
    };
    let (b, _) = run_in_pty(&w.env(&w.lauf), &spec, w.dir.path());
    let a = a.join().unwrap();
    assert!(
        a.contains("who=michael.laufersweiler@playbookexp.com"),
        "{a}"
    );
    assert!(a.contains("/workspaces/playbook/claude"));
    assert!(b.contains("who=mhlauf1@gmail.com"), "{b}");
    assert!(b.contains("/workspaces/lauf/claude"));
}

// ── Credentials that bypass the workspace login (found in Mike's repos, 2026-10-20) ──

#[test]
fn t1_a_token_in_the_remote_url_is_refused_and_cannot_push() {
    let w = World::new();
    let env = w.env(&w.lauf);
    let (work, bare) = w.repo(&w.lauf, "mhlauf1");
    // A token embedded in an otherwise-allowed remote would push as *that token's* account.
    let url = "https://ghp_FAKEFAKEFAKEFAKEFAKEFAKEFAKEFAKE1234@github.com/mhlauf1/project.git";
    git_ok(&env, &work, &["remote", "set-url", "origin", url]);
    git_ok(
        &env,
        &work,
        &["config", &format!("url.{}.insteadOf", bare.display()), url],
    );
    let refused = verify_repo(
        &w.lauf,
        &w.layout(&w.lauf),
        &env,
        &work,
        RemotePolicy::RequireGithubRemote,
    )
    .unwrap_err();
    assert!(
        matches!(refused, RepoRefusal::SecretInConfig(_)),
        "{refused:?}"
    );
    let out = git(&env, &work, &["push", "-q", "origin", "HEAD:main"]);
    assert!(!out.status.success());
    assert!(
        stderr(&out).contains("contains a credential"),
        "{}",
        stderr(&out)
    );
    assert!(!has_branch(&bare, &env));
}

#[test]
fn t1_an_http_auth_header_cannot_be_used_to_push() {
    let w = World::new();
    let env = w.env(&w.lauf);
    let (work, bare) = w.repo(&w.lauf, "mhlauf1");
    // Repo config…
    git_ok(
        &env,
        &work,
        &[
            "config",
            "http.https://github.com/.extraheader",
            "AUTHORIZATION: basic eA==",
        ],
    );
    let refused = verify_repo(
        &w.lauf,
        &w.layout(&w.lauf),
        &env,
        &work,
        RemotePolicy::RequireGithubRemote,
    )
    .unwrap_err();
    assert!(
        matches!(refused, RepoRefusal::AuthHeaderOverride(_)),
        "{refused:?}"
    );
    git_ok(
        &env,
        &work,
        &["config", "--unset", "http.https://github.com/.extraheader"],
    );
    // …or passed on the command line with `git -c`.
    let out = git(
        &env,
        &work,
        &[
            "-c",
            "http.extraHeader=AUTHORIZATION: basic eA==",
            "push",
            "-q",
            "origin",
            "HEAD:main",
        ],
    );
    assert!(!out.status.success());
    assert!(stderr(&out).contains("extraHeader"), "{}", stderr(&out));
    assert!(!has_branch(&bare, &env));
}

#[test]
fn local_only_repos_open_when_assigned_but_never_with_a_foreign_remote() {
    let w = World::new();
    let env = w.env(&w.lauf);
    let dir = w.dir.path().join("local-only");
    fs::create_dir_all(&dir).unwrap();
    git_ok(&env, &dir, &["init", "-q"]);
    git_ok(
        &env,
        &dir,
        &["commit", "-q", "--allow-empty", "-m", "first"],
    );
    let layout = w.layout(&w.lauf);
    assert_eq!(
        verify_repo(
            &w.lauf,
            &layout,
            &env,
            &dir,
            RemotePolicy::RequireGithubRemote
        )
        .unwrap_err(),
        RepoRefusal::NoRemotes
    );
    let ok = verify_repo(&w.lauf, &layout, &env, &dir, RemotePolicy::AllowLocalOnly).unwrap();
    assert_eq!(ok.remote_url, None);
    // Adding a remote owned by the other identity later doesn't get it through.
    git_ok(
        &env,
        &dir,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/mikelauf/x.git",
        ],
    );
    let refused =
        verify_repo(&w.lauf, &layout, &env, &dir, RemotePolicy::AllowLocalOnly).unwrap_err();
    assert!(
        matches!(refused, RepoRefusal::ForeignOwner { .. }),
        "{refused:?}"
    );
}

/// ADR-017: groups live inside one workspace. Neither the store nor the database
/// itself lets a project sit in, or be moved by, another workspace.
#[tokio::test]
async fn project_groups_never_cross_workspaces() {
    use otto_core::repo::VerifiedRepo;
    use otto_core::store::{Store, StoreError};
    use otto_core::workspace::WorkspaceId;

    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("otto.db")).await.unwrap();
    store.seed_defaults().await.unwrap();
    let (playbook, lauf) = (
        WorkspaceId::parse("playbook").unwrap(),
        WorkspaceId::parse("lauf").unwrap(),
    );
    let core = VerifiedRepo {
        path: "/core".into(),
        remote_url: Some("https://github.com/lulafit/core".into()),
    };
    store.register_repo(&playbook, &core).await.unwrap();
    let pb_group = store.create_project_group(&playbook, "Core").await.unwrap();

    // A Playbook project can't go into a Lauf group, through either workspace.
    for actor in [&playbook, &lauf] {
        let moved = store
            .move_project(actor, "/core", Some("lauf-clients"))
            .await;
        assert!(matches!(moved, Err(StoreError::Rejected(_))), "{moved:?}");
    }
    // Lauf can't move, rename or delete Playbook's projects or groups.
    assert!(store.move_project(&lauf, "/core", None).await.is_err());
    assert!(store.rename_project(&lauf, "/core", "Mine").await.is_err());
    assert!(
        store
            .rename_project_group(&lauf, &pb_group.id, "Mine")
            .await
            .is_err()
    );
    assert!(
        store
            .delete_project_group(&lauf, &pb_group.id)
            .await
            .is_err()
    );
    // Nothing changed, and the legitimate move still works.
    let core_now = &store.projects(&playbook).await.unwrap()[0];
    assert_eq!(
        (core_now.group_id.as_deref(), core_now.name.as_str()),
        (None, "core")
    );
    store
        .move_project(&playbook, "/core", Some(&pb_group.id))
        .await
        .unwrap();
    assert!(
        store
            .project_groups(&lauf)
            .await
            .unwrap()
            .iter()
            .all(|g| g.id != pb_group.id)
    );
}

/// Past conversations are read from each workspace's own Claude config, so a Playbook
/// conversation can't be listed (or resumed) from Lauf, even for the same folder.
#[test]
fn past_conversations_never_cross_workspaces() {
    use otto_core::history;

    let world = World::new();
    let cwd = world.dir.path().join("shared-folder");
    let id = "7de656b9-96e1-4226-9fbe-c8d129813f94";
    let playbook_dir =
        history::conversations_dir(&world.layout(&world.playbook).claude_dir(), &cwd);
    fs::create_dir_all(&playbook_dir).unwrap();
    fs::write(
        playbook_dir.join(format!("{id}.jsonl")),
        r#"{"type":"user","message":{"content":"playbook roadmap"}}"#,
    )
    .unwrap();

    let playbook = history::list(&world.layout(&world.playbook).claude_dir(), &cwd, 10);
    assert_eq!(playbook.len(), 1);
    assert_eq!(playbook[0].id, id);
    assert_eq!(
        history::list(&world.layout(&world.lauf).claude_dir(), &cwd, 10),
        []
    );
    // And each workspace's Claude really is pointed at its own config dir.
    let env = IsolatedEnv::build(&world.lauf, &world.layout(&world.lauf), &world.host);
    assert_eq!(
        env.get("CLAUDE_CONFIG_DIR").map(PathBuf::from),
        Some(world.layout(&world.lauf).claude_dir())
    );
}

/// SPEC §7.6: each workspace's generated CLAUDE.md and AGENTS.md hold only its own
/// knowledge (plus the shared About Mike), never `private/`, and never the other workspace.
#[test]
fn knowledge_feeds_each_workspace_only_its_own_notes() {
    use otto_core::knowledge::{self, Area, KnowledgePaths};

    let world = World::new();
    let k = KnowledgePaths::new(world.paths.root());
    for ws in [&world.playbook, &world.lauf] {
        knowledge::seed(&k, ws).unwrap();
    }
    knowledge::save_note(
        &k,
        &world.playbook.id,
        Area::Notes,
        "Roadmap.md",
        "# Roadmap\n\nPLAYBOOK-ONLY-MARKER\n",
    )
    .unwrap();
    knowledge::save_note(
        &k,
        &world.lauf.id,
        Area::Private,
        "Rates.md",
        "# Rates\n\nLAUF-PRIVATE-MARKER\n",
    )
    .unwrap();
    for ws in [&world.playbook, &world.lauf] {
        knowledge::write_instructions(&k, ws, &world.layout(ws)).unwrap();
    }

    let read = |ws: &Workspace, file: PathBuf| fs::read_to_string(file).expect(&ws.name);
    let pb_dir = k.workspace_dir(&world.playbook.id).display().to_string();
    let lauf_dir = k.workspace_dir(&world.lauf.id).display().to_string();
    for (ws, own, other) in [
        (&world.playbook, &pb_dir, &lauf_dir),
        (&world.lauf, &lauf_dir, &pb_dir),
    ] {
        let layout = world.layout(ws);
        for file in [
            layout.claude_dir().join("CLAUDE.md"),
            layout.codex_dir().join("AGENTS.md"),
        ] {
            let text = read(ws, file);
            assert!(text.contains("About Mike"), "{}", ws.name);
            assert!(
                text.contains(own.as_str()),
                "{} indexes its own notes",
                ws.name
            );
            assert!(
                !text.contains(other.as_str()),
                "{} must not see the other workspace",
                ws.name
            );
            assert!(!text.contains("/private/"), "private notes are never fed");
            assert!(!text.contains("LAUF-PRIVATE-MARKER"));
        }
    }
    let lauf_claude = read(
        &world.lauf,
        world.layout(&world.lauf).claude_dir().join("CLAUDE.md"),
    );
    assert!(!lauf_claude.contains("Roadmap") && !lauf_claude.contains("PLAYBOOK-ONLY-MARKER"));

    // Reads can't reach across, even when asked by path.
    assert!(
        knowledge::read_note(
            &k,
            &world.lauf.id,
            Area::Notes,
            "../../playbook/notes/Roadmap.md"
        )
        .is_err()
    );

    // The folders a session or Ask may open belong to that workspace (or are shared).
    let project = world.dir.path().join("stoc");
    for dirs in [
        k.session_dirs(&world.lauf.id),
        k.ask_dirs(&world.lauf.id, std::slice::from_ref(&project), true),
    ] {
        for dir in dirs {
            let d = dir.display().to_string();
            assert!(!d.starts_with(pb_dir.as_str()), "{d}");
        }
    }
    assert!(
        !k.ask_dirs(&world.lauf.id, &[], false)
            .contains(&k.area_dir(&world.lauf.id, Area::Private))
    );
    assert!(
        !k.session_dirs(&world.lauf.id)
            .contains(&k.area_dir(&world.lauf.id, Area::Private))
    );
}

/// A `claude` command that resolves into Otto's own data folder is flagged, because
/// Mike's normal terminal would then depend on Otto's files (found 2026-10-20).
#[test]
fn doctor_warns_when_claude_runs_from_otto_s_data_folder() {
    let world = World::new();
    let fakebin = world.dir.path().join("fakebin");
    let inside = world
        .layout(&world.lauf)
        .root()
        .join("xdg/data/claude/versions/9.9.9");
    fs::create_dir_all(inside.parent().unwrap()).unwrap();
    fs::write(&inside, "#!/bin/sh\necho 9.9.9\n").unwrap();
    fs::set_permissions(&inside, fs::Permissions::from_mode(0o755)).unwrap();
    std::os::unix::fs::symlink(&inside, fakebin.join("claude")).unwrap();

    let report = doctor::diagnose(&world.playbook, &world.layout(&world.playbook), &world.host);
    let check = report
        .checks
        .iter()
        .find(|c| c.id == "claudeInstall")
        .unwrap();
    assert_eq!(check.status, CheckStatus::Warn, "{}", check.detail);

    fs::remove_file(fakebin.join("claude")).unwrap();
    let outside = world.dir.path().join("claude-real");
    fs::write(&outside, "#!/bin/sh\n").unwrap();
    std::os::unix::fs::symlink(&outside, fakebin.join("claude")).unwrap();
    let report = doctor::diagnose(&world.playbook, &world.layout(&world.playbook), &world.host);
    let check = report
        .checks
        .iter()
        .find(|c| c.id == "claudeInstall")
        .unwrap();
    assert_eq!(check.status, CheckStatus::Pass);
}

/// M5a: agents report status through their own workspace's inbox. An event in Lauf's
/// inbox can't change a Playbook session, and each script writes only to its own inbox.
#[tokio::test]
async fn agent_events_only_update_their_own_workspace() {
    use otto_core::agent::AgentKind;
    use otto_core::agent_events;
    use otto_core::store::{SavedSession, SessionKind, SessionStatus, Store};

    let world = World::new();
    let (pb, lauf) = (world.layout(&world.playbook), world.layout(&world.lauf));
    let pb_inbox = agent_events::inbox_dir(&pb).display().to_string();
    let lauf_inbox = agent_events::inbox_dir(&lauf).display().to_string();
    let lauf_script = fs::read_to_string(agent_events::script_path(&lauf)).unwrap();
    assert!(lauf_script.contains(&lauf_inbox) && !lauf_script.contains(&pb_inbox));

    let store = Store::open(&world.dir.path().join("otto.db"))
        .await
        .unwrap();
    store.seed_defaults().await.unwrap();
    let pb_session = "7de656b9-96e1-4226-9fbe-c8d129813f94";
    store
        .save_session(&SavedSession {
            id: pb_session.into(),
            workspace_id: "playbook".into(),
            agent: AgentKind::Claude,
            project_path: None,
            title: "Claude Code".into(),
            kind: SessionKind::Work,
            cli_session_id: None,
            status: SessionStatus::Idle,
            last_active_at: String::new(),
            worktree_id: None,
        })
        .await
        .unwrap();

    // Something drops a Playbook session's ID into Lauf's inbox.
    fs::write(
        agent_events::inbox_dir(&lauf).join(format!("{pb_session}.claude.x.json")),
        r#"{"hook_event_name":"UserPromptSubmit"}"#,
    )
    .unwrap();
    let events = agent_events::drain(&lauf);
    assert_eq!(events.len(), 1);
    for e in &events {
        let applied = store
            .set_session_status(&world.lauf.id, &e.session_id, e.status)
            .await
            .unwrap();
        assert!(!applied, "Lauf's inbox must not reach a Playbook session");
    }
    // Nor can Lauf queue prompts into it, drop its prompts, or take them.
    let queued = store
        .enqueue(&world.playbook.id, pb_session, "playbook-only task")
        .await
        .unwrap();
    assert!(
        store
            .enqueue(&world.lauf.id, pb_session, "sneaky")
            .await
            .is_err()
    );
    assert!(store.unqueue(&world.lauf.id, &queued.id).await.is_err());
    assert_eq!(
        store.take_next(&world.lauf.id, pb_session).await.unwrap(),
        None
    );
    assert_eq!(
        store.queued(&world.playbook.id, pb_session).await.unwrap(),
        [queued]
    );

    let pb_now = store
        .session(&world.playbook.id, pb_session)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pb_now.status, SessionStatus::Idle);
    assert!(
        store
            .session(&world.lauf.id, pb_session)
            .await
            .unwrap()
            .is_none()
    );
}

/// M5b: a parallel task's worktree is still the workspace's repo. It commits as that
/// workspace, its pushes pass the same guardrails, and another workspace can neither open
/// it nor push from it.
#[test]
fn worktrees_keep_their_workspace_identity_and_guardrails() {
    use otto_core::worktree;

    let w = World::new();
    let env = w.env(&w.lauf);
    let (work, bare) = w.repo(&w.lauf, "mhlauf1");
    let made = worktree::create(&env, &w.layout(&w.lauf), &work, "Update the footer copy").unwrap();
    assert_eq!(made.branch, "otto/update-the-footer-copy");
    assert!(made.path.starts_with(worktree::root(&w.layout(&w.lauf))));
    // A second task with the same words gets its own branch.
    let again =
        worktree::create(&env, &w.layout(&w.lauf), &work, "update the footer copy").unwrap();
    assert_eq!(again.branch, "otto/update-the-footer-copy-2");

    // It opens in Lauf, and is refused in Playbook (the remote is Lauf's).
    verify_repo(
        &w.lauf,
        &w.layout(&w.lauf),
        &env,
        &made.path,
        RemotePolicy::RequireGithubRemote,
    )
    .unwrap();
    let pb_env = w.env(&w.playbook);
    assert!(
        verify_repo(
            &w.playbook,
            &w.layout(&w.playbook),
            &pb_env,
            &made.path,
            RemotePolicy::RequireGithubRemote
        )
        .is_err()
    );

    // Commits there are Lauf's, and Lauf can push the branch.
    fs::write(made.path.join("footer.txt"), "new\n").unwrap();
    assert!(worktree::is_dirty(&env, &made.path).unwrap());
    git_ok(&env, &made.path, &["add", "footer.txt"]);
    git_ok(&env, &made.path, &["commit", "-q", "-m", "footer"]);
    let author = git_ok(&env, &made.path, &["log", "-1", "--format=%ae"]);
    assert_eq!(
        String::from_utf8_lossy(&author.stdout).trim(),
        "mhlauf1@gmail.com"
    );
    git_ok(
        &env,
        &made.path,
        &["push", "-q", "origin", "HEAD:refs/heads/otto/footer"],
    );
    assert!(has_branch(&bare, &env));

    // Playbook's identity can't push from Lauf's worktree.
    let out = git(
        &pb_env,
        &made.path,
        &["push", "-q", "origin", "HEAD:refs/heads/pb"],
    );
    assert!(!out.status.success(), "{}", stderr(&out));

    // Removing refuses uncommitted work, then works once it's clean; the branch stays.
    fs::write(made.path.join("scratch.txt"), "wip\n").unwrap();
    assert!(matches!(
        worktree::remove(&env, &work, &made.path),
        Err(worktree::WorktreeError::Dirty)
    ));
    fs::remove_file(made.path.join("scratch.txt")).unwrap();
    worktree::remove(&env, &work, &made.path).unwrap();
    assert!(!made.path.exists());
    let branches = git_ok(&env, &work, &["branch", "--list", "otto/*"]);
    assert!(String::from_utf8_lossy(&branches.stdout).contains("otto/update-the-footer-copy"));
}
