//! Copying parts of Mike's normal Claude Code setup (`~/.claude`, `~/.claude.json`) into
//! a workspace's isolated `CLAUDE_CONFIG_DIR`, chosen item by item (Mike, 2026-10-20).
//!
//! Copies only: the source is never modified, and nothing is shared between workspaces.
//! Never copied: credentials, history, sessions, projects, telemetry, or account-synced
//! content (each workspace's own login syncs that).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Map, Value};
use specta::Type;

use crate::workspace::WorkspaceLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SetupKind {
    Instructions,
    Skill,
    Agent,
    Command,
    Settings,
    Plugin,
    McpServer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SetupItem {
    /// Stable id, e.g. `skill:audit-website.md` or `mcp:Sanity`.
    pub id: String,
    pub kind: SetupKind,
    pub name: String,
    pub detail: String,
    /// Shown before importing, e.g. when an MCP server stores an auth header.
    pub warning: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[error("Couldn't read your Claude setup at {path}: {source}")]
    Read { path: String, source: io::Error },
    #[error("Couldn't write to the workspace at {path}: {source}")]
    Write { path: String, source: io::Error },
    #[error("{path} isn't valid JSON: {message}")]
    Json { path: String, message: String },
    #[error("Unknown setup item “{0}”.")]
    Unknown(String),
}

/// Where Mike's normal Claude setup lives.
#[derive(Debug, Clone)]
pub struct ClaudeSource {
    /// `~/.claude`
    pub dir: PathBuf,
    /// `~/.claude.json` (user-scope MCP servers live here)
    pub json: PathBuf,
}

impl ClaudeSource {
    pub fn for_home(home: &Path) -> Self {
        Self {
            dir: home.join(".claude"),
            json: home.join(".claude.json"),
        }
    }
}

/// Entries inside skills/, agents/ and commands/ that are never offered.
const EXCLUDED_ENTRIES: &[&str] = &["synced"];

fn read_json(path: &Path) -> Result<Value, SetupError> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).map_err(|e| SetupError::Json {
            path: path.display().to_string(),
            message: e.to_string(),
        }),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Value::Object(Map::new())),
        Err(source) => Err(SetupError::Read {
            path: path.display().to_string(),
            source,
        }),
    }
}

