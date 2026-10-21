import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import type { AgentKind, ProjectGroup, Workspace } from '../bindings';
import { OttoMark } from '../brand/OttoMark';
import { DoctorPanel } from '../doctor/DoctorPanel';
import { Knowledge } from '../overview/Knowledge';
import { Overview } from '../overview/Overview';
import { CommandPalette, type PaletteAction } from '../palette/CommandPalette';
import { ImportProjects } from '../projects/ImportProjects';
import { sessionLabels } from '../sessions/labels';
import { SessionError, useSessions } from '../sessions/registry';
import { TerminalView } from '../sessions/TerminalView';
import { ClaudeSetup } from '../setup/ClaudeSetup';
import { Sidebar } from '../sidebar/Sidebar';
import { useWorkspaceUi, type WorkspacePage } from '../state/workspace';
import styles from './App.module.css';
import {
  FriendlyError,
  queryKeys,
  useAllDoctors,
  useDoctor,
  useProjectGroups,
  useProjects,
  useWorkspaces,
} from './queries';

const NEW_SESSION: { agent: AgentKind; label: string }[] = [
  { agent: 'claude', label: 'Claude' },
  { agent: 'codex', label: 'Codex' },
  { agent: 'shell', label: 'Shell' },
];

/**
 * Main window: sidebar (workspace → groups → projects → sessions), the selected session's
 * terminal or a workspace page, and the Identity Doctor. The terminal is the only input.
 */
