-- Repos with no GitHub remote can be registered once Mike picks a workspace
-- (decided 2026-10-20). Pushes stay locked until a matching remote exists.
CREATE TABLE repos_new (
    id            INTEGER PRIMARY KEY,
    workspace_id  TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    path          TEXT NOT NULL UNIQUE,
    remote_url    TEXT, -- NULL for local-only repos
    verified_at   TEXT NOT NULL
) STRICT;
INSERT INTO repos_new SELECT id, workspace_id, path, remote_url, verified_at FROM repos;
DROP TABLE repos;
ALTER TABLE repos_new RENAME TO repos;