fn write_json(path: &Path, value: &Value) -> Result<(), SetupError> {
    let err = |source| SetupError::Write {
        path: path.display().to_string(),
        source,
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(err)?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| SetupError::Json {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    fs::write(path, text).map_err(err)
}

fn entries(dir: &Path) -> Vec<String> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = read
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.') && !EXCLUDED_ENTRIES.contains(&n.as_str()))
        .collect();
    names.sort();
    names
}

/// Everything in Mike's setup that can be imported.
///
/// # Errors
/// Fails if a config file exists but can't be parsed.
pub fn inventory(source: &ClaudeSource) -> Result<Vec<SetupItem>, SetupError> {
    let mut items = Vec::new();
    let item = |id: String, kind, name: &str, detail: &str, warning: Option<String>| SetupItem {
        id,
        kind,
        name: name.to_owned(),
        detail: detail.to_owned(),
        warning,
    };

    if source.dir.join("CLAUDE.md").is_file() {
        items.push(item(
            "instructions".into(),
            SetupKind::Instructions,
            "CLAUDE.md",
            "Your personal instructions",
            None,
        ));
    }
    for (folder, kind, detail) in [
        ("skills", SetupKind::Skill, "Skill"),
        ("agents", SetupKind::Agent, "Subagent"),
        ("commands", SetupKind::Command, "Slash command"),
    ] {
        for name in entries(&source.dir.join(folder)) {
            items.push(item(
                format!("{folder}:{name}"),
                kind,
                name.trim_end_matches(".md"),
                detail,
                None,
            ));
        }
    }

    let settings = read_json(&source.dir.join("settings.json"))?;
    if let Some(obj) = settings.as_object().filter(|o| !o.is_empty()) {
        let keys: Vec<&str> = obj
            .keys()
            .map(String::as_str)
            .filter(|k| !matches!(*k, "enabledPlugins" | "extraKnownMarketplaces"))
            .collect();
        let warning = obj
            .get("env")
            .is_some_and(|e| e.as_object().is_some_and(|m| !m.is_empty()))
            .then(|| "Includes environment variables, which may hold secrets.".to_owned());
        items.push(item(
            "settings".into(),
            SetupKind::Settings,
            "settings.json",
            &keys.join(", "),
            warning,
        ));
    }

    let installed = read_json(&source.dir.join("plugins/installed_plugins.json"))?;
    if let Some(plugins) = installed.get("plugins").and_then(Value::as_object) {
        for key in plugins.keys() {
            items.push(item(
                format!("plugin:{key}"),
                SetupKind::Plugin,
                key.split('@').next().unwrap_or(key),
                key,
                None,
            ));
        }
    }

    let user = read_json(&source.json)?;
    if let Some(servers) = user.get("mcpServers").and_then(Value::as_object) {
        for (name, cfg) in servers {
            let kind = cfg.get("type").and_then(Value::as_str).unwrap_or("stdio");
            let has = |k: &str| {
                cfg.get(k)
                    .and_then(Value::as_object)
                    .is_some_and(|m| !m.is_empty())
            };
            let warning = (has("headers") || has("env")).then(|| {
                "Stores credentials in its config. They'll be copied into this workspace."
                    .to_owned()
            });
            items.push(item(
                format!("mcp:{name}"),
                SetupKind::McpServer,
                name,
                &format!("MCP server ({kind})"),
                warning,
            ));
        }
    }
    Ok(items)
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), SetupError> {
    let werr = |source| SetupError::Write {
        path: to.display().to_string(),
        source,
    };
    let meta = fs::symlink_metadata(from).map_err(|source| SetupError::Read {
        path: from.display().to_string(),
        source,
    })?;
    if meta.file_type().is_symlink() {
        let target = fs::read_link(from).map_err(|source| SetupError::Read {
            path: from.display().to_string(),
            source,
        })?;
        let _ = fs::remove_file(to);
        return std::os::unix::fs::symlink(target, to).map_err(werr);
    }
    if meta.is_dir() {
        fs::create_dir_all(to).map_err(werr)?;
        let read = fs::read_dir(from).map_err(|source| SetupError::Read {
            path: from.display().to_string(),
            source,
        })?;
        for entry in read.filter_map(Result::ok) {
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(werr)?;
    }
    fs::copy(from, to).map(|_| ()).map_err(werr)
}

/// Replace `from` with `to` in every string inside `value` (absolute install paths).
fn rewrite_paths(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::String(s) if s.contains(from) => *s = s.replace(from, to),
        Value::Array(items) => items.iter_mut().for_each(|v| rewrite_paths(v, from, to)),
        Value::Object(map) => map.values_mut().for_each(|v| rewrite_paths(v, from, to)),
        _ => {}
    }
}

fn object(value: &mut Value) -> &mut Map<String, Value> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    match value {
        Value::Object(map) => map,
        // Unreachable: replaced with an object just above.
        _ => unreachable!("value was just made an object"),
    }
}

fn merge_key(target: &mut Value, section: &str, key: &str, entry: Value) {
    let section = object(target)
        .entry(section)
        .or_insert_with(|| Value::Object(Map::new()));
    object(section).insert(key.to_owned(), entry);
}

