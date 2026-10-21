-- Project groups (ADR-017): sections inside one workspace, such as Lauf's Clients,
-- Studio and Personal. A group never spans workspaces, and a project can only sit in
-- a group of its own workspace: the composite foreign key below makes the database
-- itself refuse anything else.
CREATE TABLE project_groups (
    id            TEXT PRIMARY KEY NOT NULL,
    workspace_id  TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    position      INTEGER NOT NULL,
    UNIQUE (workspace_id, name),
    UNIQUE (id, workspace_id)
) STRICT;

CREATE TABLE repos_new (
    id            INTEGER PRIMARY KEY,
    workspace_id  TEXT NOT NULL REFERENCES workspaces (id) ON DELETE CASCADE,
    path          TEXT NOT NULL UNIQUE, -- a repo belongs to exactly one workspace
    remote_url    TEXT, -- NULL for local-only repos
    verified_at   TEXT NOT NULL,
    group_id      TEXT, -- NULL means "not in a group"
    FOREIGN KEY (group_id, workspace_id) REFERENCES project_groups (id, workspace_id)
) STRICT;
INSERT INTO repos_new (id, workspace_id, path, remote_url, verified_at)
    SELECT id, workspace_id, path, remote_url, verified_at FROM repos;
DROP TABLE repos;
ALTER TABLE repos_new RENAME TO repos;

-- Existing installs: give Lauf its starter groups. Fresh installs get them from
-- `Store::seed_defaults`, which runs after the workspaces exist.
INSERT INTO project_groups (id, workspace_id, name, position)
    SELECT 'lauf-' || lower(g.name), 'lauf', g.name, g.position
    FROM (SELECT 'Clients' AS name, 0 AS position
          UNION ALL SELECT 'Studio', 1
          UNION ALL SELECT 'Personal', 2) AS g
    WHERE EXISTS (SELECT 1 FROM workspaces WHERE id = 'lauf');
