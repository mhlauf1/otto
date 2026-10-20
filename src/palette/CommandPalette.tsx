import * as Dialog from '@radix-ui/react-dialog';
import { useMemo, useState } from 'react';
import { score } from './match';
import styles from './CommandPalette.module.css';

export interface PaletteAction {
  id: string;
  group: string;
  label: string;
  /** Extra words the action should match, e.g. "terminal" for Shell. */
  keywords?: string;
  shortcut?: string;
  run: () => void;
}

interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  actions: PaletteAction[];
}

/** ⌘K (SPEC §6.3). Grouped, keyboard-first; Supabase/Magnific/Navattic references in DESIGN.md. */
export function CommandPalette({ open, onOpenChange, actions }: CommandPaletteProps) {
  const [query, setQuery] = useState('');
  const [cursor, setCursor] = useState(0);

  const results = useMemo(() => {
    const scored = actions
      .map((a) => ({ a, s: score(query, `${a.label} ${a.keywords ?? ''}`) }))
      .filter((x): x is { a: PaletteAction; s: number } => x.s !== null);
    // With a query, rank by score; without one, keep the given grouping and order.
    const searching = query.trim() !== '';
    if (searching) scored.sort((x, y) => y.s - x.s);
    // Group headings only when browsing; search results are one ranked list.
    return scored.map((x, i) => ({
      action: x.a,
      heading: !searching && x.a.group !== scored[i - 1]?.a.group ? x.a.group : null,
    }));
  }, [actions, query]);

  const close = () => {
    onOpenChange(false);
    setQuery('');
    setCursor(0);
  };

  const run = (action: PaletteAction | undefined) => {
    if (!action) return;
    close();
    action.run();
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setCursor((c) => Math.min(c + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setCursor((c) => Math.max(c - 1, 0));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      run(results[cursor]?.action);
    }
  };

  return (
    <Dialog.Root
      open={open}
      onOpenChange={(next) => {
        if (next) onOpenChange(true);
        else close();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className={styles.overlay} />
        <Dialog.Content className={styles.panel} aria-describedby={undefined} onKeyDown={onKeyDown}>
          <Dialog.Title className={styles.srOnly}>Command palette</Dialog.Title>
          <input
            className={styles.input}
            placeholder="Type a command or search…"
            value={query}
            autoFocus
            role="combobox"
            aria-expanded
            aria-controls="otto-palette-list"
            aria-activedescendant={
              results[cursor] ? `palette-${results[cursor].action.id}` : undefined
            }
            onChange={(e) => {
              setQuery(e.target.value);
              setCursor(0);
            }}
          />
          <ul className={styles.list} id="otto-palette-list" role="listbox">
            {results.length === 0 ? <li className={styles.empty}>No matching commands</li> : null}
            {results.map(({ action, heading }, i) => {
              return (
                <li key={action.id} role="presentation">
                  {heading ? <p className={styles.group}>{heading}</p> : null}
                  <div
                    id={`palette-${action.id}`}
                    role="option"
                    aria-selected={i === cursor}
                    className={styles.item}
                    onMouseMove={() => {
                      setCursor(i);
                    }}
                    onClick={() => {
                      run(action);
                    }}
                  >
                    <span>{action.label}</span>
                    {action.shortcut ? <kbd className={styles.kbd}>{action.shortcut}</kbd> : null}
                  </div>
                </li>
              );
            })}
          </ul>
          <footer className={styles.footer}>
            <span>
              <kbd className={styles.kbd}>↑↓</kbd> navigate
            </span>
            <span>
              <kbd className={styles.kbd}>↵</kbd> select
            </span>
            <span>
              <kbd className={styles.kbd}>esc</kbd> close
            </span>
          </footer>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
