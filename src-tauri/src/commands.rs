//! IPC commands. Keep these thin: translate the call, delegate to `otto-core`.

use otto_core::{AppInfo, AppInfoError};

#[tauri::command]
#[specta::specta]
pub fn app_info() -> Result<AppInfo, AppInfoError> {
    AppInfo::current(env!("CARGO_PKG_VERSION"))
}
