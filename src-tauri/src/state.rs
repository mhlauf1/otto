//! Long-lived app state, created once at startup and shared with every command.

use otto_core::env::HostEnv;
use otto_core::knowledge::{self, KnowledgePaths};
use otto_core::provision::provision;
use otto_core::store::{Store, StoreError};
use otto_core::workspace::{OttoPaths, Workspace, WorkspaceId};

/// The data folder under `~/Library/Application Support`. The dev build (`pnpm dev`) gets its
/// own, so working on Otto can never touch the installed app's database, logins or hooks
/// (ADR-020).
pub const DATA_FOLDER: &str = if cfg!(debug_assertions) {
    "Otto Dev"
} else {
    "Otto"
};

pub struct AppState {
    pub paths: OttoPaths,
    pub knowledge: KnowledgePaths,
    pub host: HostEnv,
    pub store: Store,
    pub sessions: crate::sessions::Sessions,
}

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("Couldn't find your home folder.")]
    NoHome,
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Provision(#[from] otto_core::provision::ProvisionError),
    #[error(transparent)]
    Knowledge(#[from] knowledge::KnowledgeError),
    #[error("Couldn't create Otto's folder: {0}")]
    Io(#[from] std::io::Error),
}

impl AppState {
    /// Open the database, seed the SPEC workspaces on first launch, and (re)provision
    /// every workspace so hooks and git config are always current.
    pub async fn start() -> Result<Self, StartupError> {
        let host = HostEnv::detect();
        if host.home.as_os_str().is_empty() || host.home == std::path::Path::new("/") {
            return Err(StartupError::NoHome);
        }
        let paths = OttoPaths::new(
            host.home
                .join("Library/Application Support")
                .join(DATA_FOLDER),
        );
        std::fs::create_dir_all(paths.root())?;
        let store = Store::open(&paths.database()).await?;
        store.seed_defaults().await?;
        let knowledge = KnowledgePaths::new(paths.root());
        for ws in store.workspaces().await? {
            let layout = paths.workspace(&ws.id);
            provision(&ws, &layout)?;
            knowledge::seed(&knowledge, &ws)?;
            knowledge::write_instructions(&knowledge, &ws, &layout)?;
        }
        Ok(Self {
            paths,
            knowledge,
            host,
            store,
            sessions: crate::sessions::Sessions::default(),
        })
    }

    pub async fn workspace(&self, id: &str) -> Result<Workspace, crate::commands::CommandError> {
        let id = WorkspaceId::parse(id)
            .map_err(|e| crate::commands::CommandError::Invalid(e.to_string()))?;
        self.store
            .workspaces()
            .await
            .map_err(|e| crate::commands::CommandError::Internal(e.to_string()))?
            .into_iter()
            .find(|w| w.id == id)
            .ok_or_else(|| crate::commands::CommandError::NotFound(id.to_string()))
    }
}
