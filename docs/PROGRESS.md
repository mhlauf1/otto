# Progress

Newest entries first.

## Current milestone: M1 — Isolation core ✅ (awaiting Mike's demo)

### Definition of done

- [x] Workspace model with injection-proof validation (`workspace.rs`).
- [x] `IsolatedEnv`: `env_clear()` plus the ADR-008 allowlist; forbidden keys can't be set.
- [x] Per-workspace directories (0700), managed gitconfig, hooks, askpass and the Codex file-storage setting (`provision.rs`).
- [x] `pre-commit`/`pre-push` hooks: fail closed, check configured and effective URLs, check every outgoing commit, gated by the Doctor.
- [x] Repo registration rules (`repo.rs`) plus SQLite store with migrations and an append-only audit log.
- [x] Identity Doctor (gh, git, ssh, Claude, Codex, hooks, rc-file secrets), run on launch, on switch, and on demand with ⌘⇧I.
- [x] Isolation suite: 19 tests, offline, real git and gh, and a dedicated CI job.
- [x] Real logins set up: Playbook = `mikelauf`, Lauf = `mhlauf1`, each isolated with `--insecure-storage`.
- [ ] Demo to Mike.

**Known gaps (by design, later milestones):** SSH key generation and onboarding (M6); Claude and Codex sign-in flows (M2); repo picker UI (M3); the Playbook GitHub **org** isn't on the Playbook allowlist yet, so Mike needs to supply its name.

## Previous milestone: M0 — Foundations

### Definition of done

- [x] Living files created and filled (`AGENTS.md`/`CLAUDE.md`, `PROGRESS`, `DECISIONS`, `LEARNING`, `DESIGN`, `PARKING_LOT`, `SECURITY`).
- [x] Rust toolchain pinned (1.99.1). Node 24.22 pinned via `.nvmrc`. pnpm lockfile with exact versions.
- [x] Cargo workspace: `crates/otto-core` (no Tauri) plus a thin `src-tauri` shell.
- [x] Typed IPC with `tauri-specta`: an `app_info` command, with generated bindings and a staleness test.
- [x] Otto brand: mark, wordmark, lockup, app icon and brand notes in `DESIGN.md`, all from Mobbin references.
- [x] `src/design/tokens.ts` → generated `tokens.css`, with a staleness test, Stylelint enforcement and an orange-hue guard.
- [x] Hello window: overlay title bar, vibrancy sidebar, light and dark, workspace tint, ⌘1/⌘2.
- [x] ESLint (strict type-checked), Prettier, Stylelint, Vitest (6 tests), Playwright WebKit smoke test (2 tests), Clippy pedantic with `-D warnings`, rustfmt, Rust tests (3).
- [x] GitHub Actions CI (macOS, SHA-pinned actions). It runs only once the repo is pushed to GitHub.
- [x] ADR-005 to ADR-008 for isolation (credential storage, SSH, hooks, env allowlist).
- [x] `LEARNING.md` chapter 0.
- [x] Mike reviewed the app and approved the brand (2026-10-20). The `mhlauf1/otto` repo has been created.
- [ ] First push. This is blocked until a Lauf (`mhlauf1`) GitHub login exists; the only credential on this Mac is `mikelauf` (Playbook).

**Not done on purpose:** there's no placeholder `isolation-suite` CI job. A job that passes with zero tests would claim a guarantee we don't have yet (SPEC §0.3). M1 adds the real suite as a required check.

### Log

#### 2026-10-20 (M1)

- Built the isolation core in `otto-core`, wired it into Tauri, and added the Identity Doctor inspector.
- The tests found a real bug: git falls back to SSH_ASKPASS for HTTPS passwords, which could have leaked an SSH passphrase. Fixed (ADR-012).
- Judgment call: only identity checks gate pushes (ADR-011).
- Mike's default `~/.config/gh` was briefly switched to mhlauf1 by a mistyped login. Otto doesn't use it, but Mike may want to run `gh auth login` as mikelauf for normal terminals.

#### 2026-10-20

- Read SPEC end to end and agreed the M0 plan with Mike.
- Decisions: CSS Modules + CSS variables for styling; repo under `mhlauf1` (Lauf identity); isolation credential research happens during M0.
- Mike asked for an Otto brand identity (logo, branding) as part of M0.
- Created the living files.

### Blockers / notes

- Rust is not installed yet, and the Codex CLI is missing (needed by M2).
- gh 2.48 is old. Suggest `brew upgrade gh`.
- This Claude Code session runs on the Playbook account. Otto is a Lauf project, so consider continuing under the Lauf Claude account.
