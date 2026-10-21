//! Parallel tasks get their own checkout of the repo: a git worktree on a new `otto/…`
//! branch, kept in the workspace's own folder (SPEC §8.2, M5b). Every git command runs in
//! the workspace's isolated environment, so the worktree commits and pushes as that
//! workspace and its guardrails (hooks, Identity Doctor) apply unchanged.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::env::IsolatedEnv;
use crate::workspace::WorkspaceLayout;

const MAX_SLUG: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum WorktreeError {
    #[error("Couldn't run git: {0}")]
    Git(String),
    #[error("Git couldn't create the worktree: {0}")]
    Create(String),
    #[error(
        "This worktree has changes that aren't committed. Commit or discard them first, then remove it."
    )]
    Dirty,
    #[error("Git couldn't remove the worktree: {0}")]
    Remove(String),
}

/// Where a workspace keeps its worktrees.
#[must_use]
pub fn root(layout: &WorkspaceLayout) -> PathBuf {
    layout.root().join("worktrees")
}

/// A short, branch-safe name from the task: "Update the footer copy!" → "update-the-footer-copy".
#[must_use]
pub fn slug(task: &str) -> String {
    let words: Vec<String> = task
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    let mut out = String::new();
    for word in words {
        let next = if out.is_empty() {
            word
        } else {
            format!("{out}-{word}")
        };
        if next.len() > MAX_SLUG {
            break;
        }
        out = next;
    }
    if out.is_empty() {
        "task".to_owned()
    } else {
        out
    }
}

fn git(env: &IsolatedEnv, dir: &Path, args: &[&str]) -> Result<(bool, String), WorktreeError> {
    let out = env
        .apply(Command::new("git").arg("-C").arg(dir).args(args))
        .output()
        .map_err(|e| WorktreeError::Git(e.to_string()))?;
    let text = if out.status.success() {
        String::from_utf8_lossy(&out.stdout)
    } else {
        String::from_utf8_lossy(&out.stderr)
    };
    Ok((out.status.success(), text.trim().to_owned()))
}

/// A new worktree for `repo`: the branch and folder it got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    pub branch: String,
    pub path: PathBuf,
}

/// Create a worktree of `repo` on a new branch `otto/<slug>` (numbered if taken), based on
/// the repo's current commit, under the workspace's worktree folder.
///
/// # Errors
/// Fails if git can't create it.
pub fn create(
    env: &IsolatedEnv,
    layout: &WorkspaceLayout,
    repo: &Path,
    task: &str,
) -> Result<Created, WorktreeError> {
    let base = slug(task);
    let project = repo.file_name().map_or_else(
        || "project".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let dir = root(layout).join(&project);
    std::fs::create_dir_all(&dir).map_err(|e| WorktreeError::Create(e.to_string()))?;
    for n in 1..100 {
        let name = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        let branch = format!("otto/{name}");
        let path = dir.join(&name);
        let (exists, _) = git(
            env,
            repo,
            &[
                "show-ref",
                "--verify",
                "--quiet",
                &format!("refs/heads/{branch}"),
            ],
        )?;
        if exists || path.exists() {
            continue;
        }
        let (ok, message) = git(
            env,
            repo,
            &[
                "worktree",
                "add",
                "-b",
                &branch,
                &path.display().to_string(),
            ],
        )?;
        if !ok {
            return Err(WorktreeError::Create(message));
        }
        return Ok(Created { branch, path });
    }
    Err(WorktreeError::Create(
        "too many worktrees with this name".into(),
    ))
}

/// Whether the worktree has uncommitted changes (including new files).
///
/// # Errors
/// Fails if git can't run there.
pub fn is_dirty(env: &IsolatedEnv, path: &Path) -> Result<bool, WorktreeError> {
    let (ok, status) = git(env, path, &["status", "--porcelain"])?;
    if !ok {
        return Err(WorktreeError::Git(status));
    }
    Ok(!status.is_empty())
}

/// Remove a clean worktree. Its branch stays, so anything committed there is kept.
///
/// # Errors
/// [`WorktreeError::Dirty`] if it has uncommitted changes; otherwise git's own error.
pub fn remove(env: &IsolatedEnv, repo: &Path, path: &Path) -> Result<(), WorktreeError> {
    if path.exists() && is_dirty(env, path)? {
        return Err(WorktreeError::Dirty);
    }
    let (ok, message) = git(
        env,
        repo,
        &["worktree", "remove", &path.display().to_string()],
    )?;
    if ok || !path.exists() {
        let _ = git(env, repo, &["worktree", "prune"]);
        return Ok(());
    }
    Err(WorktreeError::Remove(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_short_and_branch_safe() {
        assert_eq!(slug("Update the footer copy!"), "update-the-footer-copy");
        assert_eq!(slug("  ..//  "), "task");
        assert_eq!(slug("Résumé: fix ünïcode"), "r-sum-fix-n-code");
        let long = slug("one two three four five six seven eight nine ten eleven");
        assert!(long.len() <= MAX_SLUG && !long.ends_with('-'), "{long}");
    }
}
