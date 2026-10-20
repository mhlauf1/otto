import * as Dialog from '@radix-ui/react-dialog';
import { useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import type { DiscoveredRepo, Workspace } from '../bindings';
import { FriendlyError, queryKeys, registerProject, useDiscoveredProjects } from '../app/queries';
import styles from '../ui/Sheet.module.css';

interface ImportProjectsProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workspaces: Workspace[];
}

/** Choice per repo: a workspace id, or null to skip. */
type Choices = Record<string, string | null>;

function suggestedChoice(repo: DiscoveredRepo): string | null {
  if (repo.registeredTo) return null;
  return repo.suggestion.kind === 'workspace' ? repo.suggestion.workspace : null;
}

/**
 * Auto-discover + confirm (decided with Mike 2026-10-20). Otto sorts repos by GitHub
 * owner; Mike confirms once. Repos with no remote need a workspace picked by hand.
 */
export function ImportProjects({ open, onOpenChange, workspaces }: ImportProjectsProps) {
  const discovered = useDiscoveredProjects(open);
  const queryClient = useQueryClient();
  const [overrides, setOverrides] = useState<Choices>({});
  const [saving, setSaving] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  const repos = useMemo(() => discovered.data ?? [], [discovered.data]);
  const choiceFor = (repo: DiscoveredRepo) =>
    repo.path in overrides ? (overrides[repo.path] ?? null) : suggestedChoice(repo);

  const groups = useMemo(() => {
    const byKind = (pred: (r: DiscoveredRepo) => boolean) =>
      repos.filter((r) => !r.registeredTo && pred(r));
    return [
      { title: 'Matched by GitHub owner', repos: byKind((r) => r.suggestion.kind === 'workspace') },
      {
        title: 'No remote: pick a workspace (pushes stay locked)',
        repos: byKind((r) => r.suggestion.kind === 'needsChoice'),
      },
      { title: 'Already imported', repos: repos.filter((r) => r.registeredTo) },
      {
        title: 'Can’t be imported',
        repos: byKind((r) => r.suggestion.kind === 'notYours' || r.suggestion.kind === 'blocked'),
      },
    ].filter((g) => g.repos.length > 0);
  }, [repos]);

  const toImport = repos.filter((r) => !r.registeredTo && choiceFor(r) !== null);

  const save = async () => {
    setSaving(true);
    const failed: Record<string, string> = {};
    for (const repo of toImport) {
      const ws = choiceFor(repo);
      if (!ws) continue;
      try {
        await registerProject(ws, repo.path, repo.suggestion.kind === 'needsChoice');
      } catch (e) {
        failed[repo.path] = e instanceof FriendlyError ? e.message : 'Couldn’t import.';
      }
    }
    setErrors(failed);
    setSaving(false);
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.discovered }),
      ...workspaces.map((w) =>
        queryClient.invalidateQueries({ queryKey: queryKeys.projects(w.id) }),
      ),
    ]);
    if (Object.keys(failed).length === 0) onOpenChange(false);
  };

  const wsName = (id: string | null) => workspaces.find((w) => w.id === id)?.name ?? id ?? '';

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className={styles.overlay} />
        <Dialog.Content className={styles.sheet}>
          <header className={styles.header}>
            <Dialog.Title className={styles.title}>Import projects</Dialog.Title>
            <Dialog.Description className={styles.description}>
              Otto found these git repos and sorted them by GitHub owner. Each project belongs to
              exactly one workspace.
            </Dialog.Description>
          </header>

          <div className={styles.body}>
            {discovered.isPending ? <p className={styles.notice}>Looking for repos…</p> : null}
            {discovered.isError ? (
              <p className={styles.notice} role="alert">
                {discovered.error instanceof FriendlyError
                  ? discovered.error.message
                  : 'Couldn’t scan for projects.'}
              </p>
            ) : null}
            {groups.map((group) => (
              <section key={group.title} className={styles.group}>
                <h3 className={styles.groupTitle}>
                  {group.title} <span className={styles.count}>{group.repos.length}</span>
                </h3>
                <ul className={styles.list}>
                  {group.repos.map((repo) => {
                    const importable =
                      !repo.registeredTo &&
                      (repo.suggestion.kind === 'workspace' ||
                        repo.suggestion.kind === 'needsChoice');
                    const choice = choiceFor(repo);
                    return (
                      <li key={repo.path} className={styles.row}>
                        <div className={styles.repo}>
                          <span className={styles.name}>{repo.name}</span>
                          <span className={styles.meta} title={repo.path}>
                            {repo.remoteUrl ?? repo.path.replace(/^\/Users\/[^/]+/, '~')}
                          </span>
                          {repo.suggestion.kind === 'blocked' ? (
                            <span className={styles.reason}>{repo.suggestion.reason}</span>
                          ) : null}
                          {repo.suggestion.kind === 'notYours' ? (
                            <span className={styles.reason}>
                              Owned by {repo.suggestion.owners.join(', ')}, which no workspace
                              allows.
                            </span>
                          ) : null}
                          {errors[repo.path] ? (
                            <span className={styles.reason}>{errors[repo.path]}</span>
                          ) : null}
                        </div>
                        {repo.registeredTo ? (
                          <span className={styles.badge}>{wsName(repo.registeredTo)}</span>
                        ) : importable ? (
                          <select
                            className={styles.select}
                            aria-label={`Workspace for ${repo.name}`}
                            value={choice ?? ''}
                            onChange={(e) => {
                              setOverrides((o) => ({ ...o, [repo.path]: e.target.value || null }));
                            }}
                          >
                            <option value="">Skip</option>
                            {workspaces
                              .filter(
                                (w) =>
                                  repo.suggestion.kind === 'needsChoice' ||
                                  (repo.suggestion.kind === 'workspace' &&
                                    repo.suggestion.workspace === w.id),
                              )
                              .map((w) => (
                                <option key={w.id} value={w.id}>
                                  {w.name}
                                </option>
                              ))}
                          </select>
                        ) : null}
                      </li>
                    );
                  })}
                </ul>
              </section>
            ))}
          </div>

          <footer className={styles.footer}>
            <Dialog.Close className={styles.secondary}>Cancel</Dialog.Close>
            <button
              type="button"
              className={styles.primary}
              disabled={saving || toImport.length === 0}
              onClick={() => {
                void save();
              }}
            >
              {saving ? 'Importing…' : `Import ${String(toImport.length)} projects`}
            </button>
          </footer>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
