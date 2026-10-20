//! The workspace model: who an identity is, and where its isolated files live.
//!
//! Every string here ends up inside git config, shell hook scripts or file paths,
//! so constructors validate strictly. An invalid value can't exist past this module.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;
use specta::Type;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InvalidWorkspace {
    #[error("Workspace IDs use lowercase letters, digits and dashes (got “{0}”).")]
    Id(String),
    #[error("“{0}” isn't a valid git author name.")]
    GitName(String),
    #[error("“{0}” isn't a valid email address.")]
    Email(String),
    #[error("“{0}” isn't a valid GitHub account or org name.")]
    GithubOwner(String),
    #[error("A workspace needs at least one allowed GitHub owner.")]
    EmptyAllowlist,
}

/// A short, path- and shell-safe identifier such as `playbook` or `lauf`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Type)]
#[serde(transparent)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    /// # Errors
    /// Fails unless `raw` matches `^[a-z][a-z0-9-]{0,31}$`.
    pub fn parse(raw: &str) -> Result<Self, InvalidWorkspace> {
        let mut chars = raw.chars();
        let valid = raw.len() <= 32
            && chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if valid {
            Ok(Self(raw.to_owned()))
        } else {
            Err(InvalidWorkspace::Id(raw.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A GitHub user or organisation login, compared case-insensitively.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(transparent)]
pub struct GithubOwner(String);

impl GithubOwner {
    /// # Errors
    /// Fails unless `raw` is 1–39 characters of letters, digits and single inner dashes.
    pub fn parse(raw: &str) -> Result<Self, InvalidWorkspace> {
        let valid = (1..=39).contains(&raw.len())
            && raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !raw.starts_with('-')
            && !raw.ends_with('-')
            && !raw.contains("--");
        if valid {
            Ok(Self(raw.to_ascii_lowercase()))
        } else {
            Err(InvalidWorkspace::GithubOwner(raw.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn matches(&self, other: &str) -> bool {
        self.0.eq_ignore_ascii_case(other)
    }
}

/// Characters we never allow in values written into git config or shell scripts.
fn has_unsafe_chars(s: &str) -> bool {
    s.chars()
        .any(|c| c.is_control() || matches!(c, '"' | '\\' | '`' | '$' | '\'' | '<' | '>'))
}

/// The git and GitHub identity a workspace commits and pushes as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    git_name: String,
    git_email: String,
    github_user: GithubOwner,
}

impl Identity {
    /// # Errors
    /// Rejects names or emails that could break out of git config or a hook script.
    pub fn new(
        git_name: &str,
        git_email: &str,
        github_user: &str,
    ) -> Result<Self, InvalidWorkspace> {
        let name = git_name.trim();
        if name.is_empty() || name.len() > 100 || has_unsafe_chars(name) {
            return Err(InvalidWorkspace::GitName(git_name.to_owned()));
        }
        let email = git_email.trim();
        let email_ok = email.len() <= 254
            && !has_unsafe_chars(email)
            && !email.chars().any(char::is_whitespace)
            && email.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty() && domain.contains('.') && !domain.contains('@')
            });
        if !email_ok {
            return Err(InvalidWorkspace::Email(git_email.to_owned()));
        }
        Ok(Self {
            git_name: name.to_owned(),
            git_email: email.to_ascii_lowercase(),
            github_user: GithubOwner::parse(github_user)?,
        })
    }

    pub fn git_name(&self) -> &str {
        &self.git_name
    }

    pub fn git_email(&self) -> &str {
        &self.git_email
    }

    pub fn github_user(&self) -> &GithubOwner {
        &self.github_user
    }
}

/// The single restrained accent each workspace gets (SPEC §6.1). Maps to `workspaceAccent` tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Tint {
    Cobalt,
    Jade,
}

impl Tint {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cobalt => "cobalt",
            Self::Jade => "jade",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "cobalt" => Some(Self::Cobalt),
            "jade" => Some(Self::Jade),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub tint: Tint,
    pub identity: Identity,
    /// GitHub users and orgs this workspace may push to (SPEC §4.2).
    pub owner_allowlist: Vec<GithubOwner>,
}

impl Workspace {
    /// # Errors
    /// Fails if the allowlist is empty: a workspace that may push nowhere is a mistake, not a policy.
    pub fn new(
        id: WorkspaceId,
        name: &str,
        tint: Tint,
        identity: Identity,
        owner_allowlist: Vec<GithubOwner>,
    ) -> Result<Self, InvalidWorkspace> {
        if owner_allowlist.is_empty() {
            return Err(InvalidWorkspace::EmptyAllowlist);
        }
        Ok(Self {
            id,
            name: name.trim().to_owned(),
            tint,
            identity,
            owner_allowlist,
        })
    }

    pub fn allows_owner(&self, owner: &str) -> bool {
        self.owner_allowlist.iter().any(|o| o.matches(owner))
    }
}

/// Otto's root on disk: `~/Library/Application Support/Otto` in the app, a temp dir in tests.
#[derive(Debug, Clone)]
pub struct OttoPaths {
    root: PathBuf,
}

impl OttoPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn database(&self) -> PathBuf {
        self.root.join("otto.db")
    }

    pub fn workspace(&self, id: &WorkspaceId) -> WorkspaceLayout {
        WorkspaceLayout {
            root: self.root.join("workspaces").join(id.as_str()),
        }
    }
}

/// Where each isolated piece of a workspace lives (SPEC §4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLayout {
    root: PathBuf,
}

impl WorkspaceLayout {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn claude_dir(&self) -> PathBuf {
        self.root.join("claude")
    }
    pub fn anthropic_dir(&self) -> PathBuf {
        self.root.join("anthropic")
    }
    pub fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }
    pub fn codex_dir(&self) -> PathBuf {
        self.root.join("codex")
    }
    pub fn gh_dir(&self) -> PathBuf {
        self.root.join("gh")
    }
    pub fn gitconfig(&self) -> PathBuf {
        self.root.join("gitconfig")
    }
    pub fn hooks_dir(&self) -> PathBuf {
        self.root.join("hooks")
    }
    pub fn ssh_dir(&self) -> PathBuf {
        self.root.join("ssh")
    }
    pub fn ssh_key(&self) -> PathBuf {
        self.ssh_dir().join("id_ed25519")
    }
    pub fn known_hosts(&self) -> PathBuf {
        self.ssh_dir().join("known_hosts")
    }
    pub fn askpass(&self) -> PathBuf {
        self.ssh_dir().join("askpass")
    }
    pub fn xdg_dir(&self, kind: &str) -> PathBuf {
        self.root.join("xdg").join(kind)
    }
    pub fn zsh_dir(&self) -> PathBuf {
        self.root.join("zsh")
    }
    /// Written by the Identity Doctor; `pre-push` refuses to run unless it says `ok`.
    pub fn doctor_status(&self) -> PathBuf {
        self.hooks_dir().join("doctor-status")
    }

    /// Every directory Otto creates for a workspace.
    pub fn directories(&self) -> Vec<PathBuf> {
        vec![
            self.root.clone(),
            self.claude_dir(),
            self.anthropic_dir(),
            self.tmp_dir(),
            self.codex_dir(),
            self.gh_dir(),
            self.hooks_dir(),
            self.ssh_dir(),
            self.xdg_dir("config"),
            self.xdg_dir("data"),
            self.xdg_dir("state"),
            self.xdg_dir("cache"),
            self.zsh_dir(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_ids_are_path_and_shell_safe() {
        assert!(WorkspaceId::parse("lauf").is_ok());
        assert!(WorkspaceId::parse("play-book2").is_ok());
        for bad in [
            "",
            "Lauf",
            "2lauf",
            "../etc",
            "a b",
            "a;rm",
            "a$b",
            &"a".repeat(33),
        ] {
            assert!(WorkspaceId::parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn github_owners_follow_github_rules() {
        assert_eq!(GithubOwner::parse("MHLauf1").unwrap().as_str(), "mhlauf1");
        for bad in ["", "-a", "a-", "a--b", "a/b", "a b", &"a".repeat(40)] {
            assert!(GithubOwner::parse(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn identities_reject_injection() {
        assert!(Identity::new("Mike Laufersweiler", "mhlauf1@gmail.com", "mhlauf1").is_ok());
        assert!(Identity::new("Mike\n[core]", "a@b.co", "x").is_err());
        assert!(Identity::new("Mike", "a@b.co\"; rm -rf", "x").is_err());
        assert!(Identity::new("Mike", "$(whoami)@b.co", "x").is_err());
        assert!(Identity::new("Mike", "not-an-email", "x").is_err());
    }

    #[test]
    fn allowlist_is_case_insensitive_and_required() {
        let id = Identity::new("M", "m@x.co", "mhlauf1").unwrap();
        let ws = Workspace::new(
            WorkspaceId::parse("lauf").unwrap(),
            "Lauf",
            Tint::Jade,
            id.clone(),
            vec![GithubOwner::parse("mhlauf1").unwrap()],
        )
        .unwrap();
        assert!(ws.allows_owner("MHLAUF1"));
        assert!(!ws.allows_owner("mikelauf"));
        let empty = Workspace::new(
            WorkspaceId::parse("x").unwrap(),
            "X",
            Tint::Jade,
            id,
            vec![],
        );
        assert_eq!(empty.unwrap_err(), InvalidWorkspace::EmptyAllowlist);
    }
}
