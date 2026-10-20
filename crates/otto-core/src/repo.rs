//! Repos belong to exactly one workspace (SPEC §4.2). This module decides whether a
//! repo may open in a workspace, running git inside that workspace's environment.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use specta::Type;

use crate::env::IsolatedEnv;
use crate::remote::{contains_credentials, parse_github_remote};
use crate::workspace::{Workspace, WorkspaceLayout};

/// Why a repo can't open in a workspace. Each message is shown to Mike as-is.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Type)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum RepoRefusal {
    #[error("{0} isn't a git repository.")]
    NotARepo(String),
    #[error(
        "Remote “{remote}” points at github.com/{owner}, which isn't allowed in this workspace."
    )]
    ForeignOwner { remote: String, owner: String },
    #[error("Remote “{remote}” ({url}) isn't on GitHub, so Otto can't verify who owns it.")]
    UnknownHost { remote: String, url: String },
    #[error("This repo has no remotes yet. Add a GitHub remote owned by this workspace first.")]
    NoRemotes,
    #[error("This repo overrides core.hooksPath locally, which would bypass Otto's guardrails.")]
    HooksOverridden,
    #[error("This repo sets its own credential helper, which could push as a different account.")]
    CredentialHelperOverridden,
    #[error(
        "This repo's git config contains a credential ({0}). It could push as a different account. Remove it, and revoke the token on GitHub."
    )]
    SecretInConfig(String),
    #[error(
        "This repo sends its own HTTP auth header ({0}), which could push as a different account."
    )]
    AuthHeaderOverride(String),
    #[error("This repo is already registered to the {0} workspace.")]
    BelongsToOtherWorkspace(String),
    #[error("Git failed: {0}")]
    Git(String),
}

/// A repo that passed every check, ready to register.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedRepo {
    pub path: String,
    pub remote_url: String,
}

fn git(env: &IsolatedEnv, repo: &Path, args: &[&str]) -> Result<(bool, String), RepoRefusal> {
    let out = env
        .apply(Command::new("git").arg("-C").arg(repo).args(args))
        .output()
        .map_err(|e| RepoRefusal::Git(e.to_string()))?;
    Ok((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_owned(),
    ))
}

/// Check that `path` may open in `workspace`.
///
/// # Errors
/// Returns the first [`RepoRefusal`] that applies.
pub fn verify_repo(
    workspace: &Workspace,
    layout: &WorkspaceLayout,
    env: &IsolatedEnv,
    path: &Path,
) -> Result<VerifiedRepo, RepoRefusal> {
    let display = path.display().to_string();
    let (ok, top) = git(env, path, &["rev-parse", "--show-toplevel"])?;
    if !ok {
        return Err(RepoRefusal::NotARepo(display));
    }
    let root = PathBuf::from(top);

    // Repo-local settings that could undo isolation.
    let (_, hooks_path) = git(
        env,
        &root,
        &["config", "--local", "--get", "core.hooksPath"],
    )?;
    if !hooks_path.is_empty() && Path::new(&hooks_path) != layout.hooks_dir() {
        return Err(RepoRefusal::HooksOverridden);
    }
    let (_, helpers) = git(
        env,
        &root,
        &[
            "config",
            "--local",
            "--get-regexp",
            r"^credential\..*helper$",
        ],
    )?;
    if !helpers.is_empty() {
        return Err(RepoRefusal::CredentialHelperOverridden);
    }

    // Anything that could authenticate as someone else: embedded tokens or
    // user:password URLs anywhere in repo config, and custom HTTP auth headers.
    let (_, local) = git(env, &root, &["config", "--local", "--list"])?;
    for line in local.lines() {
        let key = line.split('=').next().unwrap_or("");
        if contains_credentials(line) {
            return Err(RepoRefusal::SecretInConfig(key.to_owned()));
        }
        if key.starts_with("http.") && key.ends_with("extraheader") {
            return Err(RepoRefusal::AuthHeaderOverride(key.to_owned()));
        }
    }

    // Every remote URL (fetch and push) must be a GitHub repo owned by an allowlisted owner.
    let (_, urls) = git(
        env,
        &root,
        &[
            "config",
            "--local",
            "--get-regexp",
            r"^remote\..*\.(url|pushurl)$",
        ],
    )?;
    let mut first_url = None;
    for line in urls.lines() {
        let Some((key, url)) = line.split_once(' ') else {
            continue;
        };
        let remote = key
            .trim_start_matches("remote.")
            .rsplit_once('.')
            .map_or(key, |(r, _)| r);
        match parse_github_remote(url) {
            Some(gh) if workspace.allows_owner(&gh.owner) => {
                first_url.get_or_insert_with(|| url.to_owned());
            }
            Some(gh) => {
                return Err(RepoRefusal::ForeignOwner {
                    remote: remote.to_owned(),
                    owner: gh.owner,
                });
            }
            None => {
                return Err(RepoRefusal::UnknownHost {
                    remote: remote.to_owned(),
                    url: url.to_owned(),
                });
            }
        }
    }
    let remote_url = first_url.ok_or(RepoRefusal::NoRemotes)?;
    Ok(VerifiedRepo {
        path: root.display().to_string(),
        remote_url,
    })
}
