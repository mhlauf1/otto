# Otto — V1 Product & Engineering Spec

Oct 20, 2026 · @Mike · Owner: Mike (Lauf / Playbook)

> **How to use this doc:** Save it as `docs/SPEC.md` in an empty repo called `otto`, open Claude Code in that folder, and say: *"Read docs/SPEC.md end to end. Follow Section 0 exactly, then begin Milestone 0."*

---

## 0. Instructions to the build agent (read first)

You are the lead engineer on Otto. Mike is a web developer (React and TypeScript) who is new to desktop apps, Tauri and Rust. He wants a professional, production-grade app, not a prototype, and he wants to **learn the system as it's built**.

Non-negotiables, in priority order:

1. **Workspace isolation is sacred.** A Playbook session must never be able to commit, push, authenticate or read context as Lauf, and the reverse. This takes precedence over every feature and every deadline (see Section 4).
2. **Design is king.** Visual references come **only** from the Mobbin MCP. No serif fonts. No orange. No generic "AI app" aesthetic (see Section 6).
3. **Quality over speed.** Use strict typing, tests, and small reviewed milestones. Never stub something and call it done.
4. **Teach as you go.** Explain each architectural decision in plain language and log it (see Section 9).

Before writing any app code, create the living files listed in Section 9 and keep them up to date at the end of **every** working session. Work one milestone at a time (Section 11). At the end of each milestone, stop, summarize what was built, and ask Mike to verify before moving on.

---

## 1. Problem

Mike works across separate identities:

- **Playbook** (his 9-to-5): a Claude Code Pro account, a Codex/ChatGPT Pro account, and a Playbook GitHub account.
- **Lauf** (lauf.co, his design and development studio, plus personal work): a personal Claude account and a personal GitHub account.

Today he constantly signs in and out (`claude` login, `gh auth login`, switching git emails and terminals). Context gets lost, usage on one account sits idle, and there is a real risk of pushing work under the wrong identity.

## 2. Vision

**Otto is a calm, beautiful Mac app Mike opens every morning.** Every account is always signed in, each with its own context, usage and GitHub identity. He dictates (with Wispr Flow) into whichever agent he's focused on. Sessions from last night are exactly where he left them, and agents can run side by side or in a queue.

It is also a public portfolio piece: proof that Mike is a design engineer.

### 2.1 Words Otto uses

Every screen uses these words with these meanings, and the Overview page shows them.

- **Workspace:** one identity: its logins, git author and GitHub account. `Playbook` or `Lauf`. Nothing crosses between workspaces (Section 4).
- **Project:** a repo or folder registered to exactly one workspace (§4.2). Projects can sit in a **group**, such as Lauf's Clients, Studio and Personal. A group belongs to one workspace.
- **Session:** one running Claude Code, Codex or Shell, inside one project or in General. It always runs as its workspace.
- **General:** sessions with no project. They start in the home folder.
- **Overview:** a workspace's landing page: who you are there, what's running, and its projects.
- **Knowledge:** a workspace's notes and docs (Section 7).

## 3. V1 scope

### In scope

1. **Native macOS app** built on Tauri 2, with a React and TypeScript frontend and a Rust core.
2. **Workspaces:** `Playbook` and `Lauf` at launch, with a data model that supports adding more later.
3. **Agents per workspace:** Claude Code (both workspaces) and Codex CLI (both workspaces). Each runs as a real terminal (PTY) session with an isolated config directory, so each keeps its own login and usage.
4. **Per-workspace GitHub and git identity:** `gh` auth, git author name and email, SSH key and commit signing.
5. **Layered shared context:** a global "About Mike" layer plus a per-workspace layer, fed automatically to every agent.
6. **Persistent sessions:** tabs, scrollback and agent sessions are restored on relaunch using each CLI's resume support.
7. **Simple task queue:** queue prompts per agent to run in parallel or one after another.
8. **Text input built for dictation:** dictation (Wispr Flow) goes straight into the agent's own prompt in the terminal. There is one input, not two.
9. **Identity Doctor:** a live check of who each workspace is signed in as.
10. **Learning mode** and the living docs (Section 9).