/// Copy the chosen items into `layout`'s Claude config dir. Returns the ids imported.
///
/// # Errors
/// Stops at the first item that can't be read or written; earlier items stay imported.
pub fn import(
    source: &ClaudeSource,
    layout: &WorkspaceLayout,
    ids: &[String],
) -> Result<Vec<String>, SetupError> {
    let dest = layout.claude_dir();
    let src_prefix = source.dir.display().to_string();
    let dest_prefix = dest.display().to_string();
    let settings_path = dest.join("settings.json");
    let mut done = Vec::new();

    for id in ids {
        let (kind, name) = id.split_once(':').unwrap_or((id.as_str(), ""));
        // Names come from the UI; never let one escape the folder it names.
        if name.contains('/') || name.contains("..") || EXCLUDED_ENTRIES.contains(&name) {
            return Err(SetupError::Unknown(id.clone()));
        }
        match kind {
            "instructions" => copy_tree(&source.dir.join("CLAUDE.md"), &dest.join("CLAUDE.md"))?,
            "skills" | "agents" | "commands" => {
                let from = source.dir.join(kind).join(name);
                if !from.exists() {
                    return Err(SetupError::Unknown(id.clone()));
                }
                copy_tree(&from, &dest.join(kind).join(name))?;
            }
            "settings" => {
                let mut theirs = read_json(&source.dir.join("settings.json"))?;
                let theirs = object(&mut theirs);
                theirs.remove("enabledPlugins");
                theirs.remove("extraKnownMarketplaces");
                let mut ours = read_json(&settings_path)?;
                let ours_map = object(&mut ours);
                for (k, v) in theirs.iter() {
                    ours_map.insert(k.clone(), v.clone());
                }
                write_json(&settings_path, &ours)?;
            }
            "plugin" => {
                let (_, market) = name
                    .split_once('@')
                    .ok_or_else(|| SetupError::Unknown(id.clone()))?;
                let plugins_src = source.dir.join("plugins");
                let plugins_dest = dest.join("plugins");

                let installed = read_json(&plugins_src.join("installed_plugins.json"))?;
                let mut entry = installed["plugins"]
                    .get(name)
                    .cloned()
                    .ok_or_else(|| SetupError::Unknown(id.clone()))?;
                let markets = read_json(&plugins_src.join("known_marketplaces.json"))?;
                let mut market_entry = markets.get(market).cloned().unwrap_or(Value::Null);

                // Plugin code and its marketplace checkout.
                for record in entry.as_array().into_iter().flatten() {
                    // Only copy code that lives inside ~/.claude; never follow paths elsewhere.
                    let rel = record
                        .get("installPath")
                        .and_then(Value::as_str)
                        .and_then(|p| Path::new(p).strip_prefix(&source.dir).ok().map(|r| (p, r)));
                    if let Some((path, rel)) = rel {
                        copy_tree(Path::new(path), &dest.join(rel))?;
                    }
                }
                let market_dir = plugins_src.join("marketplaces").join(market);
                if market_dir.is_dir() {
                    copy_tree(&market_dir, &plugins_dest.join("marketplaces").join(market))?;
                }

                // Registries, with absolute paths pointed at the workspace copy.
                rewrite_paths(&mut entry, &src_prefix, &dest_prefix);
                rewrite_paths(&mut market_entry, &src_prefix, &dest_prefix);
                let installed_path = plugins_dest.join("installed_plugins.json");
                let mut ours = read_json(&installed_path)?;
                object(&mut ours)
                    .entry("version")
                    .or_insert(installed.get("version").cloned().unwrap_or(Value::from(2)));
                merge_key(&mut ours, "plugins", name, entry);
                write_json(&installed_path, &ours)?;
                if !market_entry.is_null() {
                    let known_path = plugins_dest.join("known_marketplaces.json");
                    let mut known = read_json(&known_path)?;
                    object(&mut known).insert(market.to_owned(), market_entry);
                    write_json(&known_path, &known)?;
                }

                // Enable it, and remember where its marketplace comes from.
                let theirs = read_json(&source.dir.join("settings.json"))?;
                let mut settings = read_json(&settings_path)?;
                merge_key(&mut settings, "enabledPlugins", name, Value::Bool(true));
                if let Some(m) = theirs
                    .get("extraKnownMarketplaces")
                    .and_then(|m| m.get(market))
                {
                    merge_key(&mut settings, "extraKnownMarketplaces", market, m.clone());
                }
                write_json(&settings_path, &settings)?;
            }
            "mcp" => {
                let user = read_json(&source.json)?;
                let server = user["mcpServers"]
                    .get(name)
                    .cloned()
                    .ok_or_else(|| SetupError::Unknown(id.clone()))?;
                let path = dest.join(".claude.json");
                let mut ours = read_json(&path)?;
                merge_key(&mut ours, "mcpServers", name, server);
                write_json(&path, &ours)?;
            }
            _ => return Err(SetupError::Unknown(id.clone())),
        }
        done.push(id.clone());
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{OttoPaths, WorkspaceId};

    fn fixture() -> (
        tempfile::TempDir,
        ClaudeSource,
        WorkspaceLayout,
        WorkspaceLayout,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let src = ClaudeSource::for_home(&home);
        let w = |p: &str, s: &str| {
            let path = src.dir.join(p);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, s).unwrap();
        };
        w("skills/audit.md", "audit");
        w("skills/synced/abc/skill.md", "account-synced");
        w("agents/prospector.md", "agent");
        w(".credentials.json", r#"{"secret":"token"}"#);
        w("history.jsonl", "private history");
        w(
            "settings.json",
            r#"{"model":"opus","theme":"dark","enabledPlugins":{"ux@mk":true},"extraKnownMarketplaces":{"mk":{"source":{"source":"github","repo":"a/b"}}}}"#,
        );
        let cache = src.dir.join("plugins/cache/mk/ux/1.0.0");
        w("plugins/cache/mk/ux/1.0.0/SKILL.md", "plugin code");
        w("plugins/marketplaces/mk/README.md", "market");
        w(
            "plugins/installed_plugins.json",
            &format!(
                r#"{{"version":2,"plugins":{{"ux@mk":[{{"scope":"user","installPath":"{}"}}]}}}}"#,
                cache.display()
            ),
        );
        w(
            "plugins/known_marketplaces.json",
            &format!(
                r#"{{"mk":{{"installLocation":"{}"}}}}"#,
                src.dir.join("plugins/marketplaces/mk").display()
            ),
        );
        fs::write(
            &src.json,
            r#"{"mcpServers":{"Sanity":{"type":"http","url":"https://x","headers":{"Authorization":"Bearer s"}},"mobbin":{"type":"http","url":"https://m"}},"oauthAccount":{"email":"x"}}"#,
        )
        .unwrap();
        let paths = OttoPaths::new(dir.path().join("otto"));
        let a = paths.workspace(&WorkspaceId::parse("playbook").unwrap());
        let b = paths.workspace(&WorkspaceId::parse("lauf").unwrap());
        (dir, src, a, b)
    }

    #[test]
    fn inventory_offers_setup_and_never_secrets_history_or_synced_items() {
        let (_d, src, _, _) = fixture();
        let ids: Vec<String> = inventory(&src).unwrap().into_iter().map(|i| i.id).collect();
        assert!(ids.contains(&"skills:audit.md".to_owned()));
        assert!(ids.contains(&"agents:prospector.md".to_owned()));
        assert!(ids.contains(&"plugin:ux@mk".to_owned()));
        assert!(ids.contains(&"mcp:Sanity".to_owned()));
        assert!(ids.contains(&"settings".to_owned()));
        for id in &ids {
            assert!(
                !id.contains("synced") && !id.contains("credentials") && !id.contains("history"),
                "{id}"
            );
        }
    }

    #[test]
    fn mcp_servers_with_auth_headers_carry_a_warning() {
        let (_d, src, _, _) = fixture();
        let items = inventory(&src).unwrap();
        let sanity = items.iter().find(|i| i.id == "mcp:Sanity").unwrap();
        assert!(sanity.warning.is_some());
        let mobbin = items.iter().find(|i| i.id == "mcp:mobbin").unwrap();
        assert!(mobbin.warning.is_none());
    }

    #[test]
    fn imports_into_one_workspace_only_and_rewrites_plugin_paths() {
        let (_d, src, playbook, lauf) = fixture();
        let ids: Vec<String> = ["skills:audit.md", "plugin:ux@mk", "mcp:mobbin", "settings"]
            .map(String::from)
            .to_vec();
        import(&src, &lauf, &ids).unwrap();

        let dest = lauf.claude_dir();
        assert_eq!(
            fs::read_to_string(dest.join("skills/audit.md")).unwrap(),
            "audit"
        );
        assert!(dest.join("plugins/cache/mk/ux/1.0.0/SKILL.md").exists());
        let installed = fs::read_to_string(dest.join("plugins/installed_plugins.json")).unwrap();
        assert!(installed.contains(&dest.display().to_string()));
        assert!(
            !installed.contains(&src.dir.display().to_string()),
            "still points at ~/.claude"
        );
        let settings: Value =
            serde_json::from_str(&fs::read_to_string(dest.join("settings.json")).unwrap()).unwrap();
        assert_eq!(settings["model"], "opus");
        assert_eq!(settings["enabledPlugins"]["ux@mk"], true);
        let mcp: Value =
            serde_json::from_str(&fs::read_to_string(dest.join(".claude.json")).unwrap()).unwrap();
        assert!(mcp["mcpServers"]["mobbin"].is_object());
        assert!(
            mcp.get("oauthAccount").is_none(),
            "account data must never be copied"
        );

        // Nothing leaked into the other workspace, and no secrets were copied anywhere.
        assert!(!playbook.claude_dir().exists());
        assert!(!dest.join(".credentials.json").exists());
        assert!(!dest.join("history.jsonl").exists());
        assert!(!dest.join("skills/synced").exists());
    }

    #[test]
    fn rejects_ids_that_try_to_escape() {
        let (_d, src, _, lauf) = fixture();
        for bad in [
            "skills:../.credentials.json",
            "skills:synced",
            "agents:a/b",
            "bogus:x",
        ] {
            assert!(import(&src, &lauf, &[bad.to_owned()]).is_err(), "{bad}");
        }
    }
}
