# Progress

Newest entries first.

## Current milestone: M5b — Up next and parallel tasks (built, awaiting Mike's check)

### Definition of done

- [x] Mike chose the "Up next" panel and worktrees inside Otto's data folder. SPEC §8.2, ADR-024.
- [x] Queue (migration 0007): add with ↵, remove with ×. Sent automatically when the agent reports it finished, never while working or waiting, never twice (`claim_idle`).
- [x] Parallel: "Run in parallel" or ⌘↵ makes an `otto/<task>` worktree and starts the agent there with the prompt. Shown under the project with its branch. Resumable. Remove from the project's ··· menu (refused when dirty or in use; the branch is kept).
- [x] Tests: 64 core, 30 isolation (new: worktree identity, guardrails and removal against real git; the queue is workspace-scoped), 25 unit (4 for Up next), 4 e2e.
- [ ] Mike: queue two prompts behind a running Claude task and watch them go in turn. Try "Run in parallel" in a project.

## Previous milestone: M5a — Sessions survive restarts ✅ (PR #3)

### Definition of done

- [x] SPEC §8 split into 8.1 and 8.2, M5 split into 5a and 5b (Mike's approval). ADR-022 and ADR-023.
- [x] `sessions` table (0006). Open sessions come back paused; click to resume (Claude `--resume`, Codex `codex resume`, Shell with scrollback); × to forget.
- [x] Agent-reported status (Claude hooks with `--settings`, Codex `notify` with `-c`) through per-workspace inboxes. Working, Needs you and Ended on rows. macOS notifications.
- [x] Shell scrollback saved (256 KB cap, every 10 s and at quit) and restored.
- [x] Housekeeping: Claude's auto-updater off in workspaces; stray Claude downloads pruned unless in use; Identity Doctor "Claude install" warning; Doctor logs only changes; closed sessions forgotten after 30 days.
- [x] Tests: 61 core, 29 isolation (new: `agent_events_only_update_their_own_workspace`, `doctor_warns_when_claude_runs_from_otto_s_data_folder`), 21 unit, 4 e2e. Verified against the real CLIs: Codex accepts `-c notify`, Claude accepts `--settings`, and the running app drained an inbox event.
- [ ] Mike: run `claude install` in a normal Terminal (fixes the Identity Doctor warning and frees about 1 GB on the next launch). Then in Otto Dev, start a Claude session, quit Otto, reopen, and resume it.

## Previous milestone: M4 — Knowledge ✅ (PR #2)

### Definition of done

- [x] SPEC §7 rewritten with Mike's approval (index of notes, About Mike shared). ADR-021.
- [x] Knowledge storage, plus starter notes for Playbook (Company with Lula Fit → Playbook and Mike since March 2020; Architecture; People; Glossary; private Equity) and Lauf (Studio, Clients). Never overwritten.
- [x] Generated `CLAUDE.md` and `AGENTS.md` per workspace: Mike's own instructions, About Mike, Overview and the note index. Rebuilt at launch, on save, before each session and after setup import. Hand-written files are kept as `*.user.md`.
- [x] Claude sessions can open their own notes and About Mike (`--add-dir`). Ask covers notes, projects and, optionally, private.
- [x] Knowledge view: list, Read/Edit with autosave, Markdown, Mermaid, images, project docs (read-only), assets.
- [x] Isolation: `knowledge_feeds_each_workspace_only_its_own_notes` (27 tests). 11 knowledge unit tests, including path escapes and symlinks.
- [ ] Mike tries it: fill in Playbook's Company note, open a new Playbook Claude session and ask what it knows about Playbook, then check a Lauf session doesn't know it. Try Ask.

## Previous milestone: M3.5 — Shape and labels ✅ (PR #1)

Planned with Mike in the 2026-10-20 "where next" session: Phase A (this milestone), then Phase B Knowledge (M4, expanded), then M5 and M6.

### Definition of done

- [x] One set of words (SPEC §2.1), also shown on the Overview "What's what" card.
- [x] Project groups inside a workspace (migration 0004). Lauf starts with Clients, Studio and Personal. Create, rename, reorder, delete, and move projects between groups.
- [x] Isolation: a composite foreign key plus workspace-filtered store methods, so no cross-workspace group moves. New isolation test (25 total).
- [x] Sidebar: segmented workspace switcher, Overview and Knowledge, groups → projects → sessions, General last, "+" on a project row opens a session there.
- [x] Overview page and a Knowledge placeholder. The title bar reads workspace · project · session.
- [x] Composer removed; the terminal is the only input (ADR-018).
- [x] Selected project, page and collapsed rows remembered across launches.
- [x] Mobbin references (5) in DESIGN.md. Light and dark screenshots, one QA fix.
- [x] Earlier Claude conversations listed under each project and resumable with one click (ADR-019, pulled forward from M5). New isolation test (26 total).
- [x] Mike's first-look fixes: obvious add rows, "+" per group, an "Add a project" row in empty groups, rename projects (migration 0005).
- [x] Otto Dev (own data folder and bundle ID, "Dev" tag) and `pnpm app:install` (ADR-020).
- [ ] Mike tries it in the app (`pnpm dev`): put stoc under Clients, start a session from its row, and check it nests there.

## Previous milestone: M3 — Shell and design pass ✅

### Definition of done

- [x] Mobbin references (7) recorded in DESIGN.md before building.
- [x] Command palette ⌘K: new session, switch workspace, jump to session, run Identity Doctor. "Open context" arrives with M4.
- [x] Dictation-friendly composer: ⌘↵ sends as bracketed paste, esc to terminal, ⌘L focus. ⌥↵ arrives with the queue (M5).
- [x] Sidebar identity footer; all values from tokens.
- [x] Light and dark screenshots with a QA comparison note; two issues found and fixed.
- [x] Tests: 10 unit, 3 e2e (palette included), Rust and isolation suites unchanged and green.

## Previous milestone: M2 — Terminal sessions ✅ (Mike signed in to all four accounts, all green)

### Definition of done

- [x] PTY sessions in `otto-core`, spawned only from `IsolatedEnv` and covered by 2 new isolation tests (21 total).
- [x] xterm.js with WebGL and fit, themed from tokens; scrollback survives switches.
- [x] Claude (with a fixed `--session-id`), Codex and Shell sessions; ⌘T new Claude session, ⌘W close.
- [x] Sign-in flows: Identity Doctor "Sign in" buttons run `claude auth login` / `codex login` inside the workspace.
- [x] Codex CLI 0.163 installed (Node 24, nvm).
- [x] Mike signed in to Claude and Codex in both workspaces. Identity Doctor is fully green (verified in the audit log, 2026-10-20 13:23).

## Previous milestone: M1 — Isolation core ✅ (CI green)

### Definition of done

- [x] Workspace model with injection-proof validation (`workspace.rs`).
- [x] `IsolatedEnv`: `env_clear()` plus the ADR-008 allowlist; forbidden keys can't be set.
- [x] Per-workspace directories (0700), managed gitconfig, hooks, askpass and the Codex file-storage setting (`provision.rs`).
- [x] `pre-commit`/`pre-push` hooks: fail closed, check configured and effective URLs, check every outgoing commit, gated by the Doctor.
- [x] Repo registration rules (`repo.rs`) plus SQLite store with migrations and an append-only audit log.
- [x] Identity Doctor (gh, git, ssh, Claude, Codex, hooks, rc-file secrets), run on launch, on switch, and on demand with ⌘⇧I.
- [x] Isolation suite: 19 tests, offline, real git and gh, and a dedicated CI job.
- [x] Real logins set up: Playbook = `mikelauf`, Lauf = `mhlauf1`, each isolated with `--insecure-storage`.
- [x] CI green on GitHub, including the isolation job.

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
- [x] First push (done once the `mhlauf1` login existed; CI is green on GitHub).

**Not done on purpose:** there's no placeholder `isolation-suite` CI job. A job that passes with zero tests would claim a guarantee we don't have yet (SPEC §0.3). M1 adds the real suite as a required check.

### Log

#### 2026-10-20 (M6 parked)

- Mike parked M6 (polish and ship) to start building **Lauf Studio**, a separate Lauf project (`~/Desktop/lauf/dev/lauf-studio`): a section library plus a site builder that exports Next.js sites and ships them to GitHub and Vercel. It runs inside Otto's Lauf workspace, so its GitHub and Vercel steps inherit Lauf's identity and guardrails. Otto's code doesn't change for it.
- M5 (a and b) is complete. M6 stays next for Otto whenever Mike picks it back up. Onboarding extras (owner-list editing, SSH setup, new workspaces) were declined for now.

#### 2026-10-20 (M5b)

- Installed the build with "From your projects" removed.
- Built M5b (ADR-024). This completes SPEC §8. Next: M6 (polish and ship: performance budgets, accessibility, onboarding, signed build).

#### 2026-10-20 (M5a)

- Mike asked how Otto is built, where data lives, whether it bloats and what it costs. Measured: the database is 128 KB, but the data folder was 1.5 GB because each workspace had its own Claude download, and Claude's updater had re-pointed `~/.local/bin/claude` into Otto's Lauf folder. Fixed going forward (ADR-023). Mike needs to run `claude install` once.
- Built M5a (ADR-022). New dependency: `tauri-plugin-notification` 2.5.1, at exact versions.
- Mike moved back to the installed Otto.app (`pnpm app:install`). The database upgraded to 0006 with all 11 projects kept. Playbook's stray Claude copy was pruned (1.5 GB → 1.2 GB); Lauf's waits until Mike runs `claude install`.
- Mike asked to remove "From your projects" from Knowledge. Removed end to end (SPEC §7.3, ADR-021 amendment).
- Next: Mike checks M5a, then M5b (queue and worktrees).

#### 2026-10-20 (M4 Knowledge)

- Mike approved the §7 rewrite: an index of notes, and About Mike shared by every workspace.
- Built storage, generated instruction files, project docs, Ask, the Knowledge view, and the isolation test.
- New dependencies, at exact versions: react-markdown 10.1.0, remark-gfm 4.0.1, mermaid 12.1.0 (loaded only when a note has a diagram).
- Next: Mike checks M4 in Otto Dev. Then M5: persistence (sessions across relaunch, scrollback, the queue, worktrees and notifications).

#### 2026-10-20 (where next, and M3.5)

- Mike reviewed Otto after using it for real: he worked on stoc in a Lauf session and merged and deployed with no sign-in. He asked for a step back on structure, labels, knowledge and the centre pane.
- Decisions:
  - Sessions nest under projects.
  - The terminal stays in the centre; a chat view is parked.
  - Lauf gets groups inside one identity.
  - Knowledge covers notes for agents, the repo-docs index, private notes and an "Ask" session, plus company and code understanding, diagrams and assets.
  - Compose is removed.
- Built Phase A (M3.5), described above. ADR-017 and ADR-018. SPEC §2.1, §3, §5.3, §6.3, §11 and the changelog updated with Mike's approval of the plan.
- Mike reported that the changes weren't showing and his stoc session was gone. Causes: he was running the old installed Otto.app, and relaunching Otto ends in-memory sessions. The conversation was safe in the Lauf Claude config. Built resume (ADR-019) and rebuilt Otto.app.
- Fixed a bug: "+" on General could open the session in the last-selected project. Every caller now passes the folder explicitly.
- Next: Mike checks M3.5. Then Phase B, M4 Knowledge: rewrite SPEC §7 first (feed agents through each workspace's own `CLAUDE.md` and `AGENTS.md`, never through repo files), as ADR-021.
- Still open: revoke the tokens found in the port-v1 and cadence-private-capital remotes.

#### 2026-10-20 (hardening for daily use)

- **Security:** found GitHub tokens embedded in two repo remotes (port-v1, cadence-private-capital). port-v1 was stripped with Mike's OK; **cadence-private-capital is awaiting Mike**. Both tokens need revoking. Otto now refuses or blocks credential-bearing config and push URLs (G10).
- Playbook org `lulafit` added (migration 0002 plus defaults; a test keeps them in sync).
- Projects: auto-discover plus confirm, local-only repos, sessions open in the project (ADR-015).
- Claude setup import picker (ADR-016).
- Next: build Otto.app so it doesn't depend on the dev session; then M4 context.

#### 2026-10-20 (M3 revision)

- Mike tried M3 and found two inputs confusing. The composer is now on demand (⌘L) with ↵ to send. "+ New session" is a single menu. SPEC §6.3 updated with Mike's approval.
- Gap noted: sessions open in `~`. They need a project picker (repo registration UI), to be pulled forward before or alongside M4.

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

- gh 2.48 is old. Suggest `brew upgrade gh`.
- This Claude Code session runs on the Playbook account. Otto is a Lauf project, so consider continuing under the Lauf Claude account.
