-- M5b (SPEC §8.2): prompts waiting for a session, and worktrees for parallel tasks.

CREATE TABLE worktrees (
    id            TEXT PRIMARY KEY NOT NULL,
    workspace_id  TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    project_path  TEXT NOT NULL,         -- the registered repo it was made from
    path          TEXT NOT NULL UNIQUE,  -- inside <workspace>/worktrees/
    branch        TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

-- A session started for a parallel task runs in its worktree.
ALTER TABLE sessions ADD COLUMN worktree_id TEXT REFERENCES worktrees (id) ON DELETE SET NULL;

CREATE TABLE queue_items (
    id            TEXT PRIMARY KEY NOT NULL,
    workspace_id  TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    session_id    TEXT NOT NULL REFERENCES sessions (id) ON DELETE CASCADE,
    prompt        TEXT NOT NULL,
    position      INTEGER NOT NULL,
    state         TEXT NOT NULL DEFAULT 'queued', -- queued | sent
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    sent_at       TEXT
) STRICT;

CREATE INDEX queue_waiting ON queue_items (session_id, state, position);
