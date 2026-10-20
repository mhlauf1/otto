-- Otto app state (SPEC §5.3). Later milestones add agents, sessions and tasks.

CREATE TABLE workspaces (
    id                  TEXT PRIMARY KEY NOT NULL,
    name                TEXT NOT NULL,
    tint                TEXT NOT NULL,
    git_name            TEXT NOT NULL,
    git_email           TEXT NOT NULL,
    github_user         TEXT NOT NULL,
    gh_owner_allowlist  TEXT NOT NULL, -- JSON array of GitHub owners
    position            INTEGER NOT NULL,
    created_at          TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
) STRICT;

CREATE TABLE repos (
    id            INTEGER PRIMARY KEY,
    workspace_id  TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    path          TEXT NOT NULL UNIQUE, -- a repo belongs to exactly one workspace
    remote_url    TEXT NOT NULL,
    verified_at   TEXT NOT NULL
) STRICT;

-- Append-only audit log: identity checks, blocked pushes, refused repos.
CREATE TABLE events (
    id            INTEGER PRIMARY KEY,
    at            TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    workspace_id  TEXT,
    kind          TEXT NOT NULL,
    detail        TEXT NOT NULL -- JSON
) STRICT;

CREATE TRIGGER events_append_only_update BEFORE UPDATE ON events
BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
CREATE TRIGGER events_append_only_delete BEFORE DELETE ON events
BEGIN SELECT RAISE(ABORT, 'events are append-only'); END;
