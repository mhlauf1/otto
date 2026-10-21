import * as Dialog from '@radix-ui/react-dialog';
import { useQueryClient } from '@tanstack/react-query';
import { open as pickFolder } from '@tauri-apps/plugin-dialog';
import { useMemo, useState } from 'react';
import { commands, type DiscoveredRepo, type Workspace } from '../bindings';
import {
  FriendlyError,
  groupActions,
  queryKeys,
  registerProject,
  useDiscoveredProjects,
} from '../app/queries';
import styles from '../ui/Sheet.module.css';

interface ImportProjectsProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workspaces: Workspace[];
  /** Import straight into this group. Only projects of the group's workspace go into it. */
  target?: { workspaceId: string; groupId: string; groupName: string } | undefined;
}

const isImportable = (r: DiscoveredRepo) =>
  !r.registeredTo && (r.suggestion.kind === 'workspace' || r.suggestion.kind === 'needsChoice');

const tidyPath = (path: string) => path.replace(/^\/Users\/[^/]+/, '~');

/**
 * Opt-in import (Mike, 2026-10-20): nothing is pre-selected. Tick what you want.
 * Matched repos already know their workspace from the GitHub owner; repos with no
 * remote need one picked. "Choose folder…" adds a repo outside the scanned places.
 */
