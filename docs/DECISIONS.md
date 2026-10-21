# Architecture decision records

Each ADR records the context, the options, the decision and its consequences. Newest last. Status is one of: Proposed, Accepted or Superseded.

---

## ADR-000 — Template

- **Status:**
- **Context:** what problem forced a decision.
- **Options:** what we considered.
- **Decision:** what we chose.
- **Consequences:** what gets easier, what gets harder, what we must watch.

---

## ADR-001 — Split the Rust code into `otto-core` and a thin Tauri shell

- **Status:** Accepted (2026-10-20)
- **Context:** The isolation guarantees (SPEC §4) must be proven by tests in CI. Code that depends on Tauri needs a window runtime, which makes headless tests slow and brittle.
- **Options:** (a) everything in `src-tauri`; (b) a Cargo workspace with a pure `crates/otto-core` library plus `src-tauri` holding only commands and wiring.
- **Decision:** (b). `otto-core` owns the env builder, workspace model, store, hooks and Identity Doctor logic. `src-tauri` translates IPC calls into core calls.
- **Consequences:** Isolation tests run with plain `cargo test`. Command handlers stay thin. The cost is one more crate boundary to maintain.

## ADR-002 — Typed IPC with `tauri-specta`

- **Status:** Accepted (2026-10-20)
- **Context:** SPEC §5.2 requires typed IPC end to end, with no stringly-typed `invoke("name")` calls.
- **Options:** `ts-rs` (exports types only, so command names stay strings); `tauri-specta` (exports types **and** a typed function per command, including error types).
- **Decision:** `tauri-specta`. Bindings are generated into `src/bindings.ts` in debug builds and checked in. CI fails if they are stale.
- **Consequences:** Renaming a Rust command breaks the TypeScript build, which is what we want. `tauri-specta` tracks Tauri releases, so pin versions together.

## ADR-003 — Styling with CSS Modules plus generated CSS variables

- **Status:** Accepted (2026-10-20, chosen by Mike)
- **Context:** SPEC §6.2 makes `src/design/tokens.ts` the single source of truth, generates CSS variables from it, and bans hard-coded values in components.
- **Options:** CSS Modules; vanilla-extract; Tailwind generated from tokens.
- **Decision:** CSS Modules reading CSS variables generated from `tokens.ts`. A Vitest test fails if `tokens.css` is stale. Stylelint (`declaration-strict-value`) rejects raw colours and spacing.
- **Consequences:** Plain, learnable CSS with fine native-feel control. Token typing is enforced by the generator and lint rather than by the type checker.

## ADR-004 — Repo identity

- **Status:** Accepted (2026-10-20)
- **Context:** Otto is a Lauf portfolio piece, and Otto exists to stop identity mix-ups.
- **Decision:** The repo lives under GitHub `mhlauf1`. Commits use `mhlauf1@gmail.com`, set in the repo-local git config.
- **Consequences:** Building it from a Playbook-signed-in Claude session is a known wrinkle. Mike may move development to the Lauf Claude account.

## ADR-005 — Per-CLI credential isolation (verified 2026-10-20)

