//! Otto core: everything that must be provably correct without a window.
//!
//! In M0 this only describes the running app. M1 adds the workspace model,
//! the isolated environment builder, managed git hooks and the Identity Doctor.

pub mod app_info;

pub use app_info::{AppInfo, AppInfoError};
