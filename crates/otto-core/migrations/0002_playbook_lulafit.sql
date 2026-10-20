-- Mike confirmed 2026-10-20: Playbook's GitHub org is `lulafit`.
UPDATE workspaces
SET gh_owner_allowlist = '["mikelauf","lulafit"]'
WHERE id = 'playbook' AND gh_owner_allowlist = '["mikelauf"]';
