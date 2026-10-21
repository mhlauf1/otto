//! Knowledge (SPEC §7, ADR-021): notes on this Mac, fed to agents through each workspace's
//! own `CLAUDE.md` and `AGENTS.md`, never through repo files.
//!
//! Every path a caller hands in is relative and is resolved *inside* one area of one
//! workspace (or the shared global folder). Anything that escapes it, by `..`, an absolute
//! path or a symlink, is refused.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::workspace::{Workspace, WorkspaceId, WorkspaceLayout};

/// First line of every generated instruction file. Files without it belong to Mike.
pub const MANAGED_MARKER: &str = "<!-- Managed by Otto: regenerated from your Knowledge notes. Edit notes in Otto, or your own instructions in CLAUDE.user.md / AGENTS.user.md. -->";

const ABOUT_MIKE: &str = "About Mike.md";
const OVERVIEW: &str = "Overview.md";
/// Long notes are cut here in the generated files; the agent can open the rest.
const MAX_INLINE_CHARS: usize = 8_000;
const MAX_SUMMARY: usize = 140;
const MAX_DEPTH: usize = 4;
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_PROJECT_DOCS: usize = 200;

#[derive(Debug, thiserror::Error)]
pub enum KnowledgeError {
    #[error("That file isn't inside this workspace's knowledge.")]
    OutsideArea,
    #[error("Only Markdown notes (.md) can be edited here.")]
    NotMarkdown,
    #[error("That isn't an image Otto can show.")]
    NotImage,
    #[error("That image is too large to show (over 10 MB).")]
    TooLarge,
    #[error("A note needs a title.")]
    EmptyTitle,
    #[error("Assets are added as files, not written here.")]
    ReadOnlyArea,
    #[error("Couldn't read or write knowledge: {0}")]
    Io(#[from] io::Error),
}

/// Which part of knowledge a file is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Area {
    /// Shared by every workspace: About Mike.
    Global,
    /// The workspace's notes. Indexed for its agents.
    Notes,
    /// Never fed to agents unless Mike includes it in an Ask session.
    Private,
    Assets,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    Markdown,
    Image,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEntry {
    pub area: Area,
    /// Relative to the area's folder, with `/` separators.
    pub path: String,
    pub title: String,
    pub summary: Option<String>,
    pub kind: FileKind,
    pub updated_secs: u32,
}

/// `<otto root>/knowledge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgePaths {
    root: PathBuf,
}

impl KnowledgePaths {
    pub fn new(otto_root: &Path) -> Self {
        Self {
            root: otto_root.join("knowledge"),
        }
    }
    pub fn global_dir(&self) -> PathBuf {
        self.root.join("global")
    }
    pub fn workspace_dir(&self, ws: &WorkspaceId) -> PathBuf {
        self.root.join("workspaces").join(ws.as_str())
    }
    pub fn area_dir(&self, ws: &WorkspaceId, area: Area) -> PathBuf {
        match area {
            Area::Global => self.global_dir(),
            Area::Notes => self.workspace_dir(ws).join("notes"),
            Area::Private => self.workspace_dir(ws).join("private"),
            Area::Assets => self.workspace_dir(ws).join("assets"),
        }
    }

    /// Folders a normal Claude session in `ws` may open (`--add-dir`): its notes and the
    /// global folder. Never `private/`, never another workspace.
    pub fn session_dirs(&self, ws: &WorkspaceId) -> Vec<PathBuf> {
        vec![self.area_dir(ws, Area::Notes), self.global_dir()]
    }

    /// Folders an Ask session may open, besides its working folder (`notes/`): the global
    /// folder, the workspace's own projects, and `private/` only when Mike says so.
    pub fn ask_dirs(
        &self,
        ws: &WorkspaceId,
        own_projects: &[PathBuf],
        include_private: bool,
    ) -> Vec<PathBuf> {
        let mut dirs = vec![self.global_dir()];
        if include_private {
            dirs.push(self.area_dir(ws, Area::Private));
        }
        dirs.extend(own_projects.iter().cloned());
        dirs
    }
}

