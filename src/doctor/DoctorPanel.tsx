import type { UseQueryResult } from '@tanstack/react-query';
import type { AgentKind, CheckStatus, DoctorReport, Workspace } from '../bindings';
import { FriendlyError } from '../app/queries';
import styles from './DoctorPanel.module.css';

const GLYPH: Record<CheckStatus, string> = { pass: '✓', warn: '!', skipped: '–', fail: '✕' };
const SPOKEN: Record<CheckStatus, string> = {
  pass: 'Passing',
  warn: 'Warning',
  skipped: 'Not used',
  fail: 'Failing',
};

interface DoctorPanelProps {
  workspace: Workspace;
  report: UseQueryResult<DoctorReport>;
  onRerun: () => void;
  /** Open the CLI's own sign-in flow inside this workspace. */
  onSignIn: (agent: AgentKind) => void;
}

/** Checks that can be fixed by signing in from a terminal session. */
const SIGN_IN: Partial<Record<string, AgentKind>> = { claude: 'claude', codex: 'codex' };

/** Identity Doctor inspector (SPEC §4.4). Status is always glyph + word + colour, never colour alone. */
export function DoctorPanel({ workspace, report, onRerun, onSignIn }: DoctorPanelProps) {
  const verdict = report.data
    ? report.data.allGreen
      ? 'All clear. Pushes are allowed.'
      : report.data.pushAllowed
        ? 'Identity verified, so pushes are allowed. Some agents still need setup.'
        : 'Pushes are blocked until the identity checks are fixed.'
    : null;

  return (
    <aside className={styles.panel} aria-label="Identity Doctor">
      <header className={styles.header}>
        <div>
          <h2 className={styles.title}>Identity</h2>
          <p className={styles.subtitle}>{workspace.identity.githubUser}</p>
          <p className={styles.email} title={workspace.identity.gitEmail}>
            {workspace.identity.gitEmail}
          </p>
        </div>
        <button
          type="button"
          className={styles.rerun}
          onClick={onRerun}
          disabled={report.isFetching}
        >
          {report.isFetching ? 'Checking…' : 'Check again'}
        </button>
      </header>

      {report.isError ? (
        <p className={styles.notice} role="status">
          {report.error instanceof FriendlyError
            ? report.error.message
            : 'The Identity Doctor couldn’t run.'}
        </p>
      ) : report.data ? (
        <>
          <p
            className={styles.verdict}
            data-state={report.data.pushAllowed ? 'ok' : 'blocked'}
            role="status"
          >
            {verdict}
          </p>
          <ul className={styles.list}>
            {report.data.checks.map((check) => (
              <li key={check.id} className={styles.item} data-status={check.status}>
                <span className={styles.glyph} aria-label={SPOKEN[check.status]} role="img">
                  {GLYPH[check.status]}
                </span>
                <div className={styles.body}>
                  <p className={styles.label}>{check.label}</p>
                  <p className={styles.detail}>{check.detail}</p>
                  {check.status === 'fail' &&
                  SIGN_IN[check.id] &&
                  check.detail.includes('signed in') ? (
                    <button
                      type="button"
                      className={styles.action}
                      onClick={() => {
                        const agent = SIGN_IN[check.id];
                        if (agent) onSignIn(agent);
                      }}
                    >
                      Sign in to {check.label} as {workspace.name}
                    </button>
                  ) : check.fix && check.status !== 'pass' ? (
                    <p className={styles.fix}>{check.fix}</p>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        </>
      ) : (
        <p className={styles.notice} role="status">
          Checking who {workspace.name} is signed in as…
        </p>
      )}
    </aside>
  );
}
