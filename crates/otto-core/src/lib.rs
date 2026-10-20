//! Otto core: everything that must be provably correct without a window.
//!
//! - [`workspace`]: identities and their on-disk layout
//! - [`env`]: the isolated environment every child process runs in
//! - [`hooks`] and [`provision`]: managed git config, hooks and helpers
//! - [`remote`] and [`repo`]: which GitHub owner a repo pushes to, and whether it may open
//! - [`doctor`]: the Identity Doctor
//! - [`store`]: SQLite app state

pub mod app_info;
pub mod doctor;
pub mod env;
pub mod hooks;
pub mod provision;
pub mod remote;
pub mod repo;
pub mod store;
pub mod workspace;

pub use app_info::{AppInfo, AppInfoError};
