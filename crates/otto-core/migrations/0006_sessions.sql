-- Sessions that outlive Otto (SPEC §8, M5a). A row stays `open` until Mike closes the
-- session or its program exits, so sessions still open when Otto quits come back paused.
CREATE TABLE sessions (
    id               TEXT PRIMARY KEY NOT NULL, -- Otto's ID; also Claude's session ID
    workspace_id     TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    agent            TEXT NOT NULL,             -- claude | codex | shell
    project_path     TEXT,                      -- NULL means General
    title            TEXT NOT NULL,
    kind             TEXT NOT NULL DEFAULT 'work', -- work | ask | askPrivate
    cli_session_id   TEXT,                      -- Codex's thread ID, once it reports one
    state            TEXT NOT NULL DEFAULT 'open', -- open | closed
    status           TEXT NOT NULL DEFAULT 'idle', -- idle | running | waiting | exited
    created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_active_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    closed_at        TEXT
) STRICT;

CREATE INDEX sessions_open ON sessions (workspace_id, state);
