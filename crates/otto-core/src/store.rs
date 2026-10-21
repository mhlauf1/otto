//! SQLite app state (SPEC §5.3), via sqlx. Migrations live in `crates/otto-core/migrations`.

use std::path::Path;
use std::str::FromStr;

use sqlx::Row;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

use crate::repo::VerifiedRepo;

/// A project registered to a workspace.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub path: String,
    /// What the sidebar shows: Mike's chosen name, else the folder name.
    pub name: String,
    /// The folder's own name, e.g. `brady-digital`.
    pub folder_name: String,
    pub remote_url: Option<String>,
    /// The group the project sits in; `None` means ungrouped.
    pub group_id: Option<String>,
}

/// A section of projects inside one workspace, such as Lauf's "Clients" (ADR-017).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectGroup {
    pub id: String,
    pub name: String,
}

/// Starter groups per workspace. Ids match migration 0004, which adds them to
/// installs that already existed.
const DEFAULT_GROUPS: &[(&str, &[&str])] = &[("lauf", &["Clients", "Studio", "Personal"])];

const MAX_GROUP_NAME: usize = 40;
const MAX_PROJECT_NAME: usize = 60;

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
    /// A request Otto won't carry out. The message says why, in plain words.
    #[error("{0}")]
    Rejected(String),
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
        for (workspace, names) in DEFAULT_GROUPS {
            for (position, name) in names.iter().enumerate() {
                sqlx::query(
                    "INSERT OR IGNORE INTO project_groups (id, workspace_id, name, position)
                     VALUES (?1, ?2, ?3, ?4)",
                )
                .bind(format!("{workspace}-{}", name.to_lowercase()))
                .bind(workspace)
                .bind(name)
                .bind(i64::try_from(position).unwrap_or(i64::MAX))
                .execute(&self.pool)
                .await?;
            }
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

    /// Projects registered to a workspace, by name.
    ///
    /// # Errors
    /// Fails on database errors.
    pub async fn projects(&self, workspace: &WorkspaceId) -> Result<Vec<Project>, StoreError> {
        let rows = sqlx::query(
            "SELECT path, remote_url, group_id, display_name FROM repos WHERE workspace_id = ?1",
        )
        .bind(workspace.as_str())
        .fetch_all(&self.pool)
        .await?;
        let mut projects: Vec<Project> = rows
            .iter()
            .map(|r| {
                let path: String = r.get("path");
                let folder_name = Path::new(&path)
                    .file_name()
                    .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
                let display: Option<String> = r.get("display_name");
                Project {
                    path,
                    name: display.unwrap_or_else(|| folder_name.clone()),
                    folder_name,
                    remote_url: r.get("remote_url"),
                    group_id: r.get("group_id"),
                }
            })
            .collect();
        projects.sort_by_key(|p| p.name.to_lowercase());
        Ok(projects)
    }

    /// A workspace's project groups, in sidebar order.
    ///
    /// # Errors
    /// Fails on database errors.
    pub async fn project_groups(
        &self,
        workspace: &WorkspaceId,
    ) -> Result<Vec<ProjectGroup>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, name FROM project_groups WHERE workspace_id = ?1 ORDER BY position, name",
        )
        .bind(workspace.as_str())
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| ProjectGroup {
                id: r.get("id"),
                name: r.get("name"),
            })
            .collect())
    }

    /// Add a group at the end of a workspace's list.
    ///
    /// # Errors
    /// Rejects an empty, overlong or duplicate name; fails on database errors.
    pub async fn create_project_group(
        &self,
        workspace: &WorkspaceId,
        name: &str,
    ) -> Result<ProjectGroup, StoreError> {
        let name = group_name(name)?;
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO project_groups (id, workspace_id, name, position)
             VALUES (?1, ?2, ?3,
               (SELECT COALESCE(MAX(position), -1) + 1 FROM project_groups WHERE workspace_id = ?2))",
        )
        .bind(&id)
        .bind(workspace.as_str())
        .bind(&name)
        .execute(&self.pool)
        .await
        .map_err(|e| duplicate_name(e, &name))?;
        Ok(ProjectGroup { id, name })
    }

    /// # Errors
    /// Rejects a bad name or a group from another workspace; fails on database errors.
    pub async fn rename_project_group(
        &self,
        workspace: &WorkspaceId,
        id: &str,
        name: &str,
    ) -> Result<(), StoreError> {
        let name = group_name(name)?;
        let done =
            sqlx::query("UPDATE project_groups SET name = ?1 WHERE id = ?2 AND workspace_id = ?3")
                .bind(&name)
                .bind(id)
                .bind(workspace.as_str())
                .execute(&self.pool)
                .await
                .map_err(|e| duplicate_name(e, &name))?;
        if done.rows_affected() == 0 {
            return Err(not_this_workspace("group"));
        }
        Ok(())
    }

    /// Delete a group. Its projects stay in the workspace, ungrouped.
    ///
    /// # Errors
    /// Rejects a group from another workspace; fails on database errors.
    pub async fn delete_project_group(
        &self,
        workspace: &WorkspaceId,
        id: &str,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE repos SET group_id = NULL WHERE group_id = ?1 AND workspace_id = ?2")
            .bind(id)
            .bind(workspace.as_str())
            .execute(&mut *tx)
            .await?;
        let done = sqlx::query("DELETE FROM project_groups WHERE id = ?1 AND workspace_id = ?2")
            .bind(id)
            .bind(workspace.as_str())
            .execute(&mut *tx)
            .await?;
        if done.rows_affected() == 0 {
            return Err(not_this_workspace("group"));
        }
        tx.commit().await?;
        Ok(())
    }

    /// Put a workspace's groups in the given order. Every one of its groups must be listed.
    ///
    /// # Errors
    /// Rejects a list that doesn't match the workspace's groups; fails on database errors.
    pub async fn reorder_project_groups(
        &self,
        workspace: &WorkspaceId,
        ids: &[String],
    ) -> Result<(), StoreError> {
        let mut current: Vec<String> = self
            .project_groups(workspace)
            .await?
            .into_iter()
            .map(|g| g.id)
            .collect();
        let mut wanted = ids.to_vec();
        current.sort();
        wanted.sort();
        if current != wanted {
            return Err(StoreError::Rejected(
                "The group list changed while you were moving it. Try again.".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        for (position, id) in ids.iter().enumerate() {
            sqlx::query(
                "UPDATE project_groups SET position = ?1 WHERE id = ?2 AND workspace_id = ?3",
            )
            .bind(i64::try_from(position).unwrap_or(i64::MAX))
            .bind(id)
            .bind(workspace.as_str())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Put a project in one of its own workspace's groups, or none. The database's
    /// composite foreign key refuses a group that belongs to another workspace.
    ///
    /// # Errors
    /// Rejects a project or group from another workspace; fails on database errors.
    pub async fn move_project(
        &self,
        workspace: &WorkspaceId,
        path: &str,
        group: Option<&str>,
    ) -> Result<(), StoreError> {
        let done =
            sqlx::query("UPDATE repos SET group_id = ?1 WHERE path = ?2 AND workspace_id = ?3")
                .bind(group)
                .bind(path)
                .bind(workspace.as_str())
                .execute(&self.pool)
                .await
                .map_err(|e| match e.as_database_error() {
                    Some(d) if d.is_foreign_key_violation() => not_this_workspace("group"),
                    _ => StoreError::Database(e),
                })?;
        if done.rows_affected() == 0 {
            return Err(not_this_workspace("project"));
        }
        Ok(())
    }

    /// Give a project a name of Mike's choosing. An empty name goes back to the folder name.
    ///
    /// # Errors
    /// Rejects an overlong name or a project from another workspace; fails on database errors.
    pub async fn rename_project(
        &self,
        workspace: &WorkspaceId,
        path: &str,
        name: &str,
    ) -> Result<(), StoreError> {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.chars().count() > MAX_PROJECT_NAME {
            return Err(StoreError::Rejected(format!(
                "Keep project names to {MAX_PROJECT_NAME} characters or fewer."
            )));
        }
        let done =
            sqlx::query("UPDATE repos SET display_name = ?1 WHERE path = ?2 AND workspace_id = ?3")
                .bind((!name.is_empty()).then_some(&name))
                .bind(path)
                .bind(workspace.as_str())
                .execute(&self.pool)
                .await?;
        if done.rows_affected() == 0 {
            return Err(not_this_workspace("project"));
        }
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

fn group_name(name: &str) -> Result<String, StoreError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(StoreError::Rejected("A group needs a name.".into()));
    }
    if name.chars().count() > MAX_GROUP_NAME {
        return Err(StoreError::Rejected(format!(
            "Keep group names to {MAX_GROUP_NAME} characters or fewer."
        )));
    }
    Ok(name.to_owned())
}

fn duplicate_name(e: sqlx::Error, name: &str) -> StoreError {
    match e.as_database_error() {
        Some(d) if d.is_unique_violation() => {
            StoreError::Rejected(format!("There's already a group called “{name}”."))
        }
        _ => StoreError::Database(e),
    }
}

fn not_this_workspace(what: &str) -> StoreError {
    StoreError::Rejected(format!("That {what} isn't part of this workspace."))
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
            vec![
                GithubOwner::parse("mikelauf")?,
                GithubOwner::parse("lulafit")?,
            ],
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
        assert!(
            ws[0].allows_owner("lulafit"),
            "Playbook org (Mike, 2026-10-20)"
        );
        assert!(!ws[0].allows_owner("mhlauf1"));
    }

    #[tokio::test]
    async fn migrations_bring_existing_installs_to_the_current_defaults() {
        // An install seeded before migration 0002 must end up with the same allowlist
        // as a fresh one. Simulate it: seed the old value, then re-run migrations.
        let (dir, store) = temp_store().await;
        store.seed_defaults().await.unwrap();
        sqlx::query(
            "UPDATE workspaces SET gh_owner_allowlist = '[\"mikelauf\"]' WHERE id = 'playbook'",
        )
        .execute(&store.pool)
        .await
        .unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 2")
            .execute(&store.pool)
            .await
            .unwrap();
        let store = Store::open(&dir.path().join("otto.db")).await.unwrap();
        let fresh = default_workspaces().unwrap();
        let migrated = store.workspaces().await.unwrap();
        assert_eq!(migrated[0].owner_allowlist, fresh[0].owner_allowlist);
    }

    #[tokio::test]
    async fn a_repo_belongs_to_exactly_one_workspace() {
        let (_dir, store) = temp_store().await;
        store.seed_defaults().await.unwrap();
        let repo = VerifiedRepo {
            path: "/r".into(),
            remote_url: Some("https://github.com/mhlauf1/r".into()),
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

    fn ws(id: &str) -> WorkspaceId {
        WorkspaceId::parse(id).unwrap()
    }

    async fn store_with_repo(path: &str, workspace: &str) -> (tempfile::TempDir, Store) {
        let (dir, store) = temp_store().await;
        store.seed_defaults().await.unwrap();
        let repo = VerifiedRepo {
            path: path.into(),
            remote_url: None,
        };
        store.register_repo(&ws(workspace), &repo).await.unwrap();
        (dir, store)
    }

    #[tokio::test]
    async fn lauf_starts_with_clients_studio_and_personal() {
        let (_dir, store) = temp_store().await;
        store.seed_defaults().await.unwrap();
        let names: Vec<_> = store
            .project_groups(&ws("lauf"))
            .await
            .unwrap()
            .into_iter()
            .map(|g| g.name)
            .collect();
        assert_eq!(names, ["Clients", "Studio", "Personal"]);
        assert_eq!(store.project_groups(&ws("playbook")).await.unwrap(), []);
    }

    #[tokio::test]
    async fn migrations_0004_and_0005_keep_projects_and_add_lauf_groups_to_existing_installs() {
        // An install from before 0004: workspaces and a project, no groups table yet.
        let (dir, store) = store_with_repo("/stoc", "lauf").await;
        for sql in [
            "DELETE FROM project_groups",
            "DELETE FROM _sqlx_migrations WHERE version >= 4",
            "CREATE TABLE repos_old AS SELECT id, workspace_id, path, remote_url, verified_at FROM repos",
            "DROP TABLE repos",
            "DROP TABLE project_groups",
            "ALTER TABLE repos_old RENAME TO repos",
        ] {
            sqlx::query(sql).execute(&store.pool).await.unwrap();
        }
        let store = Store::open(&dir.path().join("otto.db")).await.unwrap();
        assert_eq!(store.project_groups(&ws("lauf")).await.unwrap().len(), 3);
        let projects = store.projects(&ws("lauf")).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].group_id, None);
    }

    #[tokio::test]
    async fn group_names_are_trimmed_required_and_unique_per_workspace() {
        let (_dir, store) = temp_store().await;
        store.seed_defaults().await.unwrap();
        let g = store
            .create_project_group(&ws("playbook"), "  Core  ")
            .await
            .unwrap();
        assert_eq!(g.name, "Core");
        assert!(matches!(
            store.create_project_group(&ws("playbook"), "Core").await,
            Err(StoreError::Rejected(_))
        ));
        assert!(matches!(
            store.create_project_group(&ws("playbook"), "   ").await,
            Err(StoreError::Rejected(_))
        ));
        // The same name in another workspace is a different group.
        store
            .create_project_group(&ws("lauf"), "Core")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn deleting_a_group_keeps_its_projects() {
        let (_dir, store) = store_with_repo("/stoc", "lauf").await;
        store
            .move_project(&ws("lauf"), "/stoc", Some("lauf-clients"))
            .await
            .unwrap();
        assert_eq!(
            store.projects(&ws("lauf")).await.unwrap()[0]
                .group_id
                .as_deref(),
            Some("lauf-clients")
        );
        store
            .delete_project_group(&ws("lauf"), "lauf-clients")
            .await
            .unwrap();
        let projects = store.projects(&ws("lauf")).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].group_id, None);
    }

    #[tokio::test]
    async fn projects_take_a_chosen_name_and_can_go_back_to_the_folder_name() {
        let (_dir, store) = store_with_repo("/dev/brady-digital", "lauf").await;
        let get = || async { store.projects(&ws("lauf")).await.unwrap().remove(0) };
        assert_eq!(get().await.name, "brady-digital");
        store
            .rename_project(
                &ws("lauf"),
                "/dev/brady-digital",
                "  Brady   Digital Consulting ",
            )
            .await
            .unwrap();
        let p = get().await;
        assert_eq!(
            (p.name.as_str(), p.folder_name.as_str()),
            ("Brady Digital Consulting", "brady-digital")
        );
        store
            .rename_project(&ws("lauf"), "/dev/brady-digital", "")
            .await
            .unwrap();
        assert_eq!(get().await.name, "brady-digital");
        assert!(matches!(
            store
                .rename_project(&ws("lauf"), "/dev/brady-digital", &"x".repeat(61))
                .await,
            Err(StoreError::Rejected(_))
        ));
    }

    #[tokio::test]
    async fn groups_reorder_only_with_the_full_list() {
        let (_dir, store) = temp_store().await;
        store.seed_defaults().await.unwrap();
        let order = ["lauf-personal", "lauf-clients", "lauf-studio"].map(String::from);
        store
            .reorder_project_groups(&ws("lauf"), &order)
            .await
            .unwrap();
        let ids: Vec<_> = store
            .project_groups(&ws("lauf"))
            .await
            .unwrap()
            .into_iter()
            .map(|g| g.id)
            .collect();
        assert_eq!(ids, order);
        assert!(
            store
                .reorder_project_groups(&ws("lauf"), &order[..2])
                .await
                .is_err()
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
