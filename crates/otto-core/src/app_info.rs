use serde::Serialize;
use specta::Type;

/// Facts about the running build, shown in the hello window to prove the IPC round trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub os: String,
    pub arch: String,
}

#[derive(Debug, thiserror::Error, Serialize, Type)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum AppInfoError {
    /// The OS isn't one Otto supports. V1 is macOS only (SPEC §5.1).
    #[error("Otto only runs on macOS for now (found {0}).")]
    UnsupportedOs(String),
}

impl AppInfo {
    /// Describe this build. `version` is passed in so the caller decides
    /// whether it comes from Cargo or from the Tauri config.
    ///
    /// # Errors
    /// Returns [`AppInfoError::UnsupportedOs`] when not running on macOS.
    pub fn current(version: &str) -> Result<Self, AppInfoError> {
        Self::for_target(version, std::env::consts::OS, std::env::consts::ARCH)
    }

    fn for_target(version: &str, os: &str, arch: &str) -> Result<Self, AppInfoError> {
        if os != "macos" {
            return Err(AppInfoError::UnsupportedOs(os.to_owned()));
        }
        Ok(Self {
            name: "Otto".to_owned(),
            version: version.to_owned(),
            os: os.to_owned(),
            arch: arch.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_macos_builds() {
        let info = AppInfo::for_target("0.1.0", "macos", "aarch64").unwrap();
        assert_eq!(info.name, "Otto");
        assert_eq!(info.arch, "aarch64");
    }

    #[test]
    fn rejects_other_operating_systems() {
        let err = AppInfo::for_target("0.1.0", "linux", "x86_64").unwrap_err();
        assert_eq!(
            err.to_string(),
            "Otto only runs on macOS for now (found linux)."
        );
    }
}