/// Create the folders and starter notes. Existing files are never overwritten.
///
/// # Errors
/// Fails if a folder or file can't be created.
pub fn seed(paths: &KnowledgePaths, ws: &Workspace) -> Result<(), KnowledgeError> {
    for dir in [
        paths.global_dir(),
        paths.area_dir(&ws.id, Area::Notes),
        paths.area_dir(&ws.id, Area::Private),
        paths.area_dir(&ws.id, Area::Assets),
    ] {
        fs::create_dir_all(&dir)?;
        set_private(&dir)?;
    }
    write_if_missing(&paths.global_dir().join(ABOUT_MIKE), STARTER_ABOUT_MIKE)?;
    for (area, name, body) in starter_notes(ws) {
        write_if_missing(&paths.area_dir(&ws.id, area).join(name), &body)?;
    }
    Ok(())
}

fn set_private(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(dir, fs::Permissions::from_mode(0o700))
}

fn write_if_missing(path: &Path, body: &str) -> io::Result<()> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => io::Write::write_all(&mut file, body.as_bytes()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
}

const STARTER_ABOUT_MIKE: &str = "# About Mike\n\nMike Laufersweiler: design engineer. Works at Playbook (since March 2020) and runs Lauf, a studio that builds websites for clients. Every workspace's agents read this note, so keep it to things that are fine everywhere.\n\n## How I like to work\n\n- \n\n## Stack and tools\n\n- \n\n## Tone\n\n- \n";

fn starter_notes(ws: &Workspace) -> Vec<(Area, &'static str, String)> {
    let overview = format!(
        "# {name} overview\n\nThe one note every {name} session reads in full. Say what {name} is and what matters most right now.\n",
        name = ws.name
    );
    let mut notes = vec![(Area::Notes, OVERVIEW, overview)];
    match ws.id.as_str() {
        "playbook" => notes.extend([
            (
                Area::Notes,
                "Company.md",
                "# Company\n\nPlaybook, formerly Lula Fit. A startup that's starting to take off. Mike has been here since March 2020.\n\n## History\n\n- Lula Fit\n- Rebrand to Playbook\n\n## What we build\n\n## Where it's going\n".to_owned(),
            ),
            (
                Area::Notes,
                "Architecture.md",
                "# Architecture\n\nHow Playbook's repos and services fit together. Ask can draft this from the project docs.\n\n## Repos\n\n## How they connect\n\n```mermaid\nflowchart LR\n  web --> core\n```\n".to_owned(),
            ),
            (
                Area::Notes,
                "People.md",
                "# People\n\nWho's who, and who to ask about what.\n".to_owned(),
            ),
            (
                Area::Notes,
                "Glossary.md",
                "# Glossary\n\nPlaybook words and what they mean.\n".to_owned(),
            ),
            (
                Area::Private,
                "Equity.md",
                "# Equity\n\nStock options, grants and vesting. Private: agents only see this in an Ask session with Include private on.\n".to_owned(),
            ),
        ]),
        "lauf" => notes.extend([
            (
                Area::Notes,
                "Studio.md",
                "# Studio\n\nLauf: what the studio does, how it works, and its own projects (like Otto).\n".to_owned(),
            ),
            (
                Area::Notes,
                "Clients.md",
                "# Clients\n\nOne line per client: who they are, their site, and anything to remember.\n".to_owned(),
            ),
        ]),
        _ => {}
    }
    notes
}

/// Resolve `rel` inside one area. Refuses absolute paths, `..`, and anything that a
/// symlink would carry outside the area.
///
/// # Errors
/// [`KnowledgeError::OutsideArea`] for any escape.
pub fn resolve(
    paths: &KnowledgePaths,
    ws: &WorkspaceId,
    area: Area,
    rel: &str,
) -> Result<PathBuf, KnowledgeError> {
    contained(&paths.area_dir(ws, area), rel)
}

