# Otto — agent instructions

Otto is a native macOS app (Tauri 2 + React 19/TypeScript + Rust) that keeps Mike's Playbook and Lauf identities signed in side by side, fully isolated. The full spec is `docs/SPEC.md`; read it before any non-trivial change.

`CLAUDE.md` imports this file, so this file is the single source of agent instructions. Edit here, never there.

## Rules (from SPEC §0, in priority order)

1. **Workspace isolation is sacred.** No change may let one workspace commit, push, authenticate or read context as another. Isolation beats every feature and deadline (SPEC §4, `docs/SECURITY.md`).
2. **Design is king.** Visual references come only from the Mobbin MCP and are recorded in `docs/DESIGN.md`. System font only (no serif), no orange or terracotta, no generic "AI app" look. Every colour, spacing, radius, shadow and motion value comes from `src/design/tokens.ts`.
3. **Quality over speed.** Strict types, tests, small reviewed milestones. Never stub something and call it done.
4. **Teach as you go.** Every new concept gets 2–3 plain-English sentences in chat and an entry in `docs/LEARNING.md`.

## Workflow

- One milestone at a time (SPEC §11). At the end of each one: stop, summarize and ask Mike to verify.
- At the end of **every** working session, update `docs/PROGRESS.md` and any affected living file (`DECISIONS`, `LEARNING`, `DESIGN`, `SECURITY`, `PARKING_LOT`).
- Anything Mike mentions that isn't in V1 goes in `docs/PARKING_LOT.md`, never into code.
- `docs/SPEC.md` changes only with Mike's approval, and every change is recorded in its changelog.
- Conventional commits (`feat:`, `fix:`, `docs:`, `chore:`, `test:`, `refactor:`), small PRs.
- This repo belongs to the **Lauf** identity (`mhlauf1` / `mhlauf1@gmail.com`). Never commit here as Playbook.

## Code standards

- TypeScript: strict mode, no `any`, no non-null assertions to silence errors.
- Rust: no `unwrap()`/`expect()` outside tests (`clippy::unwrap_used` is denied). Errors are typed enums (`thiserror`) and shown to Mike in friendly language.
- IPC is typed end to end. Rust commands export TypeScript via `tauri-specta` into `src/bindings.ts` (generated, never hand-edited).
- Styling: CSS Modules that read CSS variables generated from `tokens.ts`. Stylelint rejects hard-coded colours and spacing.
- Logging: `tracing`, local only, redacted, no telemetry.
- Isolation logic lives in `crates/otto-core` (no Tauri dependency) so it can be tested headless.

## Commands

Requires Node from `.nvmrc` (`nvm use`), pnpm 10 and Rust from `rust-toolchain.toml`.

| Command                       | What it does                                                           |
| ----------------------------- | ---------------------------------------------------------------------- |
| `pnpm install`                | Install JS dependencies (exact versions, frozen in CI)                 |
| `pnpm dev`                    | Run Otto (Vite + Tauri). Regenerates `src/bindings.ts` on launch       |
| `pnpm dev:web`                | Browser-only preview at http://localhost:1420 (no Rust core)           |
| `pnpm check`                  | Everything CI runs: lint, typecheck, unit tests, Rust fmt/clippy/tests |
| `pnpm test` / `pnpm test:e2e` | Vitest / Playwright (WebKit), with screenshots in `test-results/`      |
| `pnpm tokens`                 | Regenerate `src/design/tokens.css` from `tokens.ts`                    |
| `pnpm bindings`               | Regenerate `src/bindings.ts` from the Rust commands                    |
| `pnpm icons`                  | Regenerate app icons from `src/assets/brand/app-icon.svg`              |
| `pnpm build`                  | Release `.app` and `.dmg`                                              |

Generated files are committed and checked by tests: `src/design/tokens.css`, `src/bindings.ts`.
