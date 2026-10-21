//! Keeping Otto's data folder lean (M5a).
//!
//! Before 2026-10-20, Claude's auto-updater ran inside each workspace and downloaded its own
//! copy of Claude (~500 MB each) into the workspace's XDG data folder. Updating is now off
//! inside workspaces (`DISABLE_AUTOUPDATER`), so those copies are dead weight, unless
//! Mike's `claude` command still points into one. That one is never touched.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::env::HostEnv;
use crate::workspace::WorkspaceLayout;

/// Where a workspace's own Claude updater put its downloads.
#[must_use]
pub fn stray_claude_versions(layout: &WorkspaceLayout) -> PathBuf {
    layout.xdg_dir("data").join("claude").join("versions")
}

/// The real file the `claude` command on Otto's PATH runs, if any.
#[must_use]
pub fn claude_program(host: &HostEnv) -> Option<PathBuf> {
    host.path_dirs
        .iter()
        .map(|d| d.join("claude"))
        .find(|p| p.exists())
        .and_then(|p| p.canonicalize().ok())
}

/// Delete the workspace's stray Claude downloads unless `claude` runs from them.
/// Returns the bytes freed (0 if there was nothing to do, or it's still in use).
///
/// # Errors
/// Fails if the folder exists but can't be removed.
pub fn prune_stray_claude_versions(layout: &WorkspaceLayout, host: &HostEnv) -> io::Result<u64> {
    let dir = stray_claude_versions(layout);
    let Ok(real_dir) = dir.canonicalize() else {
        return Ok(0);
    };
    if claude_program(host).is_some_and(|program| program.starts_with(&real_dir)) {
        return Ok(0);
    }
    let freed = size(&real_dir);
    fs::remove_dir_all(&real_dir)?;
    Ok(freed)
}

fn size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    fs::read_dir(path).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .map(|e| size(&e.path()))
            .sum()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{OttoPaths, WorkspaceId};

    fn setup() -> (tempfile::TempDir, WorkspaceLayout, HostEnv, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let layout =
            OttoPaths::new(dir.path().join("Otto")).workspace(&WorkspaceId::parse("lauf").unwrap());
        let versions = stray_claude_versions(&layout);
        fs::create_dir_all(&versions).unwrap();
        fs::write(versions.join("2.1.306"), vec![0u8; 1024]).unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let mut host = HostEnv::detect();
        host.path_dirs = vec![bin.clone()];
        (dir, layout, host, bin)
    }

    #[test]
    fn frees_unused_downloads() {
        let (dir, layout, host, bin) = setup();
        let real = dir.path().join("claude-real");
        fs::write(&real, "#!/bin/sh\n").unwrap();
        std::os::unix::fs::symlink(&real, bin.join("claude")).unwrap();
        assert_eq!(prune_stray_claude_versions(&layout, &host).unwrap(), 1024);
        assert!(!stray_claude_versions(&layout).exists());
        assert_eq!(prune_stray_claude_versions(&layout, &host).unwrap(), 0);
    }

    #[test]
    fn never_removes_the_copy_claude_runs_from() {
        let (_dir, layout, host, bin) = setup();
        std::os::unix::fs::symlink(
            stray_claude_versions(&layout).join("2.1.306"),
            bin.join("claude"),
        )
        .unwrap();
        assert_eq!(prune_stray_claude_versions(&layout, &host).unwrap(), 0);
        assert!(stray_claude_versions(&layout).join("2.1.306").exists());
    }
}
