import * as Dialog from '@radix-ui/react-dialog';
import { useQuery } from '@tanstack/react-query';
import { isTauri } from '@tauri-apps/api/core';
import { useState } from 'react';
import { commands, type SetupItem, type SetupKind, type Workspace } from '../bindings';
import styles from '../ui/Sheet.module.css';

const GROUPS: { kind: SetupKind; title: string }[] = [
  { kind: 'instructions', title: 'Instructions' },
  { kind: 'settings', title: 'Settings' },
  { kind: 'plugin', title: 'Plugins' },
  { kind: 'mcpServer', title: 'MCP servers' },
  { kind: 'skill', title: 'Skills' },
  { kind: 'agent', title: 'Subagents' },
  { kind: 'command', title: 'Slash commands' },
];

interface ClaudeSetupProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workspaces: Workspace[];
}

/**
 * Pick, per workspace, which parts of the normal ~/.claude setup to copy in (Mike,
 * 2026-10-20). Each workspace gets its own copy; nothing is shared or synced back.
 */
export function ClaudeSetup({ open, onOpenChange, workspaces }: ClaudeSetupProps) {
  const inventory = useQuery({
    queryKey: ['claudeSetup'],
    enabled: open && isTauri(),
    queryFn: async (): Promise<SetupItem[]> => {
      const r = await commands.claudeSetupInventory();
      if (r.status === 'error') throw new Error(r.error.message);
      return r.data;
    },
  });
  // picked[workspaceId] = set of item ids
  const [picked, setPicked] = useState<Record<string, string[]>>({});
  const [result, setResult] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const toggle = (ws: string, id: string) => {
    setPicked((p) => {
      const current = p[ws] ?? [];
      return {
        ...p,
        [ws]: current.includes(id) ? current.filter((x) => x !== id) : [...current, id],
      };
    });
  };

  const total = Object.values(picked).reduce((n, ids) => n + ids.length, 0);

  const save = async () => {
    setSaving(true);
    const lines: string[] = [];
    for (const ws of workspaces) {
      const ids = picked[ws.id] ?? [];
      if (ids.length === 0) continue;
      const r = await commands.importClaudeSetup(ws.id, ids);
      lines.push(
        r.status === 'ok'
          ? `${ws.name}: copied ${String(r.data.length)} items.`
          : `${ws.name}: ${r.error.message}`,
      );
    }
    setSaving(false);
    setPicked({});
    setResult(
      `${lines.join(' ')} New Claude sessions in those workspaces will use them; restart any that are open.`,
    );
  };

  const items = inventory.data ?? [];

  return (
    <Dialog.Root
      open={open}
      onOpenChange={(o) => {
        onOpenChange(o);
        if (!o) setResult(null);
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={styles.overlay} />
        <Dialog.Content className={styles.sheet}>
          <header className={styles.header}>
            <Dialog.Title className={styles.title}>Bring in your Claude setup</Dialog.Title>
            <Dialog.Description className={styles.description}>
              Each workspace’s Claude starts clean. Tick what to copy from your normal ~/.claude.
              Copies only: credentials, history and account-synced skills never move.
            </Dialog.Description>
          </header>

          <div className={styles.body}>
            {!isTauri() ? (
              <p className={styles.notice}>Open Otto with `pnpm dev` to read your setup.</p>
            ) : null}
            {inventory.isError ? (
              <p className={styles.notice} role="alert">
                {inventory.error.message}
              </p>
            ) : null}
            {result ? (
              <p className={styles.success} role="status">
                {result}
              </p>
            ) : null}
            {GROUPS.map(({ kind, title }) => {
              const rows = items.filter((i) => i.kind === kind);
              if (rows.length === 0) return null;
              return (
                <section key={kind} className={styles.group}>
                  <table className={styles.grid}>
                    <thead>
                      <tr>
                        <th scope="col">{title}</th>
                        {workspaces.map((w) => (
                          <th key={w.id} scope="col" className={styles.check}>
                            {w.name}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {rows.map((item) => (
                        <tr key={item.id}>
                          <td>
                            <div className={styles.repo}>
                              <span className={styles.name}>{item.name}</span>
                              <span className={styles.meta}>{item.detail}</span>
                              {item.warning ? (
                                <span className={styles.warning}>⚠ {item.warning}</span>
                              ) : null}
                            </div>
                          </td>
                          {workspaces.map((w) => (
                            <td key={w.id} className={styles.check}>
                              <input
                                type="checkbox"
                                aria-label={`Copy ${item.name} into ${w.name}`}
                                checked={(picked[w.id] ?? []).includes(item.id)}
                                onChange={() => {
                                  toggle(w.id, item.id);
                                }}
                              />
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </section>
              );
            })}
          </div>

          <footer className={styles.footer}>
            <Dialog.Close className={styles.secondary}>Done</Dialog.Close>
            <button
              type="button"
              className={styles.primary}
              disabled={saving || total === 0}
              onClick={() => {
                void save();
              }}
            >
              {saving ? 'Copying…' : `Copy ${String(total)} items`}
            </button>
          </footer>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
