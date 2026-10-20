//! Dev tool: `cargo run -p otto-core --example discover -- ~/Desktop`
//! Prints what project discovery would suggest. Read-only.

use otto_core::env::HostEnv;
use otto_core::projects::{Suggestion, classify, find_repos};
use otto_core::store::default_workspaces;

fn main() {
    let roots: Vec<_> = std::env::args()
        .skip(1)
        .map(std::path::PathBuf::from)
        .collect();
    let host = HostEnv::detect();
    let Ok(workspaces) = default_workspaces() else {
        return;
    };
    for repo in find_repos(&roots, 5) {
        let d = classify(&host, &repo, &workspaces);
        let verdict = match d.suggestion {
            Suggestion::Workspace { workspace } => format!("→ {workspace}"),
            Suggestion::NeedsChoice => "? needs a workspace (no remote)".into(),
            Suggestion::NotYours { owners } => format!("✗ not yours ({})", owners.join(", ")),
            Suggestion::Blocked { reason } => format!("⛔ blocked: {reason}"),
        };
        println!("{:<44} {verdict}", d.name);
    }
}
