//! The Tauri shell: window, IPC wiring and logging. Logic lives in `otto-core`.

mod commands;
mod sessions;
mod state;

use std::path::{Path, PathBuf};

use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands};

/// Where the generated TypeScript bindings live (`src/bindings.ts`).
pub fn bindings_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings.ts")
}

/// The single list of IPC commands. Adding a command here exposes it to the UI
/// and to the generated bindings at the same time.
pub fn ipc() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::app_info,
        commands::list_workspaces,
        commands::run_identity_doctor,
        commands::register_repo,
        commands::discover_projects,
        commands::list_projects,
        commands::inspect_project,
        commands::open_session,
        commands::write_session,
        commands::resize_session,
        commands::close_session,
        commands::list_sessions,
        commands::claude_setup_inventory,
        commands::import_claude_setup,
    ])
}

/// Write `src/bindings.ts`.
///
/// # Errors
/// Returns a message if the bindings can't be generated or written.
pub fn export_bindings(path: impl AsRef<Path>) -> Result<(), String> {
    ipc()
        .export(
            Typescript::default().header("// @ts-nocheck\n/* eslint-disable */"),
            path.as_ref(),
        )
        .map_err(|e| e.to_string())
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "otto=info,otto_lib=info,otto_core=info".into()),
        )
        .init();

    // In debug builds, regenerate the bindings on every launch so the UI never drifts.
    #[cfg(debug_assertions)]
    if let Err(err) = export_bindings(bindings_path()) {
        tracing::error!(%err, "failed to export TypeScript bindings");
    }

    let ipc = ipc();
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(ipc.invoke_handler())
        .setup(move |app| {
            ipc.mount_events(app);
            // Provision workspaces before the window can issue any command.
            let state = tauri::async_runtime::block_on(state::AppState::start())?;
            tracing::info!(root = %state.paths.root().display(), "Otto started");
            tauri::Manager::manage(app, state);
            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(err) = result {
        tracing::error!(%err, "Otto exited with an error");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CI guard: fails when `src/bindings.ts` doesn't match the Rust commands.
    /// Fix by running `pnpm bindings`.
    #[test]
    fn bindings_are_up_to_date() {
        let dir = std::env::temp_dir().join(format!("otto-bindings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fresh = dir.join("bindings.ts");
        export_bindings(&fresh).unwrap();
        let committed = std::fs::read_to_string(bindings_path()).unwrap_or_default();
        let generated = std::fs::read_to_string(&fresh).unwrap();
        assert!(
            committed == generated,
            "src/bindings.ts is stale. Run `pnpm bindings` and commit the result."
        );
    }
}