export function ImportProjects({ open, onOpenChange, workspaces, target }: ImportProjectsProps) {
  const discovered = useDiscoveredProjects(open);
  const queryClient = useQueryClient();
  /** Ticked repos → chosen workspace ('' until picked, for repos with no remote). */
  const [selected, setSelected] = useState<Record<string, string>>({});
  const [added, setAdded] = useState<DiscoveredRepo[]>([]);
  const [query, setQuery] = useState('');
  const [saving, setSaving] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [pickError, setPickError] = useState<string | null>(null);

  const all = useMemo(() => {
    const scanned = discovered.data ?? [];
    const extra = added.filter((a) => !scanned.some((s) => s.path === a.path));
    return [...extra, ...scanned];
  }, [discovered.data, added]);

  const matches = (r: DiscoveredRepo) => {
    const q = query.trim().toLowerCase();
    return !q || `${r.name} ${r.path} ${r.remoteUrl ?? ''}`.toLowerCase().includes(q);
  };
  const importable = all.filter((r) => isImportable(r) && matches(r));
  const imported = all.filter((r) => r.registeredTo && matches(r));
  const blocked = all.filter((r) => !r.registeredTo && !isImportable(r) && matches(r));

  const defaultWorkspace = (r: DiscoveredRepo) =>
    r.suggestion.kind === 'workspace' ? r.suggestion.workspace : '';

  const toggle = (r: DiscoveredRepo) => {
    setSelected((s) =>
      r.path in s
        ? Object.fromEntries(Object.entries(s).filter(([p]) => p !== r.path))
        : { ...s, [r.path]: defaultWorkspace(r) },
    );
  };

  const ready = Object.entries(selected).filter(([, ws]) => ws !== '');
  const missingWorkspace = Object.values(selected).filter((ws) => ws === '').length;

  const chooseFolder = async () => {
    setPickError(null);
    const path = await pickFolder({
      directory: true,
      multiple: false,
      title: 'Choose a project folder',
    });
    if (typeof path !== 'string') return;
    const r = await commands.inspectProject(path);
    if (r.status === 'error') {
      setPickError(r.error.message);
      return;
    }
    setAdded((a) => [r.data, ...a.filter((x) => x.path !== r.data.path)]);
    setQuery('');
    if (isImportable(r.data)) {
      setSelected((s) => ({ ...s, [r.data.path]: defaultWorkspace(r.data) }));
    }
  };

  const save = async () => {
    setSaving(true);
    const failed: Record<string, string> = {};
    for (const [path, ws] of ready) {
      const repo = all.find((r) => r.path === path);
      try {
        await registerProject(ws, path, repo?.suggestion.kind === 'needsChoice');
        if (target?.workspaceId === ws) {
          await groupActions.moveProject(ws, path, target.groupId);
        }
      } catch (e) {
        failed[path] = e instanceof FriendlyError ? e.message : 'Couldn’t import.';
      }
    }
    setErrors(failed);
    setSaving(false);
    setSelected((s) => Object.fromEntries(Object.entries(s).filter(([p]) => p in failed)));
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.discovered }),
      ...workspaces.map((w) =>
        queryClient.invalidateQueries({ queryKey: queryKeys.projects(w.id) }),
      ),
      ...workspaces.map((w) => queryClient.invalidateQueries({ queryKey: queryKeys.past(w.id) })),
    ]);
    if (Object.keys(failed).length === 0) onOpenChange(false);
  };

  const wsName = (id: string | null) => workspaces.find((w) => w.id === id)?.name ?? '';

  const row = (repo: DiscoveredRepo) => {
    const checked = repo.path in selected;
    const choice = selected[repo.path] ?? defaultWorkspace(repo);
    return (
      <li key={repo.path} className={styles.row}>
        <label className={styles.pick}>
          <input
            type="checkbox"
            checked={checked}
            onChange={() => {
              toggle(repo);
            }}
            aria-label={`Import ${repo.name}`}
          />
          <span className={styles.repo}>
            <span className={styles.name}>{repo.name}</span>
            <span className={styles.meta} title={repo.path}>
              {tidyPath(repo.path)}
            </span>
            {errors[repo.path] ? <span className={styles.reason}>{errors[repo.path]}</span> : null}
          </span>
        </label>
        {repo.suggestion.kind === 'workspace' ? (
          <span className={styles.badge}>{wsName(repo.suggestion.workspace)}</span>
        ) : (
          <select
            className={styles.select}
            aria-label={`Workspace for ${repo.name}`}
            value={choice}
            onChange={(e) => {
              const ws = e.target.value;
              setSelected((s) => ({ ...s, [repo.path]: ws }));
            }}
          >
            <option value="">Pick workspace</option>
            {workspaces.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name}
              </option>
            ))}
          </select>
        )}
      </li>
    );
  };

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className={styles.overlay} />
        <Dialog.Content className={styles.sheet}>
          <header className={styles.header}>
            <Dialog.Title className={styles.title}>
              {target ? `Import projects into ${target.groupName}` : 'Import projects'}
            </Dialog.Title>
            <Dialog.Description className={styles.description}>
              Tick the projects you want. Otto remembers where each folder is and puts it in the
              workspace that owns it on GitHub
              {target
                ? `. ${wsName(target.workspaceId)} projects go straight into ${target.groupName}.`
                : '.'}
            </Dialog.Description>
            <div className={styles.toolbar}>
              <input
                className={styles.search}
                placeholder="Search projects…"
                value={query}
                autoFocus
                onChange={(e) => {
                  setQuery(e.target.value);
                }}
              />
              <button
                type="button"
                className={styles.secondary}
                onClick={() => {
                  void chooseFolder();
                }}
              >
                Choose folder…
              </button>
            </div>
            {pickError ? (
              <p className={styles.reason} role="alert">
                {pickError}
              </p>
            ) : null}
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

            {importable.length > 0 ? (
              <ul className={styles.list}>{importable.map(row)}</ul>
            ) : discovered.isSuccess ? (
              <p className={styles.notice}>No projects match “{query}”.</p>
            ) : null}

            {imported.length > 0 ? (
              <details className={styles.group}>
                <summary className={styles.groupTitle}>
                  Already imported <span className={styles.count}>{imported.length}</span>
                </summary>
                <ul className={styles.list}>
                  {imported.map((r) => (
                    <li key={r.path} className={styles.row}>
                      <span className={styles.repo}>
                        <span className={styles.name}>{r.name}</span>
                        <span className={styles.meta}>{tidyPath(r.path)}</span>
                      </span>
                      <span className={styles.badge}>{wsName(r.registeredTo)}</span>
                    </li>
                  ))}
                </ul>
              </details>
            ) : null}

            {blocked.length > 0 ? (
              <details className={styles.group}>
                <summary className={styles.groupTitle}>
                  Can’t be imported <span className={styles.count}>{blocked.length}</span>
                </summary>
                <ul className={styles.list}>
                  {blocked.map((r) => (
                    <li key={r.path} className={styles.row}>
                      <span className={styles.repo}>
                        <span className={styles.name}>{r.name}</span>
                        <span className={styles.reason}>
                          {r.suggestion.kind === 'blocked'
                            ? r.suggestion.reason
                            : r.suggestion.kind === 'notYours'
                              ? `Owned by ${r.suggestion.owners.join(', ')}, which no workspace allows.`
                              : ''}
                        </span>
                      </span>
                    </li>
                  ))}
                </ul>
              </details>
            ) : null}
          </div>

          <footer className={styles.footer}>
            {missingWorkspace > 0 ? (
              <span className={styles.hint}>
                Pick a workspace for {missingWorkspace} project{missingWorkspace === 1 ? '' : 's'}
              </span>
            ) : null}
            <Dialog.Close className={styles.secondary}>Cancel</Dialog.Close>
            <button
              type="button"
              className={styles.primary}
              disabled={saving || ready.length === 0 || missingWorkspace > 0}
              onClick={() => {
                void save();
              }}
            >
              {saving
                ? 'Importing…'
                : ready.length === 0
                  ? 'Import'
                  : `Import ${String(ready.length)} project${ready.length === 1 ? '' : 's'}`}
            </button>
          </footer>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
