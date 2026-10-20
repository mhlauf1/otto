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

All guarantees are tested in `crates/otto-core/tests/isolation.rs`, which runs as its own required CI job. It uses real git and gh, local bare repos stand in for GitHub, and no test touches the network.

| #   | Guarantee                                              | Mechanism                                                                          | Tests                                                                                                      |
| --- | ------------------------------------------------------ | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| G1  | Spawned processes never inherit the login environment  | `IsolatedEnv`: `env_clear()` plus an allowlist; `set` refuses forbidden keys       | `t3_spawned_processes_contain_no_stripped_variables` (env, sh, zsh, with a polluted parent)                |
| G2  | Each workspace presents only its own GitHub credential | gh `--insecure-storage` in `<ws>/gh`; credential helper in the workspace gitconfig | `t4_each_workspace_only_ever_presents_its_own_github_credential` (concurrent, real gh)                     |
| G3  | Git identity and config come only from the workspace   | `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_NOSYSTEM`, `GIT_AUTHOR_*`/`GIT_COMMITTER_*`       | `t2_repo_local_email_cannot_override…`, `t4_concurrent_workspaces_only_read_their_own_git_config`          |
| G4  | Pushes reach only allowlisted owners                   | `pre-push` checks both the configured and the effective URL                        | `t1_playbook_cannot_push_to_a_lauf_owned_remote`, `t1_an_insteadof_rewrite…`, `t1_control…`                |
| G5  | Foreign-authored commits never leave                   | `pre-commit`, plus a `pre-push` per-commit check (survives `commit --no-verify`)   | `t2_commit_with_a_foreign_author_is_blocked`, `t2_a_foreign_commit_made_with_no_verify…`                   |
| G6  | Hooks fail closed                                      | `OTTO_WORKSPACE` must match; a red Doctor blocks pushes                            | `t1_hooks_fail_closed…`, `t1_pushes_are_blocked_while_the_identity_doctor_is_red`                          |
| G7  | The SSH passphrase never reaches anything but ssh      | `GIT_ASKPASS=/usr/bin/false`; askpass checks the prompt                            | `t1_git_never_offers_the_ssh_passphrase…`, `askpass_answers_only_its_own_key_prompt`                       |
| G8  | A repo belongs to exactly one workspace                | `verify_repo` plus a UNIQUE path in SQLite                                         | `repos_only_open_in…`, `repos_that_override_guardrails…`, `store::a_repo_belongs_to_exactly_one_workspace` |
| G9  | Context layers never cross workspaces                  | ContextService                                                                     | M4                                                                                                         |

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

## Incidents

| Date       | What                                                                                                                                                                                                 | Action                                                                                                                                           |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 2026-10-20 | A scan of Mike's Desktop found a GitHub token (mhlauf1) embedded in `~/Desktop/lauf/dev/port-v1`'s remote URL, in plaintext in `.git/config`. It was also printed in a Claude Code session's output. | Stripped from the remote with Mike's approval. **Mike to revoke the token on GitHub.** Otto now refuses such repos and blocks such pushes (G10). |

## Dependency audit exceptions

| Date       | Advisory                                                                                                                                                     | Path                                        | Why accepted                                                                                                                             | Revisit            |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------ |
| 2026-10-20 | GHSA-vfj7-8cjw-p6xm (`braces`, high, stack-exhaustion DoS)                                                                                                   | dev only: `stylelint › micromatch › braces` | It's a build-time lint tool that only reads glob patterns we write, and it isn't shipped. No patched version exists. CI audits `--prod`. | When a patch ships |
| 2026-10-20 | `cargo audit` warnings: RUSTSEC-2024-0436 (`paste`, unmaintained), RUSTSEC-2024-0370 (`proc-macro-error`, unmaintained), RUSTSEC-2024-0429 (`glib`, unsound) | transitive via Tauri                        | Warnings, not vulnerabilities. The first two are compile-time macros. `glib` is the Linux GTK backend and isn't compiled for macOS.      | Each Tauri upgrade |

## Credentials policy

- Otto never reads, copies or stores Claude, OpenAI or GitHub tokens. Each official CLI manages its own login inside the workspace's isolated environment.
- Otto's own secrets (for example an SSH key passphrase) go in the macOS Keychain.
