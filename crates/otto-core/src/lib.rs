//! Otto core: everything that must be provably correct without a window.
//!
//! - [`workspace`]: identities and their on-disk layout
//! - [`env`]: the isolated environment every child process runs in
//! - [`hooks`] and [`provision`]: managed git config, hooks and helpers
//! - [`remote`], [`repo`] and [`projects`]: which GitHub owner a repo pushes to, whether it may
//!   open, and finding repos on disk
//! - [`doctor`]: the Identity Doctor
//! - [`agent`] and [`pty`]: agent sessions in real terminals
//! - [`agent_events`]: what each agent is doing, reported by its own hooks
//! - [`history`]: past Claude conversations, for resuming
//! - [`housekeeping`]: keeping the data folder lean
//! - [`knowledge`]: notes, generated instruction files and project docs (SPEC §7)
//! - [`scrollback`]: Shell output kept across relaunches
//! - [`store`]: SQLite app state

pub mod agent;
pub mod agent_events;
pub mod app_info;
pub mod claude_setup;
pub mod doctor;
pub mod env;
pub mod history;
pub mod hooks;
pub mod housekeeping;
pub mod knowledge;
pub mod projects;
pub mod provision;
pub mod pty;
pub mod remote;
pub mod repo;
pub mod scrollback;
pub mod store;
pub mod workspace;

pub use app_info::{AppInfo, AppInfoError};
