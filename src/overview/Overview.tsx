import type { DoctorReport, Project, ProjectGroup, Workspace } from '../bindings';
import { FriendlyError, useAppInfo } from '../app/queries';
import { OttoMark } from '../brand/OttoMark';
import { sessionLabels } from '../sessions/labels';
import type { LiveSession } from '../sessions/registry';
import { sideFor } from '../sidebar/side';
import styles from './Overview.module.css';

/** SPEC §2 glossary, shown where Mike lands so the words always mean one thing. */
const GLOSSARY: { term: string; meaning: (ws: string) => string }[] = [
  {
    term: 'Workspace',
    meaning: (ws) =>
      `One identity: its logins, git author and GitHub account. You’re in ${ws}. Nothing crosses between workspaces.`,
  },
  {
    term: 'Project',
    meaning: (ws) => `A repo or folder that belongs to ${ws} only. Groups sort projects.`,
  },
  {
    term: 'Session',
    meaning: () => 'One running Claude Code, Codex or Shell. It sits under the project it runs in.',
  },
  {
    term: 'General',
    meaning: () => 'Sessions with no project. They start in your home folder.',
  },
];

interface OverviewProps {
  workspace: Workspace;
  workspaceIndex: number;
  report: DoctorReport | undefined;
  projects: Project[];
  groups: ProjectGroup[];
  sessions: LiveSession[];
  onOpenSession: (id: string) => void;
  onNewClaude: (projectPath: string | undefined) => void;
  onShowIdentity: () => void;
}

/** The workspace landing page: who you are here, what's running, where you can work. */
export function Overview({
  workspace,
  workspaceIndex,
  report,
  projects,
  groups,
  sessions,
  onOpenSession,
  onNewClaude,
  onShowIdentity,
}: OverviewProps) {
  const labels = sessionLabels(sessions);
  const projectName = (path: string | null) =>
    projects.find((p) => p.path === path)?.name ?? 'General';
  const groupName = (id: string | null) => groups.find((g) => g.id === id)?.name;

  return (
    <section className={styles.page} aria-labelledby="overview-title">
      <header className={styles.header}>
        <OttoMark size={40} lit={sideFor(workspaceIndex)} label={`Otto — ${workspace.name}`} />
        <div className={styles.heading}>
          <h1 id="overview-title" className={styles.title}>
            {workspace.name}
          </h1>
          <p className={styles.subtitle}>
            Signed in as {workspace.identity.githubUser} · {workspace.identity.gitEmail}
          </p>
        </div>
        <button
          type="button"
          className={styles.health}
          data-state={report === undefined ? 'unknown' : report.pushAllowed ? 'ok' : 'blocked'}
          onClick={onShowIdentity}
        >
          {report === undefined
            ? 'Checking identity…'
            : report.pushAllowed
              ? '✓ All clear'
              : '✕ Pushes blocked: see Identity'}
        </button>
      </header>

      <div className={styles.columns}>
        <div className={styles.column}>
          <h2 className={styles.sectionTitle}>Running now</h2>
          {sessions.length === 0 ? (
            <p className={styles.muted}>
              Nothing running. Press ⌘T for Claude Code in the selected project.
            </p>
          ) : (
            <ul className={styles.list}>
              {sessions.map((s) => (
                <li key={s.info.id}>
                  <button
                    type="button"
                    className={styles.item}
                    onClick={() => {
                      onOpenSession(s.info.id);
                    }}
                  >
                    <span
                      className={styles.state}
                      data-state={s.exit === undefined ? 'running' : 'done'}
                      aria-hidden
                    />
                    <span className={styles.itemName}>{labels.get(s.info.id)}</span>
                    <span className={styles.itemMeta}>{projectName(s.info.projectPath)}</span>
                  </button>
                </li>
              ))}
            </ul>
          )}

          <h2 className={styles.sectionTitle}>Projects</h2>
          {projects.length === 0 ? (
            <p className={styles.muted}>
              No projects yet. Use Import projects… in the sidebar to add {workspace.name} repos.
            </p>
          ) : (
            <ul className={styles.list}>
              {projects.map((p) => (
                <li key={p.path} className={styles.projectItem}>
                  <span className={styles.itemName}>{p.name}</span>
                  <span className={styles.itemMeta}>{groupName(p.groupId) ?? ''}</span>
                  <button
                    type="button"
                    className={styles.action}
                    onClick={() => {
                      onNewClaude(p.path);
                    }}
                  >
                    Open Claude Code
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <aside className={styles.card} aria-labelledby="whats-what">
          <h2 id="whats-what" className={styles.sectionTitle}>
            What’s what
          </h2>
          <dl className={styles.glossary}>
            {GLOSSARY.map(({ term, meaning }) => (
              <div key={term}>
                <dt>{term}</dt>
                <dd>{meaning(workspace.name)}</dd>
              </div>
            ))}
          </dl>
        </aside>
      </div>

      <CoreStatus />
    </section>
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