/// `base/rel`, only if it stays inside `base` once symlinks are followed.
fn contained(base: &Path, relative: &str) -> Result<PathBuf, KnowledgeError> {
    let rel_path = Path::new(relative);
    if relative.is_empty()
        || !rel_path
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
    {
        return Err(KnowledgeError::OutsideArea);
    }
    let base = base.canonicalize()?;
    let joined = base.join(rel_path);
    // For a file that doesn't exist yet, check its nearest existing ancestor.
    let mut probe = joined.as_path();
    let resolved = loop {
        match probe.canonicalize() {
            Ok(found) => break found,
            Err(_) => probe = probe.parent().ok_or(KnowledgeError::OutsideArea)?,
        }
    };
    if resolved.starts_with(&base) {
        Ok(joined)
    } else {
        Err(KnowledgeError::OutsideArea)
    }
}

/// Every file in the workspace's knowledge, plus About Mike.
#[must_use]
pub fn list(paths: &KnowledgePaths, ws: &WorkspaceId) -> Vec<KnowledgeEntry> {
    let mut entries = Vec::new();
    for area in [Area::Global, Area::Notes, Area::Private, Area::Assets] {
        let dir = paths.area_dir(ws, area);
        let mut files = Vec::new();
        walk(&dir, 0, &mut files);
        for file in files {
            let Ok(rel) = file.strip_prefix(&dir) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            if area == Area::Global && rel != ABOUT_MIKE {
                continue;
            }
            entries.push(entry(area, rel, &file));
        }
    }
    entries.sort_by(|a, b| {
        (a.area as u8)
            .cmp(&(b.area as u8))
            .then_with(|| (a.path != OVERVIEW).cmp(&(b.path != OVERVIEW)))
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    entries
}

/// Regular files under `dir`, skipping dotfiles and symlinks.
fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for item in read.filter_map(Result::ok) {
        let path = item.path();
        if item.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() && depth < MAX_DEPTH {
            walk(&path, depth + 1, out);
        } else if meta.is_file() {
            out.push(path);
        }
    }
}

fn kind_of(path: &Path) -> FileKind {
    match ext(path).as_str() {
        "md" | "markdown" => FileKind::Markdown,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" => FileKind::Image,
        _ => FileKind::Other,
    }
}

fn ext(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn entry(area: Area, rel: String, file: &Path) -> KnowledgeEntry {
    let kind = kind_of(file);
    let stem = file
        .file_stem()
        .map_or_else(|| rel.clone(), |s| s.to_string_lossy().into_owned());
    let (title, summary) = if kind == FileKind::Markdown {
        fs::read_to_string(file)
            .map_or((stem.clone(), None), |text| title_and_summary(&text, &stem))
    } else {
        (stem, None)
    };
    let updated_secs = fs::metadata(file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| u32::try_from(d.as_secs()).unwrap_or(u32::MAX));
    KnowledgeEntry {
        area,
        path: rel,
        title,
        summary,
        kind,
        updated_secs,
    }
}

/// The first `# ` heading (else `fallback`), and the first line of prose after it.
fn title_and_summary(text: &str, fallback: &str) -> (String, Option<String>) {
    let mut title = None;
    let mut summary = None;
    let mut in_code = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code || line.is_empty() || line.starts_with("<!--") {
            continue;
        }
        if let Some(h) = line.strip_prefix("# ") {
            if title.is_none() {
                title = Some(h.trim().to_owned());
            }
            continue;
        }
        if line.starts_with('#') || line == "-" {
            continue;
        }
        summary = Some(shorten(line.trim_start_matches("- "), MAX_SUMMARY));
        break;
    }
    (title.unwrap_or_else(|| fallback.to_owned()), summary)
}

fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max - 1).collect();
    format!("{}…", cut.trim_end())
}

/// # Errors
/// Refuses escapes and non-Markdown files; fails on I/O errors.
pub fn read_note(
    paths: &KnowledgePaths,
    ws: &WorkspaceId,
    area: Area,
    rel: &str,
) -> Result<String, KnowledgeError> {
    let path = resolve(paths, ws, area, rel)?;
    if kind_of(&path) != FileKind::Markdown {
        return Err(KnowledgeError::NotMarkdown);
    }
    Ok(fs::read_to_string(path)?)
}

