import { useEffect } from 'react';
import { OttoMark } from '../brand/OttoMark';
import { OttoWordmark } from '../brand/OttoWordmark';
import { WORKSPACES, useWorkspaceUi } from '../state/workspace';
import styles from './App.module.css';
import { FriendlyError, useAppInfo } from './useAppInfo';

/**
 * M0 hello window: native chrome, tokens in both themes, workspace tint,
 * and one typed round trip to the Rust core.
 */
export function App() {
  const { active, setActive } = useWorkspaceUi();
  const workspace = WORKSPACES.find((w) => w.id === active) ?? WORKSPACES[0];

  // ⌘1 / ⌘2 switch workspaces (SPEC §6.4).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey) return;
      // Match the physical key too, so non-US layouts still get ⌘1/⌘2.
      const match = WORKSPACES.find((w) => w.shortcut === e.key || `Digit${w.shortcut}` === e.code);
      if (match) {
        e.preventDefault();
        setActive(match.id);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
    };
  }, [setActive]);

  return (
    <div className={styles.window} data-workspace={workspace.id} data-testid="window">
      <aside className={styles.sidebar} aria-label="Sidebar">
        <div className={styles.titlebar} data-tauri-drag-region />
        <div className={styles.brand}>
          <OttoMark size={18} lit={workspace.side} />
          <OttoWordmark height={13} />
        </div>

        <nav aria-label="Workspaces">
          <p className={styles.sectionLabel}>Workspaces</p>
          <ul className={styles.workspaceList}>
            {WORKSPACES.map((w) => (
              <li key={w.id}>
                <button
                  type="button"
                  className={styles.workspaceItem}
                  data-workspace={w.id}
                  aria-current={w.id === active ? 'true' : undefined}
                  onClick={() => {
                    setActive(w.id);
                  }}
                >
                  <span className={styles.dot} aria-hidden />
                  <span className={styles.workspaceName}>{w.name}</span>
                  <kbd className={styles.kbd}>⌘{w.shortcut}</kbd>
                </button>
              </li>
            ))}
          </ul>
        </nav>

        <p className={styles.sidebarFooter}>Milestone 0 · Foundations</p>
      </aside>

      <main className={styles.main}>
        <div className={styles.titlebar} data-tauri-drag-region>
          <span className={styles.titlebarWorkspace}>{workspace.name}</span>
        </div>
        <section className={styles.hero}>
          <OttoMark size={88} lit={workspace.side} label={`Otto — ${workspace.name} workspace`} />
          <h1 className={styles.headline}>Every identity, signed in. Nothing crossing over.</h1>
          <p className={styles.subhead}>
            You’re in <strong className={styles.accentText}>{workspace.name}</strong>. Press ⌘1 or
            ⌘2 to switch.
          </p>
          <CoreStatus />
        </section>
      </main>
    </div>
  );
}

function CoreStatus() {
  const info = useAppInfo();

  if (info.isPending) {
    return <p className={styles.status}>Connecting to the core…</p>;
  }
  if (info.isError) {
    const message =
      info.error instanceof FriendlyError ? info.error.message : 'The core didn’t respond.';
    return (
      <p className={styles.status} data-state="error" role="status">
        {message}
      </p>
    );
  }
  const { name, version, os, arch } = info.data;
  return (
    <p className={styles.status} data-state="ok" role="status">
      <span className={styles.statusDot} aria-hidden />
      {name} {version} · {os} · {arch}
    </p>
  );
}
