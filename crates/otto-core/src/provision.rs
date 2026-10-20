//! Creating and refreshing a workspace's isolated directory (SPEC §4.1).

use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::hooks;
use crate::workspace::{Workspace, WorkspaceLayout};

#[derive(Debug, thiserror::Error)]
#[error("Couldn't prepare {path}: {source}")]
pub struct ProvisionError {
    path: String,
    #[source]
    source: io::Error,
}

fn ctx(path: &Path) -> impl FnOnce(io::Error) -> ProvisionError + '_ {
    move |source| ProvisionError {
        path: path.display().to_string(),
        source,
    }
}

fn write_with_mode(path: &Path, contents: &str, mode: u32) -> Result<(), ProvisionError> {
    fs::write(path, contents).map_err(ctx(path))?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(ctx(path))
}

/// Create the workspace directories (owner-only) and (re)write every Otto-managed file.
/// Safe to run on every launch. It never touches CLI logins or anything else the user owns.
///
/// # Errors
/// Returns the first filesystem error, with the path that caused it.
pub fn provision(workspace: &Workspace, layout: &WorkspaceLayout) -> Result<(), ProvisionError> {
    for dir in layout.directories() {
        fs::create_dir_all(&dir).map_err(ctx(&dir))?;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).map_err(ctx(&dir))?;
    }

    write_with_mode(
        &layout.gitconfig(),
        &hooks::gitconfig(workspace, layout),
        0o600,
    )?;
    let hooks_dir = layout.hooks_dir();
    write_with_mode(
        &hooks_dir.join("pre-commit"),
        &hooks::pre_commit(workspace, layout),
        0o700,
    )?;
    write_with_mode(
        &hooks_dir.join("pre-push"),
        &hooks::pre_push(workspace, layout),
        0o700,
    )?;
    write_with_mode(&layout.askpass(), &hooks::askpass(workspace, layout), 0o700)?;

    // Shell sessions read $ZDOTDIR (inside the workspace), never ~/.zshrc. The prompt
    // names the workspace so it's obvious which identity a shell runs as (SPEC §4.5).
    let zshrc = format!(
        "# Managed by Otto. Regenerated on every launch.\nPROMPT='%F{{8}}{name}%f %1~ %# '\nexport HISTFILE=\"$ZDOTDIR/.zsh_history\"\n",
        name = workspace.name
    );
    write_with_mode(&layout.zsh_dir().join(".zshrc"), &zshrc, 0o600)?;

    // Codex: keep auth in CODEX_HOME/auth.json, never the shared keyring (ADR-005).
    // Only add the setting, so Mike's own Codex config stays intact.
    let codex_config = layout.codex_dir().join("config.toml");
    let existing = fs::read_to_string(&codex_config).unwrap_or_default();
    if !existing
        .lines()
        .any(|l| l.trim_start().starts_with("cli_auth_credentials_store"))
    {
        let line = "cli_auth_credentials_store = \"file\" # set by Otto (ADR-005)\n";
        write_with_mode(&codex_config, &format!("{line}{existing}"), 0o600)?;
    }
    Ok(())
}

/// Record the Identity Doctor's verdict where `pre-push` can read it.
///
/// # Errors
/// Fails if the status file can't be written.
pub fn write_doctor_status(
    layout: &WorkspaceLayout,
    all_green: bool,
) -> Result<(), ProvisionError> {
    let status = if all_green { "ok" } else { "blocked" };
    write_with_mode(&layout.doctor_status(), status, 0o600)
}
