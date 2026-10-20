//! Finding Mike's repos on disk and suggesting which workspace each belongs to
//! (auto-discover + confirm, chosen by Mike 2026-10-20).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use specta::Type;

use crate::env::HostEnv;
use crate::remote::{contains_credentials, parse_github_remote};
use crate::workspace::Workspace;

/// Folders never worth descending into.
const SKIP: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    ".next",
    "vendor",
    "Library",
    ".Trash",
    ".cache",
    ".pnpm-store",
    "Pods",
];

/// Every directory under `roots` (up to `max_depth` levels) that is a git repo root.
/// Doesn't descend into a repo once found.
pub fn find_repos(roots: &[PathBuf], max_depth: usize) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, max: usize, out: &mut Vec<PathBuf>) {
        if dir.join(".git").exists() {
            out.push(dir.to_path_buf());
            return;
        }
        if depth >= max {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut dirs: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                !name.starts_with('.') && !SKIP.contains(&name.as_ref())
            })
            .map(|e| e.path())
            .collect();
        dirs.sort();
        for d in dirs {
            walk(&d, depth + 1, max, out);
        }
    }
    let mut out = Vec::new();
    for root in roots {
        walk(root, 0, max_depth, &mut out);
    }
    out
}

/// What Otto suggests for a discovered repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Suggestion {
    /// Every remote is owned by this workspace's allowlist.
    Workspace { workspace: String },
    /// No GitHub remote: Mike picks the workspace, and pushes stay locked until a matching remote exists.
    NeedsChoice,
    /// The remote belongs to an owner no workspace allows (e.g. a tutorial repo).
    NotYours { owners: Vec<String> },
    /// The repo can't be opened: a credential in its config, or a non-GitHub remote.
    Blocked { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredRepo {
    pub path: String,
    pub name: String,
    pub remote_url: Option<String>,
    pub suggestion: Suggestion,
    /// The workspace this repo is already registered to, if any (filled in by the caller).
    pub registered_to: Option<String>,
}

/// Read a repo's local config with no identity at all: no global or system config,
/// no credentials, nothing from Mike's shell.
fn local_config(host: &HostEnv, repo: &Path) -> Vec<(String, String)> {
    let path = host
        .path_dirs
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(":");
    let out = Command::new("git")
        .env_clear()
        .env("PATH", path)
        .env("HOME", &host.home)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .arg("-C")
        .arg(repo)
        .args(["config", "--local", "--list"])
        .output();
    let Ok(out) = out else { return Vec::new() };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once('=').map(|(k, v)| (k.to_owned(), v.to_owned())))
        .collect()
}

/// Suggest a workspace for one repo.
pub fn classify(host: &HostEnv, repo: &Path, workspaces: &[Workspace]) -> DiscoveredRepo {
    let config = local_config(host, repo);
    let name = repo.file_name().map_or_else(
        || repo.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let urls: Vec<&str> = config
        .iter()
        .filter(|(k, _)| {
            k.starts_with("remote.") && matches!(k.rsplit_once('.'), Some((_, "url" | "pushurl")))
        })
        .map(|(_, v)| v.as_str())
        .collect();
    let remote_url = config
        .iter()
        .find(|(k, _)| k == "remote.origin.url")
        .map(|(_, v)| v.clone());
    let done = |suggestion| DiscoveredRepo {
        path: repo.display().to_string(),
        name: name.clone(),
        // Never surface a credential-bearing URL in the UI.
        remote_url: remote_url.clone().filter(|u| !contains_credentials(u)),
        suggestion,
        registered_to: None,
    };

    if let Some((key, _)) = config
        .iter()
        .find(|(k, v)| contains_credentials(&format!("{k}={v}")))
    {
        return done(Suggestion::Blocked {
            reason: format!("A credential is stored in {key}."),
        });
    }
    if urls.is_empty() {
        return done(Suggestion::NeedsChoice);
    }
    let mut owners = Vec::new();
    for url in &urls {
        match parse_github_remote(url) {
            Some(gh) => owners.push(gh.owner.to_ascii_lowercase()),
            None => {
                return done(Suggestion::Blocked {
                    reason: format!("“{url}” isn't a GitHub remote."),
                });
            }
        }
    }
    owners.dedup();
    match workspaces
        .iter()
        .find(|w| owners.iter().all(|o| w.allows_owner(o)))
    {
        Some(w) => done(Suggestion::Workspace {
            workspace: w.id.to_string(),
        }),
        None => done(Suggestion::NotYours { owners }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_repo_roots_and_skips_noise() {
        let dir = tempfile::tempdir().unwrap();
        for p in [
            "a/.git",
            "b/c/.git",
            "b/c/nested/.git",
            "node_modules/x/.git",
            "d",
        ] {
            std::fs::create_dir_all(dir.path().join(p)).unwrap();
        }
        let found = find_repos(&[dir.path().to_path_buf()], 4);
        let names: Vec<_> = found
            .iter()
            .map(|p| p.strip_prefix(dir.path()).unwrap().display().to_string())
            .collect();
        assert_eq!(names, ["a", "b/c"]);
    }
}