/// Save a Markdown note in Global, Notes or Private.
///
/// # Errors
/// Refuses escapes, assets and non-Markdown paths; fails on I/O errors.
pub fn save_note(
    paths: &KnowledgePaths,
    ws: &WorkspaceId,
    area: Area,
    rel: &str,
    content: &str,
) -> Result<(), KnowledgeError> {
    if area == Area::Assets {
        return Err(KnowledgeError::ReadOnlyArea);
    }
    if area == Area::Global && rel != ABOUT_MIKE {
        return Err(KnowledgeError::OutsideArea);
    }
    let path = resolve(paths, ws, area, rel)?;
    if kind_of(&path) != FileKind::Markdown {
        return Err(KnowledgeError::NotMarkdown);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Write then rename, so a crash never leaves half a note.
    let tmp = path.with_extension("md.otto-tmp");
    fs::write(&tmp, content)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Create `<title>.md` (made unique) in Notes or Private, and return its relative path.
///
/// # Errors
/// Refuses an empty title or another area; fails on I/O errors.
pub fn create_note(
    paths: &KnowledgePaths,
    ws: &WorkspaceId,
    area: Area,
    title: &str,
) -> Result<String, KnowledgeError> {
    if !matches!(area, Area::Notes | Area::Private) {
        return Err(KnowledgeError::ReadOnlyArea);
    }
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let safe: String = title
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':') || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    let stem: String = safe
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches('.')
        .chars()
        .take(80)
        .collect();
    if stem.trim().is_empty() {
        return Err(KnowledgeError::EmptyTitle);
    }
    let dir = paths.area_dir(ws, area);
    fs::create_dir_all(&dir)?;
    let mut name = format!("{stem}.md");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{stem} {n}.md");
        n += 1;
    }
    save_note(paths, ws, area, &name, &format!("# {title}\n\n"))?;
    Ok(name)
}

/// An image as a `data:` URL, so the strict CSP needs no file access.
///
/// # Errors
/// Refuses escapes, non-images and images over 10 MB.
pub fn image_data_url(
    paths: &KnowledgePaths,
    ws: &WorkspaceId,
    area: Area,
    rel: &str,
) -> Result<String, KnowledgeError> {
    let path = resolve(paths, ws, area, rel)?;
    let mime = match ext(&path).as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => return Err(KnowledgeError::NotImage),
    };
    if fs::metadata(&path)?.len() > MAX_IMAGE_BYTES {
        return Err(KnowledgeError::TooLarge);
    }
    Ok(format!("data:{mime};base64,{}", base64(&fs::read(path)?)))
}

