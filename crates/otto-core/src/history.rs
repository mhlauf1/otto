//! Past Claude Code conversations, read from a workspace's own `CLAUDE_CONFIG_DIR`.
//!
//! Claude keeps every conversation as `projects/<encoded cwd>/<session id>.jsonl` inside its
//! config dir. Each workspace has its own config dir, so listing a workspace's history can
//! only ever see that workspace's conversations. Otto reads these files; it never writes them.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use specta::Type;

/// How many lines to scan for a title before giving up. Titles and the first prompt come
/// early; a long conversation can be many megabytes.
const TITLE_SCAN_LINES: usize = 400;
const MAX_TITLE: usize = 80;

/// A conversation Otto can resume.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PastConversation {
    /// Claude's session ID (a UUID), passed to `claude --resume`.
    pub id: String,
    pub title: String,
    /// Last written, in seconds since 1970.
    pub updated_secs: u32,
}

/// Claude's folder name for a working directory: every non-alphanumeric character becomes `-`.
#[must_use]
pub fn encode_cwd(cwd: &Path) -> String {
    cwd.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Where Claude, configured with `claude_dir`, keeps conversations started in `cwd`.
#[must_use]
pub fn conversations_dir(claude_dir: &Path, cwd: &Path) -> PathBuf {
    claude_dir.join("projects").join(encode_cwd(cwd))
}

/// Conversations started in `cwd`, newest first. Missing folders mean "none yet".
/// Only files named by a valid UUID are listed, so an ID is always safe to pass to `claude`.
#[must_use]
pub fn list(claude_dir: &Path, cwd: &Path, limit: usize) -> Vec<PastConversation> {
    let Ok(entries) = std::fs::read_dir(conversations_dir(claude_dir, cwd)) else {
        return Vec::new();
    };
    let mut found: Vec<(PastConversation, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != "jsonl" {
                return None;
            }
            let id = uuid::Uuid::parse_str(path.file_stem()?.to_str()?)
                .ok()?
                .to_string();
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let updated_secs = meta
                .modified()
                .ok()?
                .duration_since(UNIX_EPOCH)
                .ok()?
                .as_secs();
            let updated_secs = u32::try_from(updated_secs).unwrap_or(u32::MAX);
            Some((
                PastConversation {
                    id,
                    title: String::new(),
                    updated_secs,
                },
                path,
            ))
        })
        .collect();
    found.sort_by(|a, b| {
        b.0.updated_secs
            .cmp(&a.0.updated_secs)
            .then_with(|| a.0.id.cmp(&b.0.id))
    });
    found.truncate(limit);
    found
        .into_iter()
        .filter_map(|(mut conversation, path)| {
            // A conversation with no prompt yet (opened and closed) isn't worth resuming.
            conversation.title = title(&path)?;
            Some(conversation)
        })
        .collect()
}

/// Claude's own title if it wrote one, else the first thing Mike typed.
fn title(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut named: Option<String> = None;
    let mut first_prompt: Option<String> = None;
    for line in BufReader::new(file)
        .lines()
        .take(TITLE_SCAN_LINES)
        .map_while(Result::ok)
    {
        let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        match entry.get("type").and_then(serde_json::Value::as_str) {
            Some("custom-title") => {
                if let Some(t) = entry.get("customTitle").and_then(serde_json::Value::as_str) {
                    return Some(tidy(t));
                }
            }
            Some("ai-title") => {
                named = entry
                    .get("aiTitle")
                    .and_then(serde_json::Value::as_str)
                    .map(tidy)
                    .or(named);
            }
            Some("user") if first_prompt.is_none() => {
                first_prompt = entry
                    .pointer("/message/content")
                    .and_then(serde_json::Value::as_str)
                    .filter(|t| !t.starts_with('<'))
                    .map(tidy);
            }
            _ => {}
        }
    }
    named.or(first_prompt).filter(|t| !t.is_empty())
}