### Out of scope for V1 (see Parking Lot, Section 12)

This covers scheduled or overnight runs, the creative daily pipelines, the regular claude.ai and ChatGPT chat apps, Windows and Linux, and cloud sync.

---

## 4. Workspace isolation (hard requirement)

**Goal:** it should be structurally impossible, not merely unlikely, for work done in one workspace to leave under another workspace's identity.

### 4.1 One isolated environment per workspace

Every process Otto spawns (agent, shell, git, gh) is started by the Rust core with `env_clear()`, followed by an explicit allowlist. It never inherits Mike's login shell environment.

Each workspace has a directory at `~/Library/Application Support/Otto/workspaces/<id>/` containing:

| Purpose | Variable / mechanism | Points to |
| --- | --- | --- |
| Claude Code login and settings | `CLAUDE_CONFIG_DIR` | `<ws>/claude/` (the Keychain entry is keyed to this dir) |
| Anthropic Console profiles | `ANTHROPIC_CONFIG_DIR` | `<ws>/anthropic/` |
| Claude scratch files | `CLAUDE_CODE_TMPDIR`, `TMPDIR` | `<ws>/tmp/` |
| Codex login and settings | `CODEX_HOME` | `<ws>/codex/`, with `cli_auth_credentials_store = "file"` |
| GitHub CLI auth | `GH_CONFIG_DIR` | `<ws>/gh/`. Login uses `--insecure-storage`, because gh's default Keychain storage is shared across config dirs |
| Git identity | `GIT_CONFIG_GLOBAL` | `<ws>/gitconfig` (user.name, user.email, `core.hooksPath`; a signing key only if signing is enabled) |
| Git author and committer | `GIT_AUTHOR_*`, `GIT_COMMITTER_*`, `GIT_TERMINAL_PROMPT=0` | the workspace identity (overrides any repo-local `user.*`) |
| Block system git config | `GIT_CONFIG_NOSYSTEM=1` | none |
| SSH key | `GIT_SSH_COMMAND` | `ssh -F none -i <ws>/ssh/id_ed25519 -o IdentitiesOnly=yes -o IdentityAgent=none -o UserKnownHostsFile=<ws>/ssh/known_hosts -o StrictHostKeyChecking=accept-new -o ControlMaster=no -o ControlPath=none`. `-F none` is needed because ssh ignores `$HOME` and would otherwise read the real `~/.ssh/config` |
| SSH passphrase | `SSH_ASKPASS`, `SSH_ASKPASS_REQUIRE=force` | Otto's askpass helper, which reads that workspace's Keychain item |
| Home-like scratch | `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME`, `XDG_CACHE_HOME` | `<ws>/xdg/…` |
| Shell startup files | `ZDOTDIR` | `<ws>/zsh/` |

`HOME` stays the real home: a fake one breaks CLI installers and toolchains and doesn't stop ssh anyway. The full allowlist is ADR-008 in `docs/DECISIONS.md`.

The following are **always stripped** from the environment: `GH_TOKEN`, `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`, `GH_HOST`, `GH_REPO`, `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`, `CLAUDE_CODE_USE_*`, `OPENAI_API_KEY`, `CODEX_API_KEY`, `SSH_AUTH_SOCK`, and any `GIT_*` variable not set by Otto.

The agent must verify every variable above against current official docs for Claude Code, Codex CLI, gh and git before relying on it, and record the findings in `DECISIONS.md`.

### 4.2 Repos belong to exactly one workspace

