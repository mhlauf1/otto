# Parking lot

Ideas for V2 and later. Nothing here gets built in V1. Newest first.

## From the "where next" session (2026-10-20)

- **Chat-style centre view:** a Codex-app-like chat over the agents in headless mode. Parked by Mike. The terminal stays, because a chat view would rebuild approvals, diffs and tools, and drift from the CLIs.
- **Dictation helper:** only if dictating into the terminal proves painful, now that the composer is gone.
- **"Save to Knowledge":** at the end of a session, Claude drafts a note from what was learned (for example, a client's update history) for Mike to approve.
- **Per-project notes:** for example, `notes/clients/stoc.md`, linked to its project and shown in the project header.
- **Playbook Core MCP in Playbook only:** wire the Playbook Core MCP server into the Playbook workspace's Claude setup and never Lauf's. The Claude-setup import can already do this per workspace.
- **Session rename** and drag-to-reorder for projects and groups.
- **Links in notes open in the browser.** For now they show their address on hover but don't navigate, because that would replace Otto's window. Needs the Tauri opener plugin.
- **Ask with Codex**, and listing past Ask conversations on the Knowledge page.
- **Back up Knowledge off this Mac** (a private repo or iCloud). It's a privacy choice for Mike, because notes include equity and client details.
- **Codex's package cache** (325 MB in Lauf) belongs to Codex; revisit if it keeps growing.
- **Watch knowledge files live**, so edits made outside Otto refresh the list without reopening. For now they show on reopen, and agents get them on their next session.

## From SPEC §12 (2026-10-20)

- **Scheduled runs:** headless agent runs (`claude -p`, `codex exec`) triggered by launchd at set times (for example 8am). Missed jobs run on wake. A queue that runs when Otto launches.
- **Use-it-or-lose-it usage:** nudges and auto-queued work when an account has spare capacity.
- **Daily make-believe company:** a fictional brand each day (site, components, branding assets) feeding a portfolio library.
- **Component and section library:** a steady stream of polished components to post on Twitter/X.
- **Illustrator pipeline:** agent-driven marketing and branding assets.
- **Redesign pitch machine:** take a real business with a weak site, generate a new one from Lauf's section skeletons, deploy to Vercel, share a link.
- **Importing past work:** ingest previous projects and chats into the context layers.
- More workspaces and accounts, and usage dashboards across accounts.
- Regular claude.ai and ChatGPT chat apps, Windows and Linux, cloud sync.
