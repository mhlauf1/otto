import type { AgentKind, DoctorReport, Project, ProjectGroup, Workspace } from '../bindings';
import { OttoMark } from '../brand/OttoMark';
import { OttoWordmark } from '../brand/OttoWordmark';
import { NewSessionMenu } from '../sessions/NewSessionMenu';
import type { LiveSession } from '../sessions/registry';
import type { WorkspacePage } from '../state/workspace';
import { ProjectTree } from './ProjectTree';
import { sideFor } from './side';
import styles from './Sidebar.module.css';

interface SidebarProps {
  workspaces: Workspace[];
  active: Workspace;
  activeIndex: number;
  /** Identity Doctor report per workspace, in `workspaces` order. */
  reports: (DoctorReport | undefined)[];
  onSwitch: (w: Workspace) => void;
  /** The page shown when no session is selected. */
  page: WorkspacePage;
  onPage: (page: WorkspacePage) => void;
  projects: Project[];
  groups: ProjectGroup[];
  sessions: LiveSession[];
  activeSessionId: string | undefined;
  currentProject: Project | undefined;
  onSelectSession: (id: string) => void;
  onSetProject: (path: string | undefined) => void;
  onNewSession: (agent: AgentKind, projectPath: string | undefined) => void;
  onResume: (conversationId: string, projectPath: string | undefined) => void;
  onResumePaused: (id: string) => void;
  onDismissPaused: (id: string) => void;
  onImport: (group?: ProjectGroup) => void;
  onGroupsChanged: () => void;
  onError: (message: string) => void;
}

/**
 * The sidebar, top to bottom: which identity you are (workspace switcher), the
 * workspace's own pages, then everything you work on (groups → projects → sessions).
 */
export function Sidebar(props: SidebarProps) {
  const { active, activeIndex, page, activeSessionId, currentProject } = props;
  return (
    <aside className={styles.sidebar} aria-label="Sidebar">
      <div className={styles.titlebar} data-tauri-drag-region />
      <div className={styles.brand}>
        <OttoMark size={18} lit={sideFor(activeIndex)} />
        <OttoWordmark height={13} />
        {import.meta.env.DEV ? (
          <span
            className={styles.devTag}
            title="Running from the code (pnpm dev), with its own data"
          >
            Dev
          </span>
        ) : null}
      </div>

      <WorkspaceSwitcher
        workspaces={props.workspaces}
        active={active}
        reports={props.reports}
        onSwitch={props.onSwitch}
      />

      <nav aria-label={`${active.name} pages`}>
        <ul className={styles.list}>
          {(['overview', 'knowledge'] as const).map((p) => (
            <li key={p}>
              <button
                type="button"
                className={styles.row}
                aria-current={activeSessionId === undefined && page === p ? 'page' : undefined}
                onClick={() => {
                  props.onPage(p);
                }}
              >
                <span className={styles.rowName}>
                  {p === 'overview' ? 'Overview' : 'Knowledge'}
                </span>
              </button>
            </li>
          ))}
        </ul>
      </nav>

      <nav aria-label={`${active.name} projects`} className={styles.scroll}>
        <ProjectTree
          workspace={active}
          projects={props.projects}
          groups={props.groups}
          sessions={props.sessions}
          activeSessionId={activeSessionId}
          currentProjectPath={currentProject?.path}
          onSelectSession={props.onSelectSession}
          onSetProject={props.onSetProject}
          onNewSession={props.onNewSession}
          onResume={props.onResume}
          onResumePaused={props.onResumePaused}
          onDismissPaused={props.onDismissPaused}
          onImport={props.onImport}
          onChanged={props.onGroupsChanged}
          onError={props.onError}
        />
      </nav>

      <NewSessionMenu
        target={`${active.name} · ${currentProject?.name ?? 'General'}`}
        onPick={(agent) => {
          props.onNewSession(agent, currentProject?.path);
        }}
      />

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
  );
}

interface WorkspaceSwitcherProps {
  workspaces: Workspace[];
  active: Workspace;
  reports: (DoctorReport | undefined)[];
  onSwitch: (w: Workspace) => void;
}

/** One segment per identity. Its dot, name and ✓/✕ stay visible whichever is active. */
function WorkspaceSwitcher({ workspaces, active, reports, onSwitch }: WorkspaceSwitcherProps) {
  return (
    <nav aria-label="Workspaces" className={styles.switcher}>
      {workspaces.map((w, i) => (
        <button
          key={w.id}
          type="button"
          className={styles.segment}
          data-tint={w.tint}
          aria-current={w.id === active.id ? 'true' : undefined}
          title={`${w.name} · ${w.identity.githubUser} (⌘${String(i + 1)})`}
          onClick={() => {
            onSwitch(w);
          }}
        >
          <span className={styles.dot} aria-hidden />
          <span className={styles.segmentName}>{w.name}</span>
          <HealthBadge report={reports[i]} />
        </button>
      ))}
    </nav>
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