- Each repository is registered to a single workspace. A repo cannot be opened in a session belonging to a different workspace.
- Each workspace has an **allowlist of GitHub owners and orgs** (for example, the Playbook org for Playbook, and Mike's personal account for Lauf).
- On opening, Otto checks the repo's remotes against the allowlist. On a mismatch it refuses to open the repo and explains why.

### 4.3 Guardrails at commit and push time

Hooks are a **guardrail**, not the guarantee: agents can skip them with `--no-verify`. The guarantee is credential scoping, because a workspace holds no credential the other identity's repos would accept. The isolation tests must therefore pass **even with `--no-verify`**.

Otto sets `core.hooksPath` in each workspace's `gitconfig` to an Otto-owned hooks directory, so the hooks apply to every repo without writing into it. When a repo opens, Otto checks that no repo-local `core.hooksPath` overrides this.

- **pre-commit:** blocks the commit if the author email doesn't match the workspace's identity.
- **pre-push:** blocks the push if the remote URL's owner isn't on the workspace allowlist, or if any outgoing commit has a foreign author email.
- Hooks fail closed: if they can't determine the workspace, they block.

### 4.4 Identity Doctor

The Identity Doctor runs on launch, when you switch workspaces, and on demand. For each workspace it checks:

- `gh auth status` → the expected account
- `git config user.email` → the expected email
- `ssh -T git@github.com` using that workspace's key → the expected username
- whether Claude and Codex are logged in (via each CLI's status command)

It shows a green or red status per line. Anything red blocks pushes in that workspace until it's fixed.

### 4.5 Making the current workspace obvious

Each workspace has its own subtle tint and a clear name in the title bar, sidebar and composer. Mike should never have to wonder which identity he's in.

### 4.6 Isolation tests (required in CI)

These integration tests run against local bare repos and a fake `gh`:

1. Pushing a Lauf-owned remote from the Playbook workspace is **blocked**.
2. Committing with the wrong email is **blocked**.
3. Spawned processes contain **no** stripped variables.
4. Two workspaces running at the same time never read each other's config directories.

The build cannot ship if any of these tests fails.

### 4.7 Credentials

- Otto **never reads, copies or stores** Claude, OpenAI or GitHub tokens. Each official CLI manages its own login inside its isolated directory.
- Anything Otto itself needs to keep secret (for example, the SSH key passphrase) goes in the **macOS Keychain**.
- Mike should confirm Playbook's policy on running work credentials and code through a personal tool.

---

## 5. Architecture

### 5.1 Stack

- **Shell:** Tauri 2 (macOS only; minimum version macOS 14).
- **Core:** Rust (stable), using `tokio`, `portable-pty` for terminals, `serde`, `sqlx` with SQLite for app state, `keyring` or Security.framework for the Keychain, and `notify` for watching context files.
- **Frontend:** React 19, TypeScript in strict mode, Vite, TanStack Query, Zustand for UI state, `xterm.js` with the WebGL addon for terminals, and Radix primitives styled with our own tokens.
- **Quality tooling:** ESLint, Prettier, Clippy and rustfmt, with Vitest, Playwright and Rust unit and integration tests.
- **Packaging:** a signed and notarized `.dmg`, with Tauri's updater planned for later.

### 5.2 Process model

```
React UI  ⇄  Tauri IPC (typed commands + events)  ⇄  Rust core
                                                   ├─ WorkspaceManager (env builder, isolation)
                                                   ├─ SessionManager (PTYs, resume, scrollback)
                                                   ├─ QueueRunner (parallel / sequential tasks)
                                                   ├─ ContextService (layered context, file watch)
                                                   ├─ IdentityDoctor
                                                   └─ Store (SQLite)
```

- IPC is **typed end to end**: Rust types are exported to TypeScript (for example with `specta` or `ts-rs`), so there are no stringly-typed calls.
- Tauri capabilities are kept to the minimum: no general shell access from the frontend, a strict CSP, and no remote content.

### 5.3 Data model (SQLite)

- `workspaces`: id, name, tint, gh\_owner\_allowlist, git\_name, git\_email, created\_at
- `repos`: id, workspace\_id, path, remote\_url, verified\_at, group\_id
- `project_groups`: id, workspace\_id, name, position. A composite foreign key means a repo can only sit in a group of its own workspace.
- `agents`: id, workspace\_id, kind (claude | codex), config\_dir
- `sessions`: id, agent\_id, repo\_id, title, cli\_session\_id, status, last\_active\_at
- `tasks`: id, session\_id, prompt, mode (parallel | sequential), status, created\_at, finished\_at
- `events`: an append-only audit log of identity checks, blocked pushes, and so on

### 5.4 Performance budgets

| Measure | Target |
| --- | --- |
| Cold launch to interactive | under 800 ms |
| Switching workspaces | under 100 ms |
| Terminal input latency | under 16 ms |
| Memory when idle with 4 sessions | under 300 MB |
| Frames during animation | 60 fps, with no dropped frames when switching tabs |

These budgets are measured in CI where possible and reported at each milestone.

---

## 6. Design system — "Design is king"

### 6.1 Rules

- **References come only from the Mobbin MCP** (`search_screens`, `search_flows`, `search_sections`). Before designing any screen, search Mobbin, choose 3–5 references, and record them with links in `DESIGN.md` along with what was borrowed from each. No other inspiration sources.
- **Feel:** Apple's native Mac apps crossed with OpenAI's product polish. Think quiet, precise and spacious.
- **Type:** the system font only (SF Pro via `-apple-system`), with SF Mono for terminals and code. **No serif fonts anywhere.**
- **Colour:** a neutral greyscale base, with one restrained accent per workspace used sparingly. **No orange or terracotta** and no gradients unless a Mobbin reference justifies them.
- **Light and dark** modes following the system setting, both treated as first-class.
- Native Mac details: vibrancy and translucent sidebar, traffic-light inset, correct focus rings, respect for Reduce Motion, and full keyboard navigation.

### 6.2 Tokens

Define every value as a token in `src/design/tokens.ts`, with CSS variables generated from it. That covers spacing (4 px grid), radii, type scale, colours, shadows and motion. Hard-coded colours or spacing values in components are not allowed.

### 6.3 Core screens for V1

1. **Main window:** the sidebar runs top to bottom: workspace switcher, then Overview and Knowledge, then groups → projects → sessions, then General, then "+ New session". The selected session's terminal, or the workspace page, fills the centre. An optional inspector on the right shows identity (and context, from M4). The title bar always reads workspace · project · session.
2. **Command palette (⌘K):** new session, switch workspace, jump to session, go to Overview or Knowledge, run Identity Doctor, and open context.
3. **One input:** the terminal is the only place to type. There is no separate composer (removed 2026-10-20, ADR-018). The queue (M5) gets its own entry point, designed in M5.
4. **Onboarding:** create a workspace, sign in to each CLI inside its isolated environment, generate or import the SSH key, set the GitHub owner allowlist, and run Identity Doctor until it's green.
5. **Knowledge view:** read and edit the workspace's notes and About Mike, browse project docs, and open Ask (Section 7).
6. **Settings.**

### 6.4 Shortcuts

- ⌘1 / ⌘2: switch between workspaces
- ⌘T: new session
- ⌘W: close session (the agent keeps running if it's queued)
- ⌘K: command palette
- ⌘⇧I: Identity Doctor

### 6.5 Design QA

Every UI pull request includes light and dark screenshots plus a short note comparing them with the chosen Mobbin references.

---

## 7. Knowledge

Rewritten 2026-10-20 with Mike (ADR-021). It replaces the earlier "layered context" plan, which would have edited each repo's `CLAUDE.md` and put personal context into client repos and git.

### 7.1 What lives where

Everything stays on this Mac, in Otto's data folder, and never goes into a repo:

```
knowledge/
  global/About Mike.md          # shared by every workspace; nothing sensitive
  workspaces/<id>/
    notes/      # Markdown, images, Mermaid diagrams: agents can read these
    private/    # Markdown that agents never get unless Mike allows it
    assets/     # logos, screenshots, brand files
```

Starter notes are created once, as outlines, and are never overwritten:
- Playbook: `Overview`, `Company` (Lula Fit → Playbook; Mike since March 2020), `Architecture`, `People` and `Glossary`, plus `private/Equity`.
- Lauf: `Overview`, `Studio` and `Clients`.

### 7.2 How agents get it

- Otto writes each workspace's **own** global instruction files: `claude/CLAUDE.md` (Claude reads it from `CLAUDE_CONFIG_DIR`) and `codex/AGENTS.md` (Codex reads it from `CODEX_HOME`).
- Each file contains, in order:
  - Mike's imported Claude setup instructions (kept as `CLAUDE.user.md`)
  - About Mike
  - the workspace's `Overview` note
  - an **index** of the workspace's other notes: title, one-line summary and full path
- Full notes aren't pasted in. Agents open a note when they need it. Claude sessions get access to their own workspace's `notes/` and the global folder (`--add-dir`, with Claude's usual permission prompts), and nothing else.
- Repo files are never touched.
- The files are rebuilt at launch, whenever a note is saved, and after the Claude setup is imported.

### 7.3 Project docs stay in their repos

Knowledge doesn't list the projects' own Markdown files (removed at Mike's request, 2026-10-20, because it flooded the list). Agents already read a repo's `CLAUDE.md` and docs when working in it. To bring a doc into the conversation, Mike points the agent at the file, or uses Ask, which can open the workspace's projects (7.5).

### 7.4 The Knowledge view

A list (Shared, Notes, Private, Assets) sits beside a reader and editor. The reader shows Markdown with Mermaid diagrams and images, and the editor saves as Mike types. New notes are created from the list. The design comes from Mobbin references first.

### 7.5 Ask

"Ask" opens a normal Claude Code session for the workspace. It runs in the workspace's `notes/` folder, with access to the workspace's projects and the global folder, so it can research and draft notes. "Include private" adds `private/` for that session only.

### 7.6 Strict separation (tests)

- A workspace's generated `CLAUDE.md` and `AGENTS.md` never contain another workspace's notes or paths.
- `private/` never appears in a generated file.
- Note reads refuse paths outside the workspace's own knowledge folder, including `..` and symlinks that point out.
- Ask and `--add-dir` only ever list the workspace's own folders.

## 8. Sessions and queue

### 8.1 Sessions survive restarts (M5a)

- **Remembered:** every work session (not sign-in flows) is saved in `sessions`. It stays `open` until Mike closes it or its program exits by itself. When Otto quits, open sessions stay open and come back **paused**.
- **Paused, resume on click** (Mike, 2026-10-20). Paused sessions sit under their project with a hollow dot and "Paused", and can be forgotten with ×. Resuming re-verifies the project, then:
  - Claude runs `--resume <id>`, or starts fresh with the same ID if it was never used.
  - Codex runs `codex resume <thread>` once Codex has reported its thread.
  - Shell starts again with its saved scrollback.
  - Ask sessions get their same folders back.
- **Scrollback:** saved for Shell only (up to 256 KB each, every 10 s and at quit). Agents redraw their own history when resumed. It's deleted when the session closes.
- **Status from the agents themselves:** a per-workspace script that Claude (hooks through `--settings`) and Codex (`-c notify=…`) run when they start, finish or need Mike. It drops the event in that workspace's inbox, named by the session's `OTTO_SESSION`. Otto applies an event only to a session of that same workspace. Rows show Working, Needs you, Ended or Paused (never colour alone). A macOS notification fires when an agent finishes or needs Mike and he isn't looking at it.
- **Housekeeping:**
  - Closed sessions are forgotten after 30 days.
  - Identity Doctor results are logged only when they change.
  - Claude's auto-updater is off inside workspaces, so Mike's own install is the only one. Workspaces' stray Claude downloads are removed unless `claude` still runs from one, and the Identity Doctor warns when it does.

### 8.2 Queue and parallel work (M5b)

- **Up next** (Mike, 2026-10-20): a slim panel under the terminal of a Claude or Codex session.
  - "+ Up next" opens a small field: ↵ adds, ⇧↵ adds a line, ⌘↵ runs in parallel, and esc cancels.
  - Waiting prompts are listed and numbered, each with ×.
  - The panel holds only waiting prompts. The terminal stays the place to talk to the agent.
- **Sequential:** when the agent reports it finished (8.1), Otto sends the next prompt as a bracketed paste plus Return, after the agent's first 3 seconds.
  - Never while it works or needs Mike.
  - Never twice: one database update claims the idle session before anything is sent.
- **Parallel:** "Run in parallel" makes a git worktree of the project on a new `otto/<task>` branch, inside the workspace's own folder (Mike, 2026-10-20). It starts a new session there with the prompt as its first message.
  - The session sits under its project with its branch shown.
  - Worktrees are created and used as the workspace's identity, and are re-verified like projects.
  - The project's ··· menu lists them. **Remove** is refused while a session runs in one or it has uncommitted changes, and the branch is kept.

---

## 9. Living files the agent maintains

Create these at the start. Update them at the end of every session, and never let them go stale.

| File | Purpose |
| --- | --- |
| `docs/SPEC.md` | This document. It changes only with Mike's approval, and every change is recorded in its changelog. |
| `CLAUDE.md` + `AGENTS.md` | Short instructions for agents working on Otto itself: commands, conventions, and the rules in Section 0. Keep the two files consistent. |
| `docs/PROGRESS.md` | Current milestone, what's done, what's next, blockers. Newest entries first. |
| `docs/DECISIONS.md` | Architecture decision records: context, options, decision, consequences. |
| `docs/LEARNING.md` | **Learning mode.** Plain-English explanations for a React developer: what Tauri, Rust and PTYs are, how IPC works, why each piece exists, and what Mike should read next. Add a section per milestone. |
| `docs/DESIGN.md` | Mobbin references, tokens, screen notes and screenshots. |
| `docs/PARKING_LOT.md` | Ideas for later. Anything Mike mentions that isn't in V1 goes here, never into the code. |
| `docs/SECURITY.md` | Threat model, isolation guarantees and test coverage. |

The learning-mode rule: whenever you introduce a new concept (for example, a Rust ownership quirk or a Tauri capability), explain it in two or three sentences in chat **and** add it to `LEARNING.md`.

---

## 10. Engineering standards

- Use conventional commits with small pull requests, and run CI (lint, typecheck, tests, isolation suite) on every push.
- No `any` in TypeScript and no `unwrap()` in Rust outside tests. Errors are typed and shown to the user in friendly language.
- Logs are structured (`tracing`), kept locally and redacted, with no telemetry.
- Dependencies are pinned and audited (`cargo audit`, `pnpm audit`).
- Accessibility: full keyboard support, VoiceOver labels and sufficient contrast in both modes.

## 11. Milestones

0. **Foundations:** repo, tooling, CI, living files, a Tauri hello-window with design tokens, and the first `LEARNING.md` chapter.
1. **Isolation core:** WorkspaceManager, env builder, config directories, git hooks, Identity Doctor and the isolation test suite. *Nothing else starts until this is green.*
2. **Terminal sessions:** PTY plus xterm.js, Claude and Codex launched inside isolated environments, and sign-in flows.
3. **Shell and design pass:** sidebar, workspace switcher, command palette and composer, all built from Mobbin references.
3.5. **Shape and labels:** the words in §2.1, project groups, sessions nested under projects, the Overview page, a Knowledge placeholder, and removing the composer (ADR-017, ADR-018).
4. **Knowledge:** knowledge storage and starter notes, generated per-workspace `CLAUDE.md` and `AGENTS.md`, project docs, Ask, the Knowledge view and separation tests (Section 7).
5. **Persistence and queue**, in two parts:
   - **5a:** remembered sessions (paused, resume on click), Shell scrollback, agent-reported status and notifications, and housekeeping (§8.1).
   - **5b:** the task queue and worktrees (§8.2).
6. **Polish and ship:** performance budgets, accessibility, onboarding, and a signed and notarized build.

Each milestone has a written definition of done in `PROGRESS.md` and finishes with a demo for Mike.

---

## 12. Parking lot (V2 and later; do not build in V1)

- **Scheduled runs:** headless agent runs (`claude -p`, `codex exec`) triggered by launchd at set times, such as 8am. Missed jobs run when the Mac wakes. Includes a queue that runs when Otto launches.
- **Use-it-or-lose-it usage:** nudges and auto-queued work when an account has unused capacity.
- **Daily make-believe company:** generate a fictional brand each day, with a site, components and branding assets, building a portfolio library.
- **Component and section library:** a steady stream of polished components to post on Twitter/X.
- **Illustrator pipeline:** agent-driven marketing and branding assets.
- **Redesign pitch machine:** take a real business with a weak site, generate a new site from Lauf's section skeletons, deploy it to Vercel, and get a shareable link.
- **Importing past work:** ingest previous projects and chats into the context layers.
- More workspaces and accounts, plus usage dashboards across accounts.

## 13. Open questions for Mike

1. **Resolved:** Codex is set up for Lauf too. Both workspaces get Claude Code and Codex.
2. **Resolved:** GitHub accounts and emails are in the table below. Mike will add any extra orgs later.
3. Resolved: commit signing is not required in V1. It's an optional per-workspace toggle that adds GitHub's "Verified" badge, off by default and easy to turn on later.
4. Resolved: Mike confirmed that using Playbook credentials in Otto is fine.

### Workspace identities (confirmed with Mike)

| Workspace | GitHub account | Git email | Claude plan | Codex / ChatGPT plan |
| --- | --- | --- | --- | --- |
| Playbook | `mikelauf` | `michael.laufersweiler@playbookexp.com` | $100/mo | $100/mo Pro |
| Lauf (lauf.co + personal) | `mhlauf1` | `mhlauf1@gmail.com` | $20/mo | $100/mo Pro |

Section 4's isolation rules enforce this mapping. Only `mikelauf`, with the playbookexp.com email, can push to Playbook repos. Only `mhlauf1`, with the Gmail address, can push to Lauf or personal repos.

## Changelog

- 2026-10-20: First draft written with Mike (voice session).
- 2026-10-20: Approved by Mike after M0 isolation research (ADR-005 to ADR-008). Codex is in both workspaces (§3). §4.1 env table updated: Claude/Anthropic dirs, Codex file storage, gh `--insecure-storage`, explicit git author vars, hardened `GIT_SSH_COMMAND` with askpass, XDG/ZDOTDIR, HOME policy, and a wider strip list. §4.3: hooks are a guardrail, credential scoping is the guarantee, and `core.hooksPath` lives in the workspace gitconfig.
- 2026-10-20: Approved by Mike after trying M3: the composer is on demand (⌘L), ↵ sends and ⇧↵ adds a line (was ⌘↵). New sessions start from a single "+ New session" menu (Claude, Codex, Shell).
- 2026-10-20: Approved by Mike in the "where next" planning session. Added the §2.1 glossary and milestone 3.5 (Shape and labels). Project groups are in §5.3. §6.3 now describes the nested sidebar and the Overview page. The composer is removed and the terminal is the only input (§3 item 8, §6.3). The Section 7 Knowledge rewrite is planned for M4 and will be recorded when it lands.
- 2026-10-20: Asked for by Mike after his stoc session vanished on relaunch: Claude conversation resume is pulled forward from M5 (§8, ADR-019).
- 2026-10-20: Approved by Mike: Section 7 rewritten as Knowledge (ADR-021). Agents get an index of notes rather than every note, About Mike is shared by every workspace, and repo files are never edited. §6.3 item 5 and milestone 4 were updated to match.
- 2026-10-20: Approved by Mike: M5 is split into 5a and 5b, and sessions come back paused and resume on a click. §8 rewritten as 8.1 and 8.2, with ADR-022 and ADR-023.
- 2026-10-20: Asked for by Mike: removed "From your projects" from Knowledge (§7.3, §7.4, §7.6). Project docs stay in their repos; Ask can still open them.
- 2026-10-20: Approved by Mike for M5b: an "Up next" panel under the terminal, and worktrees inside Otto's data folder. §8.2 detailed, ADR-024.