- **Status:** Proposed. Needs Mike's sign-off because it changes SPEC §4.1.
- **Context:** SPEC §4.1 assumes that pointing each CLI at a per-workspace directory isolates its login. Research was checked against official docs, CLI source code, and local tests on Claude Code 2.1.304, gh 2.48, Apple git 2.39.5 and OpenSSH 9.9p2.
- **Findings and decisions:**
  - **Claude Code — isolates correctly.** With `CLAUDE_CONFIG_DIR` set, credentials and `.claude.json` live in that directory, and the Keychain entry is keyed to the directory ([docs](https://code.claude.com/docs/en/authentication#credential-management)). A local test with an empty dir reported "not logged in", so the default login did not leak. **Also set** `ANTHROPIC_CONFIG_DIR=<ws>/anthropic` (Console profiles live there and ignore XDG) and `CLAUDE_CODE_TMPDIR=<ws>/tmp` (otherwise both workspaces share `/private/tmp/claude-<uid>`). Login check: `claude auth status` (JSON, exits 1 when logged out). Resume: Otto sets the ID up front with `--session-id <uuid>` and later runs `--resume <uuid>`.
  - **Codex — isolates when set to file storage.** `CODEX_HOME` is documented, and the directory must exist before launch. Otto writes `cli_auth_credentials_store = "file"` into `<ws>/codex/config.toml` so auth stays in `<ws>/codex/auth.json`. `OPENAI_API_KEY` and `CODEX_API_KEY` override the login, so both are stripped. Login check: `codex login status`. Resume: `codex resume <id>`.
  - **gh — does NOT isolate by default.** `GH_CONFIG_DIR` moves only `hosts.yml`. The token sits in the shared login Keychain (`gh:github.com`, account = username), and gh falls back to an **unkeyed** entry that whichever workspace logged in last overwrites. **Decision:** every workspace logs in with `gh auth login --insecure-storage`, so the token lives in `<ws>/gh/hosts.yml` (0600, inside Otto's Application Support directory) and gh never touches the Keychain. Identity Doctor checks that `hosts.yml` holds a token for the expected user. Require gh ≥ 2.40.
  - **git — isolates.** `GIT_CONFIG_GLOBAL` also suppresses `$XDG_CONFIG_HOME/git/config`. `GIT_CONFIG_NOSYSTEM=1` removes Apple's system `credential.helper=osxkeychain` (tested). Repo-local `.git/config` still applies, so Otto **also** sets `GIT_AUTHOR_NAME/EMAIL`, `GIT_COMMITTER_NAME/EMAIL` and `GIT_TERMINAL_PROMPT=0`.
- **Consequences:** The gh token is a file on disk rather than a Keychain item. That's acceptable: it never leaves the workspace directory, and it's what makes isolation structural. Otto still never reads the token itself.

## ADR-006 — SSH: ignore user config; passphrase via askpass

- **Status:** Proposed
- **Context:** ssh resolves `~` from the system user record, not `$HOME`, so it always reads the real `~/.ssh/config`. On this Mac that file contains `UseKeychain` and `AddKeysToAgent`. `IdentityAgent=none` combined with a passphrase-protected key would prompt on every push.
- **Decision:** `GIT_SSH_COMMAND = ssh -F none -i <ws>/ssh/id_ed25519 -o IdentitiesOnly=yes -o IdentityAgent=none -o UserKnownHostsFile=<ws>/ssh/known_hosts -o StrictHostKeyChecking=accept-new -o ControlMaster=no -o ControlPath=none`. Passphrases come from an Otto askpass helper (`SSH_ASKPASS=<otto>/askpass`, `SSH_ASKPASS_REQUIRE=force`), which reads that workspace's item from the macOS Keychain. Apple's `UseKeychain` is **not** used, because it relies on a shared lookup keyed only by the key's file path.
- **Consequences:** Behaviour is deterministic and per workspace. Otto ships and signs a small askpass binary in M1.

## ADR-007 — Hooks are defense in depth, not the guarantee

- **Status:** Proposed
- **Context:** `--no-verify` skips `pre-commit` and `pre-push`, and agents can pass it.
- **Decision:** The structural guarantee is credential scoping (ADR-005/006): a workspace has no credential that GitHub would accept for the other identity's repos. Hooks catch honest mistakes early. Set `core.hooksPath` in the **workspace** `gitconfig` (it applies to every repo, with nothing written into the repo), and check at repo-open time that no repo-local value overrides it. M1 isolation tests prove pushes fail **even with `--no-verify`**.
- **Consequences:** SPEC §4.3 wording should say "guardrail", not "guarantee". Note that `mikelauf` and `mhlauf1` may each have collaborator access to the other's repos. If they do, the owner allowlist check on the push remote (in Otto, at repo open, and in the hook) is the only barrier, so Otto must also re-check remotes before running each task.

## ADR-008 — Environment allowlist and HOME

- **Status:** Proposed
- **Decision:** `env_clear()`, then set only the following:
  - **Base:** `HOME` (the real one), `USER`, `LOGNAME`, `SHELL`, `PATH` (explicit, not inherited), `TERM=xterm-256color`, `COLORTERM=truecolor`, `LANG`, `TMPDIR=<ws>/tmp`, `__CF_USER_TEXT_ENCODING`.
  - **Per workspace:** `CLAUDE_CONFIG_DIR`, `ANTHROPIC_CONFIG_DIR`, `CLAUDE_CODE_TMPDIR`, `CODEX_HOME`, `GH_CONFIG_DIR`, `GH_PROMPT_DISABLED=1`, `GH_NO_UPDATE_NOTIFIER=1`, `XDG_CONFIG_HOME/DATA_HOME/STATE_HOME/CACHE_HOME`, `ZDOTDIR=<ws>/zsh`, the git variables from ADR-005, `GIT_SSH_COMMAND`, `SSH_ASKPASS` and `SSH_ASKPASS_REQUIRE`.
  - **Must never be passed:** `GH_TOKEN`, `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`, `GH_HOST`, `GH_REPO`, `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`, `CLAUDE_CODE_USE_*`, `OPENAI_API_KEY`, `CODEX_API_KEY`, `SSH_AUTH_SOCK`, and any `GIT_*` variable Otto didn't set (including `GIT_CONFIG_PARAMETERS`/`COUNT`, `GIT_ASKPASS`, `GIT_SSH`, `GIT_DIR`).
- **Why the real HOME:** a fake HOME breaks the Claude native installer and updater and the Homebrew/nvm toolchains, and it doesn't stop ssh reading the real config anyway. Everything that holds credentials is already redirected.
- **Known residual risk:** Claude Code's Bash tool snapshots the real `~/.zshrc` (it reportedly ignores `ZDOTDIR`), and Codex runs a login shell that reads `~/.zprofile`. Anything those files export reaches agent subprocesses. **Mitigations:** set `CLAUDE_CODE_SHELL` to an Otto wrapper that re-clears the environment; Identity Doctor warns if rc files export token or socket variables; isolation test 3 runs a real agent tool call and inspects its environment.

## ADR-009 — Toolchain pins

- **Status:** Accepted (2026-10-20)
- **Decision:**
  - **Rust 1.99.1** (`rust-toolchain.toml`).
  - **Node 24.22 LTS** (`.nvmrc`). ESLint 10 needs ≥ 22.13, and the machine had 22.12.
  - **pnpm 10.16** (`packageManager`).
  - **Tauri 2.12.x**. Tauri 3 is still alpha and the SPEC says Tauri 2.
  - **tauri-specta 2.0.0-rc.25**, pinned exactly with `=`, because release candidates can break between versions.
  - **TypeScript 6.0.3**, not 7.0. `typescript-eslint` 8.72 supports only `<6.1`.
- **Rules:** every JS dependency is pinned exactly (`save-exact`). Only allowlisted packages may run install scripts (`pnpm.onlyBuiltDependencies`: esbuild). CI actions are pinned to commit SHAs.
- **Consequences:** Upgrades are deliberate PRs. Move to TypeScript 7 once typescript-eslint supports it.

## ADR-010 — Native window chrome and vibrancy

- **Status:** Accepted (2026-10-20)
- **Context:** SPEC §6.1 asks for a translucent sidebar with vibrancy and an inset for the traffic lights.
- **Decision:**
  - `titleBarStyle: Overlay`, hidden title, traffic lights at (18, 22).
  - `transparent: true` with the native `sidebar` window effect.
  - The web layer paints a translucent sidebar token over the vibrancy and a solid content surface.
  - Transparent windows on macOS need `macOSPrivateApi: true`.
- **Consequences:** Otto can't ship on the Mac App Store (it's distributed as a notarized `.dmg`, per SPEC §5.1). Vibrancy appears only inside the shell; browser previews fall back to the solid `window` colour.

## ADR-011 — Only identity checks gate pushes

- **Status:** Accepted (2026-10-20). This is a judgment call during M1; Mike can overrule it.
- **Context:** SPEC §4.4 says "anything red blocks pushes". On first real run, both workspaces were blocked because Codex wasn't installed and Claude wasn't signed in yet. Neither affects _who_ a push goes out as.
- **Decision:** Identity Doctor reports two verdicts. `allGreen` means nothing is red. `pushAllowed` means the identity checks pass: hooks, git author, GitHub login and SSH (`PUSH_GATING` in `doctor.rs`). Only `pushAllowed` is written to the `doctor-status` file that `pre-push` reads. Agent login problems still show red in the UI.
- **Consequences:** A red agent login doesn't block git work. The sidebar ✓/✕ reflects push safety.

## ADR-012 — Git may never prompt for HTTPS credentials

- **Status:** Accepted (2026-10-20)
- **Context:** An isolation test showed that when a workspace has no GitHub login, git falls back to `SSH_ASKPASS` to ask for an **HTTPS password**. With Otto's askpass helper installed, that could have handed the workspace's SSH key passphrase to a remote server as a password.
- **Decision:**
  - `GIT_ASKPASS=/usr/bin/false`, so HTTPS credentials come only from the gh credential helper, or the operation fails.
  - The askpass helper answers only ssh's exact passphrase prompt for that workspace's key.
  - Both are covered by isolation tests.
- **Also learned:** git contacts the remote **before** `pre-push` runs, so authentication with the workspace's own credential happens first. The hook test for URL-rewrite attacks therefore calls the hook directly, the same way git does, rather than relying on the network.

## ADR-013 — Terminal sessions

- **Status:** Accepted (2026-10-20)
- **Decision:**
  - PTYs are spawned in `otto-core` (`pty.rs`, using portable-pty) from an `IsolatedEnv`, exactly like other children, and are covered by the isolation suite.
  - Output streams to the UI over a Tauri `Channel` as UTF-8 text. `Utf8Stream` holds back characters split across reads.
  - Each xterm.js `Terminal` lives outside React (`src/sessions/registry.ts`), so scrollback survives tab and workspace switches. React only attaches it.
  - Claude sessions get a UUID up front (`--session-id`), so M5 can resume them.
  - Sign-in runs each CLI's own flow in a PTY inside the workspace: `claude auth login`, `codex login`.
  - The shell agent is `zsh -l` with `ZDOTDIR` set to the workspace. Its prompt names the workspace.
  - Sessions live in memory until M5 adds persistence.
- **Consequences:** Closing a session kills its program; dropping the PTY kills the child. Typing and dictation (Wispr Flow) go straight into the terminal until the M3 composer.

## ADR-014 — `freezePrototype` is off

- **Status:** Accepted (2026-10-20)
- **Context:** Tauri's `freezePrototype` freezes `Object.prototype` to stop prototype-pollution attacks. xterm.js 6 assigns to an inherited property while loading, which throws under a frozen prototype ("Attempted to assign to readonly property"), and the window rendered blank. Browser tests missed it because only the Tauri webview freezes prototypes.
- **Decision:** Turn `freezePrototype` off. Prototype pollution needs attacker-controlled input reaching the webview. Otto loads no remote content, has a strict CSP (`script-src 'self'`) and exposes only typed commands, so the remaining risk is low.
- **Consequences:** The Playwright smoke test now fails on any page error. Revisit when xterm.js no longer trips the frozen prototype.

## ADR-015 — Projects: auto-discover, confirm, local-only

- **Status:** Accepted (2026-10-20, chosen by Mike)
- **Decision:**
  - Otto scans `~/Desktop`, `~/Developer`, `~/Projects`, `~/code`, `~/dev` and `~/src` (depth 5) for git repos. It reads each repo's remotes with **no identity at all** (`env_clear`, `GIT_CONFIG_GLOBAL=/dev/null`) and suggests a workspace from the GitHub owner. Mike confirms once.
  - Repos with no remote are imported only when Mike picks the workspace. Pushes stay locked until a remote owned by that workspace exists.
  - Repos with a credential in their config, or a non-GitHub remote, can't be imported.
  - Sessions open inside a project, and the project is **re-verified on every open**.
- **Consequences:** The `repos` table allows a NULL `remote_url` (migration 0003). GitLab repos (e.g. `lulafit/signup`) aren't supported in V1.

## ADR-016 — Bringing in the normal Claude setup

- **Status:** Accepted (2026-10-20, chosen by Mike: per-workspace choice)
- **Context:** Each workspace's `CLAUDE_CONFIG_DIR` starts empty, so none of Mike's skills, plugins, MCP servers or settings exist there.
- **Decision:**
  - A picker lists items from `~/.claude` and `~/.claude.json` with a checkbox per workspace. Import **copies** them into the workspace's config dir, rewriting absolute plugin paths.
  - Never offered: `.credentials.json`, history, sessions, projects, telemetry, `oauthAccount`, or account-synced skills (each workspace's own login syncs those).
  - MCP servers or settings carrying headers or env get a visible warning, because copying them copies that credential.
- **Consequences:** Updates to `~/.claude` don't flow into workspaces automatically. Re-import to refresh.

## ADR-017 — Project groups, and sessions nested under projects

- **Status:** Accepted (2026-10-20, chosen by Mike)
- **Context:** The sidebar had three flat lists (Workspaces, Projects, Sessions), so nothing showed which project a session ran in. "Home" (no session selected) and "No project (home folder)" (start in `$HOME`) sounded alike but meant different things. Lauf also mixes client sites with studio and personal work under one identity.
- **Decision:**
  - The sidebar goes workspace switcher → Overview and Knowledge → groups → projects → sessions → General. Each session is listed under the project it runs in. "Home" became **Overview**, and "No project" became **General**.
  - Groups live **inside** a workspace (migration 0004, `project_groups`). They aren't extra workspaces, because Studio and Personal work uses the same `mhlauf1` identity. Lauf starts with Clients, Studio and Personal; Playbook starts with none.
  - The database enforces isolation itself. `repos (group_id, workspace_id)` is a composite foreign key to `project_groups (id, workspace_id)`, so a project can't sit in another workspace's group even if Otto had a bug. Every store method also filters by workspace. The isolation suite covers both (`project_groups_never_cross_workspaces`).
  - Session titles drop the project ("Claude Code", not "Claude Code · stoc") because the parent row already shows it. Repeats are numbered ("Claude Code 2").
  - Sidebar layout choices (selected project, collapsed rows, current page) persist in the webview's local storage. They are layout only, never identity.
  - Projects can have a name Mike chooses ("Brady Digital Consulting"), stored as `repos.display_name` (migration 0005). Only Otto's label changes; the folder, the repo and isolation checks still use the path.
- **Consequences:** The tree gets most useful once M5 keeps sessions across relaunches. Deleting a group keeps its projects, ungrouped.

## ADR-018 — One input: the composer is removed

- **Status:** Accepted (2026-10-20, chosen by Mike)
- **Context:** The on-demand composer (⌘L) was only a bigger text box that pasted into the agent's own `❯` prompt. Two places to type looked like two chats, and Mike found it confusing.
- **Decision:** Remove the composer (`src/composer/`, the title-bar button, ⌘L). The terminal is the only input, and Wispr Flow dictates straight into it.
- **Consequences:** The queue (M5) needs its own entry point, which replaces the old ⌥↵ plan. A dictation helper is in the parking lot in case dictating into the terminal proves painful.

## ADR-019 — Resume earlier Claude conversations (pulled forward from M5)

- **Status:** Accepted (2026-10-20, asked for by Mike)
- **Context:** Otto keeps sessions only in memory, so relaunching Otto ended Mike's stoc session and the sidebar forgot it. Claude Code had already saved the whole conversation inside the Lauf workspace's own config dir.
- **Decision:**
  - Otto reads Claude's own record, `<workspace>/claude/projects/<encoded folder>/<id>.jsonl`, rather than keeping a copy. Each project (and General) lists its earlier conversations under its live sessions, newest first, titled with Claude's AI title or the first prompt.
  - Clicking one runs `claude --resume <id>` in that folder, inside the workspace. The same conversation continues; Claude only forks when asked.
  - Isolation: history comes only from that workspace's `CLAUDE_CONFIG_DIR`, for a project registered to it. A resume ID must be a UUID that appears in that list, and an ID that's already open is refused. Tested in `past_conversations_never_cross_workspaces`.
- **Consequences:** Codex history and scrollback restore stay in M5. Sessions are still in memory, but nothing is lost when Otto quits, because each conversation is one click away.

## ADR-020 — Otto Dev and Otto.app keep separate data

- **Status:** Accepted (2026-10-20, chosen by Mike)
- **Context:** The dev build and the installed app shared `~/Library/Application Support/Otto`. Running both at once clashed. A dev migration (such as 0004) left the older installed app unable to open the database. And when a Rust change restarted the dev window, it closed Mike's working terminals. Updating the installed app meant hand-typing `rm -rf` and `ditto`.
- **Decision:**
  - Debug builds (`pnpm dev`) use `Otto Dev` as the data folder, and `tauri.dev.conf.json` gives them their own name and bundle ID (`co.lauf.otto.dev`). The sidebar shows a "Dev" tag.
  - Release builds keep `Otto` and `co.lauf.otto`.
  - `pnpm app:install` builds only the `.app` (skipping the `.dmg`), quits the installed Otto through AppleScript, swaps the bundle and reopens it.
- **Consequences:** Otto Dev has its own workspaces, logins, projects and history, so Mike signs in once there. Isolation is unchanged: each build still isolates its workspaces from each other. Self-updating (the Tauri updater) waits for a signed build in M6.

## ADR-021 — Knowledge reaches agents through each workspace's own instruction files

- **Status:** Accepted (2026-10-20, approved by Mike: index rather than every note, About Mike shared)
- **Context:** The original SPEC §7 had Otto edit each repo's `CLAUDE.md` and `AGENTS.md`. That would have put Mike's personal and company context into client repos and their git history. Mike also wants far more than one context file: company history, architecture, clients, private notes like equity, diagrams and assets.
- **Decision:**
  - Knowledge lives in `<Otto data>/knowledge/`: `global/About Mike.md`, plus `notes/`, `private/` and `assets/` for each workspace (SPEC §7.1).
  - Otto **generates** each workspace's global instruction files: `claude/CLAUDE.md` (read from `CLAUDE_CONFIG_DIR`) and `codex/AGENTS.md` (read from `CODEX_HOME`). Each holds Mike's own instructions, About Mike, the workspace's Overview, and an index of its other notes (title, summary, path). The files carry a managed marker. A hand-written file found there is moved to `CLAUDE.user.md` / `AGENTS.user.md` first, and the Claude-setup import now writes to `CLAUDE.user.md`.
  - The files are rebuilt at launch, whenever a note is saved, before every session opens, and after a setup import.
  - Claude sessions get `--add-dir` for their own `notes/` and the global folder only. Ask runs in `notes/`, adds the workspace's projects, and adds `private/` only when Mike ticks "Include private".
  - Every path from the UI is relative and is resolved inside one area. `..`, absolute paths and symlinks that leave the area are refused. Project docs can only be read if they appear in that project's doc list.
  - Images go to the UI as `data:` URLs, so the strict CSP still allows no file access. Links in notes don't navigate the window.
- **Consequences:** Repos are never touched. Each session pays for one small index, not every note. The isolation suite gains `knowledge_feeds_each_workspace_only_its_own_notes`.

## ADR-022 — Sessions survive restarts; agents report their own status

- **Status:** Accepted (2026-10-20, Mike chose paused + resume on click)
- **Context:** Sessions lived only in memory, so a relaunch lost them from the sidebar. Telling "working" from "needs you" by reading terminal text would be fragile and would break with every CLI release.
- **Decision:**
  - A `sessions` table (migration 0006) saves work sessions. A session is closed only when Mike closes it or its program exits by itself. A `quitting` flag, set on `ExitRequested`, keeps Otto's own shutdown from counting as every session ending.
  - Resume runs each CLI's own resume (`claude --resume`, `codex resume`). A never-used Claude session restarts with its same ID.
  - Status comes from the agents' own extension points: Claude hooks (`UserPromptSubmit`, `Stop`, `Notification`) passed through `--settings`, and Codex's `notify` passed through `-c`. Mike's settings files are never edited. Both call one per-workspace script. The script checks `OTTO_SESSION` is a UUID, writes only to that workspace's `inbox/`, and never fails the agent. A watcher drains the inboxes every 400 ms and emits a typed `SessionStatusChanged` event.
  - Scrollback is kept for Shell only, in Rust (so it's saved at quit without the window's help). It's capped at 256 KB and written atomically.
- **Consequences:** The isolation suite gains `agent_events_only_update_their_own_workspace`. Codex sessions resume only after Codex has reported a thread (after its first turn). Notifications use `tauri-plugin-notification` 2.5.1.

## ADR-023 — One Claude install; workspaces never update it

- **Status:** Accepted (2026-10-20)
- **Context:** Measured on Mike's Mac, the Otto data folder was 1.5 GB, almost none of it Mike's content. Claude's auto-updater, run inside each workspace, had downloaded its own copy into the workspace's XDG data folder (519 MB each), and had re-pointed `~/.local/bin/claude` into Otto's Lauf folder. Mike's normal terminal was running Claude from Otto's data.
- **Decision:**
  - `DISABLE_AUTOUPDATER=1` in every workspace environment. Mike's own terminal keeps Claude updated, and Otto uses that install.
  - At launch, `housekeeping::prune_stray_claude_versions` removes a workspace's downloads, **unless** the `claude` on the PATH resolves into them.
  - The Identity Doctor warns ("Claude install") when `claude` runs from Otto's data, and suggests running `claude install` in a normal terminal.
- **Consequences:** Once Mike runs `claude install`, about 1 GB is freed on the next launch. Codex's own package cache (325 MB) is left alone, because Codex manages it.
