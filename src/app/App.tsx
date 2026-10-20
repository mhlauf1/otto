import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useRef, useState } from 'react';
import type { AgentKind, DoctorReport, Workspace } from '../bindings';
import { OttoMark } from '../brand/OttoMark';
import { OttoWordmark } from '../brand/OttoWordmark';
import { Composer, type ComposerHandle } from '../composer/Composer';
import { DoctorPanel } from '../doctor/DoctorPanel';
import { CommandPalette, type PaletteAction } from '../palette/CommandPalette';
import { SessionError, useSessions } from '../sessions/registry';
import { TerminalView } from '../sessions/TerminalView';
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

const NEW_SESSION: { agent: AgentKind; label: string }[] = [
  { agent: 'claude', label: 'Claude' },
  { agent: 'codex', label: 'Codex' },
  { agent: 'shell', label: 'Shell' },
];

/**
 * Main window: workspaces from the core, workspace tint, terminal sessions and the
 * Identity Doctor. The design pass (palette, composer) arrives in M3.
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

  const { sessions, activeByWorkspace, open, close, select } = useSessions();
  const [sessionError, setSessionError] = useState<string | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const composer = useRef<ComposerHandle>(null);
  const activeSessionId = active ? activeByWorkspace[active.id] : undefined;
  const workspaceSessions = sessions.filter((s) => s.info.workspaceId === active?.id);
  const activeSession = sessions.find((s) => s.info.id === activeSessionId);

  // When a sign-in flow ends, re-check who that workspace is signed in as.
  useEffect(() => {
    useSessions.setState({
      onExit: (s) => {
        if (s.info.signIn) {
          void queryClient.invalidateQueries({ queryKey: queryKeys.doctor(s.info.workspaceId) });
        }
      },
    });
  }, [queryClient]);

  const startSession = async (workspaceId: string, agent: AgentKind, signIn = false) => {
    setSessionError(null);
    try {
      await open(workspaceId, agent, signIn);
    } catch (e) {
      setSessionError(e instanceof SessionError ? e.message : 'Couldn’t start the session.');
    }
  };

  // ⌘1…⌘9 switch workspaces (and re-run the Identity Doctor). ⌘T new Claude session,
  // ⌘W close session, ⌘⇧I Identity Doctor (SPEC §4.4, §6.4).
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.metaKey) return;
      if (!e.shiftKey && e.code === 'KeyK') {
        e.preventDefault();
        setPaletteOpen((o) => !o);
        return;
      }
      if (!e.shiftKey && e.code === 'KeyL' && activeSession) {
        e.preventDefault();
        composer.current?.focus();
        return;
      }
      if (e.shiftKey && e.code === 'KeyI') {
        e.preventDefault();
        toggleInspector();
        return;
      }
      if (!e.shiftKey && e.code === 'KeyT' && active) {
        e.preventDefault();
        void startSession(active.id, 'claude');
        return;
      }
      if (!e.shiftKey && e.code === 'KeyW' && activeSessionId) {
        e.preventDefault();
        close(activeSessionId);
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
  });

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

  const paletteActions: PaletteAction[] = [
    ...NEW_SESSION.map(({ agent, label }) => ({
      id: `new-${agent}`,
      group: `New in ${active.name}`,
      label: `New ${label} session`,
      keywords: agent === 'shell' ? 'terminal zsh' : 'agent',
      ...(agent === 'claude' ? { shortcut: '⌘T' } : {}),
      run: () => {
        void startSession(active.id, agent);
      },
    })),
    ...list.map((w, i) => ({
      id: `ws-${w.id}`,
      group: 'Workspaces',
      label: `Switch to ${w.name}`,
      keywords: w.identity.githubUser,
      shortcut: `⌘${String(i + 1)}`,
      run: () => {
        switchTo(w);
      },
    })),
    ...sessions.map((s) => {
      const ws = list.find((w) => w.id === s.info.workspaceId);
      return {
        id: `go-${s.info.id}`,
        group: 'Sessions',
        label: `${s.info.title} · ${ws?.name ?? ''}`,
        keywords: 'jump go',
        run: () => {
          if (ws) setActive(ws.id);
          select(s.info.workspaceId, s.info.id);
        },
      };
    }),
    {
      id: 'doctor',
      group: 'Identity',
      label: `Run Identity Doctor for ${active.name}`,
      keywords: 'check who signed in github',
      shortcut: '⌘⇧I',
      run: () => {
        if (!inspectorOpen) toggleInspector();
        void doctor.refetch();
      },
    },
  ];

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
          <ul className={styles.list}>
            {list.map((w, i) => (
              <li key={w.id}>
                <button
                  type="button"
                  className={styles.row}
                  data-tint={w.tint}
                  aria-current={w.id === active.id ? 'true' : undefined}
                  onClick={() => {
                    switchTo(w);
                  }}
                >
                  <span className={styles.dot} aria-hidden />
                  <span className={styles.rowName}>{w.name}</span>
                  <HealthBadge report={doctors[i]?.data} />
                  <kbd className={styles.kbd}>⌘{i + 1}</kbd>
                </button>
              </li>
            ))}
          </ul>
        </nav>

        <nav aria-label={`${active.name} sessions`}>
          <p className={styles.sectionLabel}>Sessions</p>
          <ul className={styles.list}>
            <li>
              <button
                type="button"
                className={styles.row}
                aria-current={activeSessionId === undefined ? 'true' : undefined}
                onClick={() => {
                  select(active.id, undefined);
                }}
              >
                <span className={styles.rowName}>Home</span>
              </button>
            </li>
            {workspaceSessions.map((s) => (
              <li key={s.info.id}>
                <button
                  type="button"
                  className={styles.row}
                  aria-current={s.info.id === activeSessionId ? 'true' : undefined}
                  onClick={() => {
                    select(active.id, s.info.id);
                  }}
                >
                  <span
                    className={styles.sessionState}
                    data-state={s.exit === undefined ? 'running' : 'done'}
                    aria-label={s.exit === undefined ? 'Running' : 'Ended'}
                    role="img"
                  />
                  <span className={styles.rowName}>{s.info.title}</span>
                </button>
              </li>
            ))}
          </ul>
          <div className={styles.newSession}>
            {NEW_SESSION.map(({ agent, label }) => (
              <button
                key={agent}
                type="button"
                className={styles.chip}
                onClick={() => {
                  void startSession(active.id, agent);
                }}
                title={agent === 'claude' ? 'New Claude session (⌘T)' : `New ${label} session`}
              >
                + {label}
              </button>
            ))}
          </div>
        </nav>

        <div className={styles.identityFooter} data-tint={active.tint}>
          <span className={styles.avatar} aria-hidden>
            {active.name.slice(0, 1)}
          </span>
          <span className={styles.identityText}>
            <span className={styles.identityName}>{active.identity.githubUser}</span>
            <span className={styles.identityEmail}>{active.identity.gitEmail}</span>
          </span>
        </div>
      </aside>

      <main className={styles.main}>
        <div className={styles.titlebar} data-tauri-drag-region>
          <span className={styles.titlebarWorkspace}>
            {active.name}
            {activeSessionId
              ? ` · ${sessions.find((s) => s.info.id === activeSessionId)?.info.title ?? ''}`
              : ''}
          </span>
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

        {sessionError ? (
          <p className={styles.banner} role="alert">
            {sessionError}
          </p>
        ) : null}

        <div className={styles.stage} hidden={activeSessionId === undefined}>
          {/* Every session stays mounted so scrollback survives tab and workspace switches. */}
          {sessions.map((s) => (
            <TerminalView key={s.info.id} session={s} visible={s.info.id === activeSessionId} />
          ))}
          {activeSession ? (
            <Composer
              key={activeSession.info.id}
              ref={composer}
              workspace={active}
              session={activeSession}
            />
          ) : null}
        </div>

        {activeSessionId === undefined ? (
          <section className={styles.hero}>
            <OttoMark
              size={88}
              lit={sideFor(activeIndex)}
              label={`Otto — ${active.name} workspace`}
            />
            <h1 className={styles.headline}>Every identity, signed in. Nothing crossing over.</h1>
            <p className={styles.subhead}>
              You’re in <strong className={styles.accentText}>{active.name}</strong> as{' '}
              {active.identity.githubUser}. Press ⌘T for a Claude session or ⌘K for everything else.
            </p>
            <CoreStatus />
          </section>
        ) : null}
      </main>

      <CommandPalette open={paletteOpen} onOpenChange={setPaletteOpen} actions={paletteActions} />

      {inspectorOpen ? (
        <DoctorPanel
          workspace={active}
          report={doctor}
          onRerun={() => {
            void doctor.refetch();
          }}
          onSignIn={(agent) => {
            void startSession(active.id, agent, true);
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
