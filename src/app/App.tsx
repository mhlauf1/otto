import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo } from 'react';
import type { DoctorReport, Workspace } from '../bindings';
import { OttoMark } from '../brand/OttoMark';
import { OttoWordmark } from '../brand/OttoWordmark';
import { DoctorPanel } from '../doctor/DoctorPanel';
import { useWorkspaceUi } from '../state/workspace';
import styles from './App.module.css';
import {
  FriendlyError,
  queryKeys,
  useAllDoctors,
  useAppInfo,
  useDoctor,
  useWorkspaces,
} from './queries';

/** Which half of the mark a workspace lights: the first two workspaces own a half each. */
const sideFor = (index: number) => (index === 0 ? 'left' : index === 1 ? 'right' : 'none');

/**
 * Main window: workspaces from the core, workspace tint, and the Identity Doctor.
 * The full shell (sessions, composer, palette) arrives in M2–M3.
 */
export function App() {
  const workspaces = useWorkspaces();
  const list = useMemo(() => workspaces.data ?? [], [workspaces.data]);
  const { activeId, setActive, inspectorOpen, toggleInspector } = useWorkspaceUi();
  const activeIndex = Math.max(
    0,
    list.findIndex((w) => w.id === activeId),
  );
  const active = list[activeIndex];
  const doctors = useAllDoctors(list);
  const doctor = useDoctor(active?.id);
  const queryClient = useQueryClient();

  // ⌘1…⌘9 switch workspaces and re-run that workspace's Identity Doctor (SPEC §4.4, §6.4).
  // ⌘⇧I toggles the Identity Doctor.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey) return;
      if (e.shiftKey && e.code === 'KeyI') {
        e.preventDefault();
        toggleInspector();
        return;
      }
      const digit = /^Digit([1-9])$/.exec(e.code)?.[1] ?? (/^[1-9]$/.test(e.key) ? e.key : null);
      const target = digit ? list[Number(digit) - 1] : undefined;
      if (target) {
        e.preventDefault();
        setActive(target.id);
        void queryClient.invalidateQueries({ queryKey: queryKeys.doctor(target.id) });
      }
    };
    window.addEventListener('keydown', onKey);
    return () => {
      window.removeEventListener('keydown', onKey);
    };
  }, [list, setActive, toggleInspector, queryClient]);

  if (!active) {
    return (
      <div className={styles.loading} data-testid="window">
        <OttoMark size={40} />
        {workspaces.isError ? (
          <p role="alert">
            {workspaces.error instanceof FriendlyError
              ? workspaces.error.message
              : 'Otto couldn’t load your workspaces.'}
          </p>
        ) : null}
      </div>
    );
  }

  const switchTo = (w: Workspace) => {
    setActive(w.id);
    void queryClient.invalidateQueries({ queryKey: queryKeys.doctor(w.id) });
  };

  return (
    <div
      className={styles.window}
      data-tint={active.tint}
      data-workspace={active.id}
      data-inspector={inspectorOpen ? 'open' : 'closed'}
      data-testid="window"
    >
      <aside className={styles.sidebar} aria-label="Sidebar">
        <div className={styles.titlebar} data-tauri-drag-region />
        <div className={styles.brand}>
          <OttoMark size={18} lit={sideFor(activeIndex)} />
          <OttoWordmark height={13} />
        </div>

        <nav aria-label="Workspaces">
          <p className={styles.sectionLabel}>Workspaces</p>
          <ul className={styles.workspaceList}>
            {list.map((w, i) => (
              <li key={w.id}>
                <button
                  type="button"
                  className={styles.workspaceItem}
                  data-tint={w.tint}
                  aria-current={w.id === active.id ? 'true' : undefined}
                  onClick={() => {
                    switchTo(w);
                  }}
                >
                  <span className={styles.dot} aria-hidden />
                  <span className={styles.workspaceName}>{w.name}</span>
                  <HealthBadge report={doctors[i]?.data} />
                  <kbd className={styles.kbd}>⌘{i + 1}</kbd>
                </button>
              </li>
            ))}
          </ul>
        </nav>

        <p className={styles.sidebarFooter}>Milestone 1 · Isolation core</p>
      </aside>

      <main className={styles.main}>
        <div className={styles.titlebar} data-tauri-drag-region>
          <span className={styles.titlebarWorkspace}>{active.name}</span>
          <button
            type="button"
            className={styles.titlebarButton}
            onClick={toggleInspector}
            aria-pressed={inspectorOpen}
            title="Identity Doctor (⌘⇧I)"
          >
            Identity
          </button>
        </div>
        <section className={styles.hero}>
          <OttoMark
            size={88}
            lit={sideFor(activeIndex)}
            label={`Otto — ${active.name} workspace`}
          />
          <h1 className={styles.headline}>Every identity, signed in. Nothing crossing over.</h1>
          <p className={styles.subhead}>
            You’re in <strong className={styles.accentText}>{active.name}</strong> as{' '}
            {active.identity.githubUser}. Press ⌘1 or ⌘2 to switch.
          </p>
          <CoreStatus />
        </section>
      </main>

      {inspectorOpen ? (
        <DoctorPanel
          workspace={active}
          report={doctor}
          onRerun={() => {
            void doctor.refetch();
          }}
        />
      ) : null}
    </div>
  );
}

/** Tiny ✓ / ✕ next to each workspace, so a red identity is visible from anywhere. */
function HealthBadge({ report }: { report: DoctorReport | undefined }) {
  if (!report) return null;
  return (
    <span
      className={styles.health}
      data-state={report.pushAllowed ? 'ok' : 'blocked'}
      role="img"
      aria-label={report.pushAllowed ? 'Identity verified' : 'Identity problem: pushes blocked'}
    >
      {report.pushAllowed ? '✓' : '✕'}
    </span>
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
