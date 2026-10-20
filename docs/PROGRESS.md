# Progress

Newest entries first.

## Current milestone: M0 — Foundations

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

#### 2026-10-20

- Read SPEC end to end and agreed the M0 plan with Mike.
- Decisions: CSS Modules + CSS variables for styling; repo under `mhlauf1` (Lauf identity); isolation credential research happens during M0.
- Mike asked for an Otto brand identity (logo, branding) as part of M0.
- Created the living files.

### Blockers / notes

- Rust is not installed yet, and the Codex CLI is missing (needed by M2).
- gh 2.48 is old. Suggest `brew upgrade gh`.
- This Claude Code session runs on the Playbook account. Otto is a Lauf project, so consider continuing under the Lauf Claude account.