fn tidy(text: &str) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= MAX_TITLE {
        return one_line;
    }
    let cut: String = one_line.chars().take(MAX_TITLE - 1).collect();
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID_A: &str = "7de656b9-96e1-4226-9fbe-c8d129813f94";
    const ID_B: &str = "2fabc426-502e-41bd-99a6-3d6942ba4f8f";

    fn write(dir: &Path, name: &str, lines: &[serde_json::Value]) {
        std::fs::create_dir_all(dir).unwrap();
        let body: Vec<String> = lines.iter().map(ToString::to_string).collect();
        std::fs::write(dir.join(name), body.join("\n")).unwrap();
    }

    #[test]
    fn encodes_the_cwd_the_way_claude_does() {
        assert_eq!(
            encode_cwd(Path::new("/Users/mike/Desktop/lauf/client-websites/stoc")),
            "-Users-mike-Desktop-lauf-client-websites-stoc"
        );
        assert_eq!(encode_cwd(Path::new("/a/My Repo.v2")), "-a-My-Repo-v2");
    }

    #[test]
    fn lists_titles_newest_first_and_skips_junk() {
        let tmp = tempfile::tempdir().unwrap();
        let cwd = Path::new("/r/stoc");
        let dir = conversations_dir(tmp.path(), cwd);
        write(
            &dir,
            &format!("{ID_A}.jsonl"),
            &[
                serde_json::json!({"type": "user", "message": {"content": "<command-name>/clear</command-name>"}}),
                serde_json::json!({"type": "user", "message": {"content": "is this the   stoc project?"}}),
                serde_json::json!({"type": "ai-title", "aiTitle": "STOC project"}),
            ],
        );
        std::thread::sleep(std::time::Duration::from_millis(1100));
        write(
            &dir,
            &format!("{ID_B}.jsonl"),
            &[
                serde_json::json!({"type": "user", "message": {"content": "draft a reply to srushti"}}),
            ],
        );
        // Not a UUID, an empty conversation, and another folder: none are listed.
        write(
            &dir,
            "agent-1234.jsonl",
            &[serde_json::json!({"type": "user"})],
        );
        write(
            &dir,
            "11111111-1111-4111-8111-111111111111.jsonl",
            &[serde_json::json!({"type": "mode"})],
        );
        write(
            &conversations_dir(tmp.path(), Path::new("/r/other")),
            "22222222-2222-4222-8222-222222222222.jsonl",
            &[serde_json::json!({"type": "user", "message": {"content": "elsewhere"}})],
        );

        let found = list(tmp.path(), cwd, 10);
        let titles: Vec<_> = found
            .iter()
            .map(|c| (c.id.as_str(), c.title.as_str()))
            .collect();
        assert_eq!(
            titles,
            [(ID_B, "draft a reply to srushti"), (ID_A, "STOC project")]
        );
    }

    #[test]
    fn a_custom_title_wins_and_long_titles_are_shortened() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = conversations_dir(tmp.path(), Path::new("/r"));
        write(
            &dir,
            &format!("{ID_A}.jsonl"),
            &[
                serde_json::json!({"type": "ai-title", "aiTitle": "auto"}),
                serde_json::json!({"type": "custom-title", "customTitle": "October updates"}),
            ],
        );
        write(
            &dir,
            &format!("{ID_B}.jsonl"),
            &[serde_json::json!({"type": "user", "message": {"content": "x".repeat(200)}})],
        );
        let found = list(tmp.path(), Path::new("/r"), 10);
        let by_id = |id: &str| found.iter().find(|c| c.id == id).unwrap().title.clone();
        assert_eq!(by_id(ID_A), "October updates");
        assert_eq!(by_id(ID_B).chars().count(), MAX_TITLE);
        assert!(by_id(ID_B).ends_with('…'));
    }

    #[test]
    fn a_missing_folder_is_just_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(list(tmp.path(), Path::new("/nowhere"), 10), []);
    }
}
