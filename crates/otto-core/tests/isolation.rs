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
use otto_core::repo::{RepoRefusal, verify_repo};
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
    let ok = verify_repo(&w.lauf, &w.layout(&w.lauf), &w.env(&w.lauf), &work).unwrap();
    assert_eq!(ok.remote_url, "https://github.com/mhlauf1/project.git");
    let refused = verify_repo(
        &w.playbook,
        &w.layout(&w.playbook),
        &w.env(&w.playbook),
        &work,
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
    let refused = verify_repo(&w.lauf, &w.layout(&w.lauf), &env, &work).unwrap_err();
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
