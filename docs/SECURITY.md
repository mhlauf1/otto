# Security

## Threat model (V1)

**Asset:** each workspace's identity — Claude, Codex and GitHub logins, the git author, the SSH key and the context layer.

**What we defend against.** Our own mistakes and our agents' mistakes, not a malicious local user:

1. Work leaving under the wrong identity (wrong commit author, push to the wrong owner, wrong `gh` account).
2. An agent in workspace A reading credentials, config or context belonging to workspace B.
3. Ambient credentials leaking in from Mike's login shell (`GH_TOKEN`, `SSH_AUTH_SOCK`, global git config and so on).
4. An agent bypassing guardrails (for example `git commit --no-verify`).

**Out of scope:** malware running as Mike, a compromised CLI binary, or physical access.

## Isolation guarantees (SPEC §4)

| #   | Guarantee                                             | Mechanism                                                               | Test                  |
| --- | ----------------------------------------------------- | ----------------------------------------------------------------------- | --------------------- |
| G1  | Spawned processes never inherit the login environment | `env_clear()` plus an allowlist in `otto-core`                          | isolation test 3 (M1) |
| G2  | Each CLI keeps its login in its workspace directory   | `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `GH_CONFIG_DIR` (see open questions) | isolation test 4 (M1) |
| G3  | Git identity comes only from the workspace            | `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_NOSYSTEM=1`                            | isolation test 2 (M1) |
| G4  | Pushes only reach allowlisted owners                  | managed `pre-push` hook plus a remote check when a repo opens           | isolation test 1 (M1) |
| G5  | Context layers never cross workspaces                 | ContextService                                                          | M4 separation tests   |

## Research findings (2026-10-20) → ADR-005 to ADR-008

| Area        | Isolates by default?                                              | Fix                                                                |
| ----------- | ----------------------------------------------------------------- | ------------------------------------------------------------------ |
| Claude Code | Yes. The Keychain entry is keyed per `CLAUDE_CONFIG_DIR` (tested) | Also set `ANTHROPIC_CONFIG_DIR` and `CLAUDE_CODE_TMPDIR`           |
| Codex       | Yes, with file storage                                            | Write `cli_auth_credentials_store = "file"`                        |
| gh          | **No.** It uses a shared Keychain entry plus an unkeyed fallback  | `gh auth login --insecure-storage` per workspace                   |
| git         | Yes (`GLOBAL` + `NOSYSTEM`, tested)                               | Also set the `GIT_AUTHOR_*`/`GIT_COMMITTER_*` env vars             |
| ssh         | **No.** It always reads the real `~/.ssh/config`                  | `-F none` plus explicit known_hosts and an askpass helper          |
| Hooks       | Can be skipped with `--no-verify`                                 | Credential scoping is the guarantee; tests push with `--no-verify` |

### Residual risks (tracked)

- **R1.** Agent shells source the user's rc files: Claude's shell snapshot reads `~/.zshrc`, and Codex runs a login shell. Mitigations: a `CLAUDE_CODE_SHELL` wrapper, plus an Identity Doctor warning when rc files export secrets.
- **R2.** Both GitHub accounts may be collaborators on each other's repos, which would make credentials alone insufficient. Mitigation: owner allowlist checks when a repo opens, before each task runs, and in `pre-push`.

## Dependency audit exceptions

| Date       | Advisory                                                                                                                                                     | Path                                        | Why accepted                                                                                                                             | Revisit            |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| 2026-10-20 | GHSA-vfj7-8cjw-p6xm (`braces`, high, stack-exhaustion DoS)                                                                                                   | dev only: `stylelint › micromatch › braces` | It's a build-time lint tool that only reads glob patterns we write, and it isn't shipped. No patched version exists. CI audits `--prod`. | When a patch ships |
| 2026-10-20 | `cargo audit` warnings: RUSTSEC-2024-0436 (`paste`, unmaintained), RUSTSEC-2024-0370 (`proc-macro-error`, unmaintained), RUSTSEC-2024-0429 (`glib`, unsound) | transitive via Tauri                        | Warnings, not vulnerabilities. The first two are compile-time macros. `glib` is the Linux GTK backend and isn't compiled for macOS.      | Each Tauri upgrade |

## Credentials policy

- Otto never reads, copies or stores Claude, OpenAI or GitHub tokens. Each official CLI manages its own login inside the workspace's isolated environment.
- Otto's own secrets (for example an SSH key passphrase) go in the macOS Keychain.
