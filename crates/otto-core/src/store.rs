//! SQLite app state (SPEC §5.3), via sqlx. Migrations live in `crates/otto-core/migrations`.

use std::path::Path;
use std::str::FromStr;

use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use crate::repo::VerifiedRepo;
use crate::workspace::{GithubOwner, Identity, InvalidWorkspace, Tint, Workspace, WorkspaceId};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Otto's database couldn't be opened or queried: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Otto's database couldn't be upgraded: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("A saved workspace is invalid: {0}")]
    Invalid(#[from] InvalidWorkspace),
    #[error("A saved workspace is invalid: {0}")]
    Corrupt(String),
}

#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// Open (creating if needed) and migrate the database at `path`.
    ///
    /// # Errors
    /// Fails if the file can't be opened or a migration fails.
    pub async fn open(path: &Path) -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    /// The SPEC §13 workspaces, created on first launch only.
    ///
    /// # Errors
    /// Fails on database errors.
    pub async fn seed_defaults(&self) -> Result<(), StoreError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
            .fetch_one(&self.pool)
            .await?;
        if count > 0 {
            return Ok(());
        }
        for ws in default_workspaces()? {
            self.upsert_workspace(&ws).await?;
        }
        Ok(())
    }

    /// # Errors
    /// Fails on database errors.
    pub async fn upsert_workspace(&self, ws: &Workspace) -> Result<(), StoreError> {
        let allow: Vec<&str> = ws.owner_allowlist.iter().map(GithubOwner::as_str).collect();
        let allow =
            serde_json::to_string(&allow).map_err(|e| StoreError::Corrupt(e.to_string()))?;
        sqlx::query(
            "INSERT INTO workspaces (id, name, tint, git_name, git_email, github_user, gh_owner_allowlist, position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, (SELECT COALESCE(MAX(position), -1) + 1 FROM workspaces))
             ON CONFLICT (id) DO UPDATE SET name = ?2, tint = ?3, git_name = ?4, git_email = ?5,
               github_user = ?6, gh_owner_allowlist = ?7",
        )
        .bind(ws.id.as_str())
        .bind(&ws.name)
        .bind(ws.tint.as_str())
        .bind(ws.identity.git_name())
        .bind(ws.identity.git_email())
        .bind(ws.identity.github_user().as_str())
        .bind(allow)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Workspaces in sidebar order. Every row is re-validated on the way out.
    ///
    /// # Errors
    /// Fails on database errors or if a stored row is no longer valid.
    pub async fn workspaces(&self) -> Result<Vec<Workspace>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, name, tint, git_name, git_email, github_user, gh_owner_allowlist
             FROM workspaces ORDER BY position",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|r| {
                let allow: Vec<String> = serde_json::from_str(r.get("gh_owner_allowlist"))
                    .map_err(|e| StoreError::Corrupt(e.to_string()))?;
                let tint: &str = r.get("tint");
                Ok(Workspace::new(
                    WorkspaceId::parse(r.get("id"))?,
                    r.get("name"),
                    Tint::parse(tint).ok_or_else(|| StoreError::Corrupt(format!("tint {tint}")))?,
                    Identity::new(r.get("git_name"), r.get("git_email"), r.get("github_user"))?,
                    allow
                        .iter()
                        .map(|o| GithubOwner::parse(o))
                        .collect::<Result<_, _>>()?,
                )?)
            })
            .collect()
    }

    /// The workspace a repo path is registered to, if any.
    ///
    /// # Errors
    /// Fails on database errors.
    pub async fn repo_owner(&self, path: &str) -> Result<Option<String>, StoreError> {
        Ok(
            sqlx::query_scalar("SELECT workspace_id FROM repos WHERE path = ?1")
                .bind(path)
                .fetch_optional(&self.pool)
                .await?,
        )
    }

    /// Register a verified repo. The UNIQUE path constraint enforces one workspace per repo.
    ///
    /// # Errors
    /// Fails on database errors, including the repo already belonging to a workspace.
    pub async fn register_repo(
        &self,
        workspace: &WorkspaceId,
        repo: &VerifiedRepo,
    ) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO repos (workspace_id, path, remote_url, verified_at)
             VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        )
        .bind(workspace.as_str())
        .bind(&repo.path)
        .bind(&repo.remote_url)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Append to the audit log.
    ///
    /// # Errors
    /// Fails on database errors.
    pub async fn record_event(
        &self,
        workspace: Option<&WorkspaceId>,
        kind: &str,
        detail: &serde_json::Value,
    ) -> Result<(), StoreError> {
        sqlx::query("INSERT INTO events (workspace_id, kind, detail) VALUES (?1, ?2, ?3)")
            .bind(workspace.map(WorkspaceId::as_str))
            .bind(kind)
            .bind(detail.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// # Errors
    /// Fails on database errors.
    pub async fn event_count(&self, kind: &str) -> Result<i64, StoreError> {
        Ok(
            sqlx::query_scalar("SELECT COUNT(*) FROM events WHERE kind = ?1")
                .bind(kind)
                .fetch_one(&self.pool)
                .await?,
        )
    }
}

/// SPEC §13: confirmed identities.
///
/// # Errors
/// Only if the constants below are invalid (covered by tests).
pub fn default_workspaces() -> Result<Vec<Workspace>, InvalidWorkspace> {
    Ok(vec![
        Workspace::new(
            WorkspaceId::parse("playbook")?,
            "Playbook",
            Tint::Cobalt,
            Identity::new(
                "Mike Laufersweiler",
                "michael.laufersweiler@playbookexp.com",
                "mikelauf",
            )?,
            vec![GithubOwner::parse("mikelauf")?],
        )?,
        Workspace::new(
            WorkspaceId::parse("lauf")?,
            "Lauf",
            Tint::Jade,
            Identity::new("Mike Laufersweiler", "mhlauf1@gmail.com", "mhlauf1")?,
            vec![GithubOwner::parse("mhlauf1")?],
        )?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn temp_store() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(&dir.path().join("otto.db")).await.unwrap();
        (dir, store)
    }

    #[tokio::test]
    async fn seeds_spec_workspaces_once() {
        let (_dir, store) = temp_store().await;
        store.seed_defaults().await.unwrap();
        store.seed_defaults().await.unwrap();
        let ws = store.workspaces().await.unwrap();
        assert_eq!(
            ws.iter().map(|w| w.id.as_str()).collect::<Vec<_>>(),
            ["playbook", "lauf"]
        );
        assert_eq!(ws[1].identity.git_email(), "mhlauf1@gmail.com");
        assert!(ws[0].allows_owner("mikelauf"));
        assert!(!ws[0].allows_owner("mhlauf1"));
    }

    #[tokio::test]
    async fn a_repo_belongs_to_exactly_one_workspace() {
        let (_dir, store) = temp_store().await;
        store.seed_defaults().await.unwrap();
        let repo = VerifiedRepo {
            path: "/r".into(),
            remote_url: "https://github.com/mhlauf1/r".into(),
        };
        store
            .register_repo(&WorkspaceId::parse("lauf").unwrap(), &repo)
            .await
            .unwrap();
        let again = store
            .register_repo(&WorkspaceId::parse("playbook").unwrap(), &repo)
            .await;
        assert!(again.is_err());
        assert_eq!(
            store.repo_owner("/r").await.unwrap().as_deref(),
            Some("lauf")
        );
    }

    #[tokio::test]
    async fn events_are_append_only() {
        let (_dir, store) = temp_store().await;
        store
            .record_event(None, "test", &serde_json::json!({"a": 1}))
            .await
            .unwrap();
        let tamper = sqlx::query("DELETE FROM events").execute(&store.pool).await;
        assert!(tamper.is_err());
        assert_eq!(store.event_count("test").await.unwrap(), 1);
    }
}