/// Copy an image Mike picked into the workspace's assets, under a unique name. Returns the
/// asset's path relative to `assets/`.
///
/// # Errors
/// Refuses non-images and images over 10 MB; fails on I/O errors.
pub fn add_asset(
    paths: &KnowledgePaths,
    ws: &WorkspaceId,
    source: &Path,
) -> Result<String, KnowledgeError> {
    if kind_of(source) != FileKind::Image {
        return Err(KnowledgeError::NotImage);
    }
    let meta = fs::metadata(source)?;
    if !meta.is_file() {
        return Err(KnowledgeError::NotImage);
    }
    if meta.len() > MAX_IMAGE_BYTES {
        return Err(KnowledgeError::TooLarge);
    }
    let stem = source
        .file_stem()
        .map_or_else(|| "image".to_owned(), |s| s.to_string_lossy().into_owned());
    let extension = ext(source);
    let dir = paths.area_dir(ws, Area::Assets);
    fs::create_dir_all(&dir)?;
    let mut name = format!("{stem}.{extension}");
    let mut n = 2;
    while dir.join(&name).exists() {
        name = format!("{stem} {n}.{extension}");
        n += 1;
    }
    fs::copy(source, dir.join(&name))?;
    Ok(name)
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[((n >> shift) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Which CLI a generated file is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    Claude,
    Codex,
}

impl Agent {
    fn managed_file(self, layout: &WorkspaceLayout) -> PathBuf {
        match self {
            Self::Claude => layout.claude_dir().join("CLAUDE.md"),
            Self::Codex => layout.codex_dir().join("AGENTS.md"),
        }
    }
    /// Mike's own instructions, which Otto includes but never edits.
    pub fn user_file(self, layout: &WorkspaceLayout) -> PathBuf {
        match self {
            Self::Claude => layout.claude_dir().join("CLAUDE.user.md"),
            Self::Codex => layout.codex_dir().join("AGENTS.user.md"),
        }
    }
}

/// The generated instruction file for one CLI in one workspace. Reads only that
/// workspace's notes, About Mike, and that workspace's own user file.
#[must_use]
pub fn instructions(
    paths: &KnowledgePaths,
    ws: &Workspace,
    layout: &WorkspaceLayout,
    agent: Agent,
) -> String {
    let mut out = format!(
        "{MANAGED_MARKER}\n\n# Otto workspace: {name}\n\nYou're working as Mike's {name} identity ({user}, {email}). Otto keeps this workspace isolated: don't use or mention other accounts.\n",
        name = ws.name,
        user = ws.identity.github_user().as_str(),
        email = ws.identity.git_email(),
    );
    if let Ok(own) = fs::read_to_string(agent.user_file(layout))
        && !own.trim().is_empty()
    {
        out.push_str("\n## Mike's instructions\n\n");
        out.push_str(&demote(own.trim(), 2));
        out.push('\n');
    }
    let notes_dir = paths.area_dir(&ws.id, Area::Notes);
    if let Ok(about) = fs::read_to_string(paths.global_dir().join(ABOUT_MIKE)) {
        out.push_str("\n## About Mike\n\n");
        out.push_str(&inline(&about, &paths.global_dir().join(ABOUT_MIKE)));
    }
    if let Ok(overview) = fs::read_to_string(notes_dir.join(OVERVIEW)) {
        let _ = write!(out, "\n## {} overview\n\n", ws.name);
        out.push_str(&inline(&overview, &notes_dir.join(OVERVIEW)));
    }
    let index: Vec<KnowledgeEntry> = list(paths, &ws.id)
        .into_iter()
        .filter(|e| e.area == Area::Notes && e.path != OVERVIEW && e.kind == FileKind::Markdown)
        .collect();
    if !index.is_empty() {
        let _ = write!(
            out,
            "\n## {name} knowledge\n\nMike's notes for {name}. Open one when it's relevant to the task:\n\n",
            name = ws.name
        );
        for e in index {
            let path = notes_dir.join(&e.path);
            let _ = match e.summary {
                Some(s) => writeln!(out, "- **{}**: {s} (`{}`)", e.title, path.display()),
                None => writeln!(out, "- **{}** (`{}`)", e.title, path.display()),
            };
        }
    }
    out
}

/// A note's body with its own `# ` title dropped and headings pushed down a level, cut at
/// [`MAX_INLINE_CHARS`].
fn inline(text: &str, path: &Path) -> String {
    let body: String = text
        .lines()
        .skip_while(|l| l.trim().is_empty())
        .enumerate()
        .filter(|(i, l)| !(*i == 0 && l.starts_with("# ")))
        .map(|(_, l)| l)
        .collect::<Vec<_>>()
        .join("\n");
    let body = demote(body.trim(), 1);
    if body.chars().count() <= MAX_INLINE_CHARS {
        return format!("{body}\n");
    }
    let cut: String = body.chars().take(MAX_INLINE_CHARS).collect();
    format!("{cut}\n\n(Continues in `{}`.)\n", path.display())
}

/// Push Markdown headings down `levels`, so a note nests under Otto's `##` section headings.
fn demote(text: &str, levels: usize) -> String {
    let extra = "#".repeat(levels);
    let mut in_code = false;
    text.lines()
        .map(|l| {
            if l.trim_start().starts_with("```") {
                in_code = !in_code;
            }
            if !in_code && l.starts_with('#') {
                format!("{extra}{l}")
            } else {
                l.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Write both generated files for a workspace. A hand-written file already sitting there
/// (from before Otto managed it) is first moved to the user file, so nothing is lost.
///
/// # Errors
/// Fails on I/O errors.
pub fn write_instructions(
    paths: &KnowledgePaths,
    ws: &Workspace,
    layout: &WorkspaceLayout,
) -> Result<(), KnowledgeError> {
    for agent in [Agent::Claude, Agent::Codex] {
        let managed = agent.managed_file(layout);
        if let Ok(existing) = fs::read_to_string(&managed) {
            let user = agent.user_file(layout);
            if !existing.starts_with(MANAGED_MARKER) && !user.exists() {
                fs::rename(&managed, &user)?;
            }
        }
        if let Some(parent) = managed.parent() {
            fs::create_dir_all(parent)?;
        }
        let body = instructions(paths, ws, layout, agent);
        if fs::read_to_string(&managed).ok().as_deref() != Some(body.as_str()) {
            fs::write(&managed, body)?;
        }
    }
    Ok(())
}

/// A Markdown doc inside a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDoc {
    /// Relative to the project root, with `/` separators.
    pub path: String,
    pub title: String,
}

/// `CLAUDE.md`, `AGENTS.md`, `README*` and `docs/**/*.md` in a project, without following
/// symlinks.
#[must_use]
pub fn project_docs(root: &Path) -> Vec<ProjectDoc> {
    let mut found = Vec::new();
    if let Ok(read) = fs::read_dir(root) {
        let mut names: Vec<String> = read
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| {
                let lower = n.to_lowercase();
                lower == "claude.md"
                    || lower == "agents.md"
                    || (lower.starts_with("readme") && kind_of(Path::new(n)) == FileKind::Markdown)
            })
            .collect();
        names.sort();
        found.extend(names.into_iter().map(|n| root.join(n)));
    }
    let mut docs = Vec::new();
    walk(&root.join("docs"), 0, &mut docs);
    docs.retain(|p| kind_of(p) == FileKind::Markdown);
    docs.sort();
    found.extend(docs);
    found
        .into_iter()
        .take(MAX_PROJECT_DOCS)
        .filter_map(|file| {
            let rel = file
                .strip_prefix(root)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            let stem = file.file_stem()?.to_string_lossy().into_owned();
            let title = fs::read_to_string(&file)
                .map_or_else(|_| stem.clone(), |t| title_and_summary(&t, &stem).0);
            Some(ProjectDoc { path: rel, title })
        })
        .collect()
}

/// Read one doc from a project, only if it's one [`project_docs`] lists.
///
/// # Errors
/// Refuses anything else; fails on I/O errors.
pub fn read_project_doc(root: &Path, rel: &str) -> Result<String, KnowledgeError> {
    if !project_docs(root).iter().any(|d| d.path == rel) {
        return Err(KnowledgeError::OutsideArea);
    }
    let path = contained(root, rel)?;
    Ok(fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::default_workspaces;
    use crate::workspace::OttoPaths;

    struct World {
        dir: tempfile::TempDir,
        otto: OttoPaths,
        k: KnowledgePaths,
        playbook: Workspace,
        lauf: Workspace,
    }

    fn world() -> World {
        let dir = tempfile::tempdir().unwrap();
        let otto = OttoPaths::new(dir.path().join("Otto"));
        let k = KnowledgePaths::new(otto.root());
        let mut ws = default_workspaces().unwrap().into_iter();
        let (playbook, lauf) = (ws.next().unwrap(), ws.next().unwrap());
        seed(&k, &playbook).unwrap();
        seed(&k, &lauf).unwrap();
        World {
            dir,
            otto,
            k,
            playbook,
            lauf,
        }
    }

    #[test]
    fn seeds_starter_notes_once_and_never_overwrites() {
        let w = world();
        let company = w.k.area_dir(&w.playbook.id, Area::Notes).join("Company.md");
        assert!(fs::read_to_string(&company).unwrap().contains("Lula Fit"));
        fs::write(&company, "# Company\n\nMine now.\n").unwrap();
        seed(&w.k, &w.playbook).unwrap();
        assert_eq!(
            fs::read_to_string(&company).unwrap(),
            "# Company\n\nMine now.\n"
        );
        assert!(
            w.k.area_dir(&w.playbook.id, Area::Private)
                .join("Equity.md")
                .exists()
        );
        assert!(
            !w.k.area_dir(&w.lauf.id, Area::Notes)
                .join("Company.md")
                .exists()
        );
    }

    #[test]
    fn lists_titles_and_summaries_with_overview_first() {
        let w = world();
        let notes: Vec<_> = list(&w.k, &w.lauf.id)
            .into_iter()
            .filter(|e| e.area == Area::Notes)
            .collect();
        assert_eq!(notes[0].path, "Overview.md");
        let clients = notes.iter().find(|e| e.path == "Clients.md").unwrap();
        assert_eq!(clients.title, "Clients");
        assert_eq!(
            clients.summary.as_deref(),
            Some("One line per client: who they are, their site, and anything to remember.")
        );
        let about = list(&w.k, &w.lauf.id);
        assert!(
            about
                .iter()
                .any(|e| e.area == Area::Global && e.path == "About Mike.md")
        );
    }

    #[test]
    fn refuses_paths_that_escape_their_area() {
        let w = world();
        let ws = &w.lauf.id;
        for bad in [
            "../private/x.md",
            "/etc/passwd",
            "a/../../b.md",
            "",
            "./x.md",
        ] {
            assert!(
                matches!(
                    resolve(&w.k, ws, Area::Notes, bad),
                    Err(KnowledgeError::OutsideArea)
                ),
                "{bad}"
            );
        }
        // A symlink inside notes/ that points at another workspace is refused too.
        let link = w.k.area_dir(ws, Area::Notes).join("sneaky");
        std::os::unix::fs::symlink(w.k.workspace_dir(&w.playbook.id), &link).unwrap();
        assert!(read_note(&w.k, ws, Area::Notes, "sneaky/notes/Company.md").is_err());
        assert!(save_note(&w.k, ws, Area::Notes, "sneaky/notes/Company.md", "x").is_err());
    }

    #[test]
    fn saves_and_creates_markdown_notes_only() {
        let w = world();
        let ws = &w.lauf.id;
        let name = create_note(&w.k, ws, Area::Notes, "Brady Digital / Consulting").unwrap();
        assert_eq!(name, "Brady Digital Consulting.md");
        assert_eq!(
            create_note(&w.k, ws, Area::Notes, "Brady Digital / Consulting").unwrap(),
            "Brady Digital Consulting 2.md"
        );
        save_note(&w.k, ws, Area::Notes, &name, "# Brady\n\nHi.\n").unwrap();
        assert_eq!(
            read_note(&w.k, ws, Area::Notes, &name).unwrap(),
            "# Brady\n\nHi.\n"
        );
        assert!(matches!(
            save_note(&w.k, ws, Area::Notes, "x.sh", "echo"),
            Err(KnowledgeError::NotMarkdown)
        ));
        assert!(save_note(&w.k, ws, Area::Assets, "a.md", "x").is_err());
        assert!(save_note(&w.k, ws, Area::Global, "Other.md", "x").is_err());
        assert!(matches!(
            create_note(&w.k, ws, Area::Notes, "  "),
            Err(KnowledgeError::EmptyTitle)
        ));
    }

    #[test]
    fn instructions_hold_about_mike_overview_and_an_index_but_never_private() {
        let w = world();
        let layout = w.otto.workspace(&w.playbook.id);
        fs::create_dir_all(layout.claude_dir()).unwrap();
        fs::write(layout.claude_dir().join("CLAUDE.md"), "Use pnpm.\n").unwrap();
        write_instructions(&w.k, &w.playbook, &layout).unwrap();

        // Mike's hand-written file moved aside, not lost, and included.
        assert_eq!(
            fs::read_to_string(layout.claude_dir().join("CLAUDE.user.md")).unwrap(),
            "Use pnpm.\n"
        );
        let claude = fs::read_to_string(layout.claude_dir().join("CLAUDE.md")).unwrap();
        assert!(claude.starts_with(MANAGED_MARKER));
        assert!(claude.contains("Use pnpm."));
        assert!(claude.contains("## About Mike"));
        assert!(claude.contains("## Playbook overview"));
        assert!(claude.contains("- **Company**: Playbook, formerly Lula Fit."));
        assert!(!claude.contains("Equity"), "private notes are never fed");
        assert!(!claude.contains("/private/"));
        let codex = fs::read_to_string(layout.codex_dir().join("AGENTS.md")).unwrap();
        assert!(codex.contains("- **Company**"));
        assert!(
            !codex.contains("Use pnpm."),
            "Claude's own file isn't Codex's"
        );

        // Regenerating keeps the user file where it is.
        write_instructions(&w.k, &w.playbook, &layout).unwrap();
        assert!(layout.claude_dir().join("CLAUDE.user.md").exists());
    }

    #[test]
    fn note_headings_nest_one_level_under_the_section() {
        let w = world();
        let layout = w.otto.workspace(&w.lauf.id);
        let text = instructions(&w.k, &w.lauf, &layout, Agent::Claude);
        assert!(text.contains("## About Mike\n"));
        assert!(text.contains("\n### How I like to work\n"));
        assert!(!text.contains("####"));
    }

    #[test]
    fn long_notes_are_cut_with_a_pointer_to_the_file() {
        let w = world();
        let notes = w.k.area_dir(&w.lauf.id, Area::Notes);
        fs::write(
            notes.join("Overview.md"),
            format!("# Lauf\n\n{}", "word ".repeat(4000)),
        )
        .unwrap();
        let layout = w.otto.workspace(&w.lauf.id);
        let text = instructions(&w.k, &w.lauf, &layout, Agent::Claude);
        assert!(text.contains("(Continues in `"));
        assert!(text.len() < 12_000);
    }

    #[test]
    fn project_docs_list_the_usual_files_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("CLAUDE.md"), "# Claude notes\n").unwrap();
        fs::write(root.join("README.md"), "# Stoc\n").unwrap();
        fs::write(root.join("secrets.env"), "KEY=1").unwrap();
        fs::create_dir_all(root.join("docs/guides")).unwrap();
        fs::write(root.join("docs/guides/deploy.md"), "# Deploying\n").unwrap();
        fs::write(root.join("docs/logo.png"), "png").unwrap();
        let docs: Vec<_> = project_docs(root)
            .into_iter()
            .map(|d| (d.path, d.title))
            .collect();
        assert_eq!(
            docs,
            [
                ("CLAUDE.md".into(), "Claude notes".into()),
                ("README.md".into(), "Stoc".into()),
                ("docs/guides/deploy.md".into(), "Deploying".into()),
            ]
        );
        assert!(read_project_doc(root, "README.md").is_ok());
        assert!(read_project_doc(root, "secrets.env").is_err());
        assert!(read_project_doc(root, "../etc/passwd").is_err());
    }

    #[test]
    fn images_become_data_urls() {
        let w = world();
        let assets = w.k.area_dir(&w.lauf.id, Area::Assets);
        fs::write(assets.join("dot.png"), [0x89, b'P', b'N', b'G']).unwrap();
        assert_eq!(
            image_data_url(&w.k, &w.lauf.id, Area::Assets, "dot.png").unwrap(),
            "data:image/png;base64,iVBORw=="
        );
        fs::write(assets.join("notes.txt"), "x").unwrap();
        assert!(image_data_url(&w.k, &w.lauf.id, Area::Assets, "notes.txt").is_err());
    }

    #[test]
    fn assets_copy_in_images_only_with_unique_names() {
        let w = world();
        let src = w.dir.path().join("logo.png");
        fs::write(&src, b"png").unwrap();
        assert_eq!(add_asset(&w.k, &w.lauf.id, &src).unwrap(), "logo.png");
        assert_eq!(add_asset(&w.k, &w.lauf.id, &src).unwrap(), "logo 2.png");
        let script = w.dir.path().join("run.sh");
        fs::write(&script, b"echo").unwrap();
        assert!(matches!(
            add_asset(&w.k, &w.lauf.id, &script),
            Err(KnowledgeError::NotImage)
        ));
        assert!(
            !w.k.area_dir(&w.playbook.id, Area::Assets)
                .join("logo.png")
                .exists()
        );
    }

    #[test]
    fn base64_matches_the_standard_vectors() {
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(input.as_bytes()), expected);
        }
    }
}
