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