export function App() {
  const workspaces = useWorkspaces();
  const list = useMemo(() => workspaces.data ?? [], [workspaces.data]);
  const {
    activeId,
    setActive,
    inspectorOpen,
    toggleInspector,
    projectByWorkspace,
    setProject,
    pageByWorkspace,
    setPage,
  } = useWorkspaceUi();
  const activeIndex = Math.max(
    0,
    list.findIndex((w) => w.id === activeId),
  );
  const active = list[activeIndex];
  const doctors = useAllDoctors(list);
  const projects = useProjects(active?.id);
  const groups = useProjectGroups(active?.id);
  const [importOpen, setImportOpen] = useState(false);
  const [importInto, setImportInto] = useState<ProjectGroup | undefined>(undefined);
  const [setupOpen, setSetupOpen] = useState(false);
  const activeProject = active
    ? (projects.data ?? []).find((p) => p.path === projectByWorkspace[active.id])
    : undefined;
  const doctor = useDoctor(active?.id);
  const queryClient = useQueryClient();

  const { sessions, activeByWorkspace, open, close, select } = useSessions();
  const [sessionError, setSessionError] = useState<string | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
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

  // A session opening or closing changes which earlier conversations to list.
  const sessionCount = sessions.length;
  useEffect(() => {
    void queryClient.invalidateQueries({ queryKey: ['past'] });
  }, [queryClient, sessionCount]);

  /**
   * Every caller says where the session runs: `projectPath`, or `undefined` for General.
   * Sign-in flows always run in the home folder. `resumeId` continues a Claude conversation.
   */
  const startSession = async (
    workspaceId: string,
    agent: AgentKind,
    {
      signIn = false,
      projectPath,
      resumeId,
    }: { signIn?: boolean; projectPath: string | undefined; resumeId?: string },
  ) => {
    setSessionError(null);
    try {
      await open(workspaceId, agent, {
        signIn,
        projectPath: signIn ? undefined : projectPath,
        resumeId,
      });
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
      if (e.shiftKey && e.code === 'KeyI') {
        e.preventDefault();
        toggleInspector();
        return;
      }
      if (!e.shiftKey && e.code === 'KeyT' && active) {
        e.preventDefault();
        void startSession(active.id, 'claude', { projectPath: activeProject?.path });
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

  const page: WorkspacePage = pageByWorkspace[active.id] ?? 'overview';
  const showPage = (p: WorkspacePage) => {
    setPage(active.id, p);
    select(active.id, undefined);
  };
  const projectList = projects.data ?? [];
  const projectNameFor = (path: string | null) =>
    projectList.find((p) => p.path === path)?.name ?? 'General';
  const activeLabel = activeSession
    ? sessionLabels(
        workspaceSessions.filter((s) => s.info.projectPath === activeSession.info.projectPath),
      ).get(activeSession.info.id)
    : undefined;
  const crumbs = activeSession
    ? [active.name, projectNameFor(activeSession.info.projectPath), activeLabel]
    : [active.name, page === 'overview' ? 'Overview' : 'Knowledge'];

  const paletteActions: PaletteAction[] = [
    ...NEW_SESSION.map(({ agent, label }) => ({
      id: `new-${agent}`,
      group: `New in ${active.name}`,
      label: `New ${label} session`,
      keywords: agent === 'shell' ? 'terminal zsh' : 'agent',
      ...(agent === 'claude' ? { shortcut: '⌘T' } : {}),
      run: () => {
        void startSession(active.id, agent, { projectPath: activeProject?.path });
      },
    })),
    {
      id: 'overview',
      group: active.name,
      label: `Go to ${active.name} Overview`,
      keywords: 'home start dashboard',
      run: () => {
        showPage('overview');
      },
    },
    {
      id: 'knowledge',
      group: active.name,
      label: `Open ${active.name} Knowledge`,
      keywords: 'notes context docs',
      run: () => {
        showPage('knowledge');
      },
    },
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
        label: [ws?.name, projectNameFor(s.info.projectPath), s.info.title].join(' · '),
        keywords: 'jump go',
        run: () => {
          if (ws) setActive(ws.id);
          select(s.info.workspaceId, s.info.id);
        },
      };
    }),
    ...(projects.data ?? []).map((p) => ({
      id: `project-${p.path}`,
      group: `${active.name} projects`,
      label: `New sessions open in ${p.name}`,
      keywords: 'use project open cd folder repo',
      run: () => {
        setProject(active.id, p.path);
      },
    })),
    {
      id: 'import-projects',
      group: `${active.name} projects`,
      label: 'Import projects…',
      keywords: 'add discover scan repos folder',
      run: () => {
        setImportOpen(true);
      },
    },
    {
      id: 'claude-setup',
      group: 'Identity',
      label: 'Bring in Claude setup…',
      keywords: 'import skills plugins mcp settings claude.md agents commands',
      run: () => {
        setSetupOpen(true);
      },
    },
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
      <Sidebar
        workspaces={list}
        active={active}
        activeIndex={activeIndex}
        reports={doctors.map((d) => d.data)}
        onSwitch={switchTo}
        page={page}
        onPage={showPage}
        projects={projectList}
        groups={groups.data ?? []}
        sessions={workspaceSessions}
        activeSessionId={activeSessionId}
        currentProject={activeProject}
        onSelectSession={(id) => {
          select(active.id, id);
        }}
        onSetProject={(path) => {
          setProject(active.id, path);
        }}
        onNewSession={(agent, projectPath) => {
          void startSession(active.id, agent, { projectPath });
        }}
        onResume={(resumeId, projectPath) => {
          void startSession(active.id, 'claude', { projectPath, resumeId });
        }}
        onImport={(group) => {
          setImportInto(group);
          setImportOpen(true);
        }}
        onGroupsChanged={() => {
          void queryClient.invalidateQueries({ queryKey: queryKeys.groups(active.id) });
          void queryClient.invalidateQueries({ queryKey: queryKeys.projects(active.id) });
        }}
        onError={setSessionError}
      />

      <main className={styles.main}>
        <div className={styles.titlebar} data-tauri-drag-region>
          <span className={styles.titlebarCrumbs}>
            {crumbs.map((c, i) => (
              <span key={i} className={i === 0 ? styles.titlebarWorkspace : undefined}>
                {i > 0 ? ' · ' : ''}
                {c}
              </span>
            ))}
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
        </div>

        {activeSessionId === undefined && page === 'overview' ? (
          <Overview
            workspace={active}
            workspaceIndex={activeIndex}
            report={doctor.data}
            projects={projectList}
            groups={groups.data ?? []}
            sessions={workspaceSessions}
            onOpenSession={(id) => {
              select(active.id, id);
            }}
            onNewClaude={(projectPath) => {
              if (projectPath) setProject(active.id, projectPath);
              void startSession(active.id, 'claude', { projectPath });
            }}
            onShowIdentity={() => {
              if (!inspectorOpen) toggleInspector();
            }}
          />
        ) : null}
        {activeSessionId === undefined && page === 'knowledge' ? (
          <Knowledge workspace={active} />
        ) : null}
      </main>

      <CommandPalette open={paletteOpen} onOpenChange={setPaletteOpen} actions={paletteActions} />
      <ImportProjects
        open={importOpen}
        onOpenChange={(o) => {
          setImportOpen(o);
          if (!o) {
            setImportInto(undefined);
            void queryClient.invalidateQueries({ queryKey: queryKeys.groups(active.id) });
          }
        }}
        workspaces={list}
        target={
          importInto
            ? { workspaceId: active.id, groupId: importInto.id, groupName: importInto.name }
            : undefined
        }
      />
      <ClaudeSetup open={setupOpen} onOpenChange={setSetupOpen} workspaces={list} />

      {inspectorOpen ? (
        <DoctorPanel
          workspace={active}
          report={doctor}
          onRerun={() => {
            void doctor.refetch();
          }}
          onClaudeSetup={() => {
            setSetupOpen(true);
          }}
          onSignIn={(agent) => {
            void startSession(active.id, agent, { signIn: true, projectPath: undefined });
          }}
        />
      ) : null}
    </div>
  );
}
