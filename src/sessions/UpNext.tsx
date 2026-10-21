import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { FriendlyError, queryKeys, queueActions, useQueue } from '../app/queries';
import type { LiveSession } from './registry';
import styles from './UpNext.module.css';

interface UpNextProps {
  workspaceId: string;
  session: LiveSession;
  /** Starts the prompt as a parallel task in a new worktree; absent outside a project. */
  onParallel?: ((prompt: string) => Promise<void>) | undefined;
}

const errorText = (e: unknown) =>
  e instanceof FriendlyError || e instanceof Error ? e.message : 'That didn’t work.';

/**
 * "Up next" (SPEC §8.2, M5b): prompts waiting for this session. Each one is sent when the
 * agent itself reports it finished; nothing is sent while it works or needs Mike. It holds
 * only waiting prompts; the terminal stays the place to talk to the agent.
 */
export function UpNext({ workspaceId, session, onParallel }: UpNextProps) {
  const queryClient = useQueryClient();
  const id = session.info.id;
  const queue = useQueue(workspaceId, id, true);
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const items = queue.data ?? [];
  const agent = session.info.agent === 'codex' ? 'Codex' : 'Claude';

  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: queryKeys.queue(workspaceId, id) });
  };

  const submit = async (parallel: boolean) => {
    const prompt = draft.trim();
    if (!prompt || busy) return;
    setBusy(true);
    setError(null);
    try {
      if (parallel && onParallel) await onParallel(prompt);
      else await queueActions.add(workspaceId, id, prompt);
      setDraft('');
      setAdding(false);
      refresh();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className={styles.panel} aria-label="Up next">
      {items.length > 0 ? (
        <>
          <p className={styles.heading}>
            Up next <span className={styles.hint}>· runs when {agent} finishes</span>
          </p>
          <ol className={styles.list}>
            {items.map((item, i) => (
              <li key={item.id} className={styles.item}>
                <span className={styles.number}>{i + 1}</span>
                <span className={styles.prompt} title={item.prompt}>
                  {item.prompt}
                </span>
                <button
                  type="button"
                  className={styles.remove}
                  aria-label={`Remove “${item.prompt}”`}
                  onClick={() => {
                    queueActions.remove(workspaceId, item.id).then(refresh, (e: unknown) => {
                      setError(errorText(e));
                    });
                  }}
                >
                  ×
                </button>
              </li>
            ))}
          </ol>
        </>
      ) : null}

      {adding ? (
        <form
          className={styles.form}
          onSubmit={(e) => {
            e.preventDefault();
            void submit(false);
          }}
        >
          <textarea
            className={styles.field}
            aria-label="Next prompt"
            placeholder={`Next prompt for ${agent}. It runs when ${agent} finishes.`}
            value={draft}
            autoFocus
            rows={2}
            onChange={(e) => {
              setDraft(e.target.value);
            }}
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                setAdding(false);
                session.term.focus();
              } else if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
                e.preventDefault();
                void submit(e.metaKey && onParallel !== undefined);
              }
            }}
          />
          <div className={styles.actions}>
            <span className={styles.keys}>
              ↵ add · ⇧↵ new line{onParallel ? ' · ⌘↵ in parallel' : ''} · esc cancel
            </span>
            {onParallel ? (
              <button
                type="button"
                className={styles.secondary}
                disabled={busy || !draft.trim()}
                title="Start it now in its own worktree, on a new otto/… branch"
                onClick={() => {
                  void submit(true);
                }}
              >
                Run in parallel
              </button>
            ) : null}
            <button type="submit" className={styles.primary} disabled={busy || !draft.trim()}>
              Add to Up next
            </button>
          </div>
        </form>
      ) : (
        <button
          type="button"
          className={styles.add}
          onClick={() => {
            setAdding(true);
          }}
        >
          + Up next
        </button>
      )}

      {error ? (
        <p className={styles.error} role="alert">
          {error}
        </p>
      ) : null}
    </section>
  );
}
