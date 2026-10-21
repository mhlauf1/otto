import type { Workspace } from '../bindings';
import styles from './Overview.module.css';

/**
 * Where the workspace's knowledge will live (M4, Phase B of the 2026-10-20 plan).
 * Until then this page says plainly what's coming, so the sidebar shape is final now.
 */
export function Knowledge({ workspace }: { workspace: Workspace }) {
  return (
    <section className={styles.page} aria-labelledby="knowledge-title">
      <header className={styles.header}>
        <div className={styles.heading}>
          <h1 id="knowledge-title" className={styles.title}>
            Knowledge
          </h1>
          <p className={styles.subtitle}>
            Coming next (M4). Everything {workspace.name}’s agents should know, kept on this Mac.
          </p>
        </div>
      </header>
      <div className={styles.card}>
        <dl className={styles.glossary}>
          <div>
            <dt>Notes</dt>
            <dd>
              Markdown notes, diagrams and images. Every {workspace.name} session reads them, and no
              other workspace can.
            </dd>
          </div>
          <div>
            <dt>Private</dt>
            <dd>Notes only you read. Agents see them only when you say so.</dd>
          </div>
          <div>
            <dt>From your projects</dt>
            <dd>Each project’s CLAUDE.md, AGENTS.md and docs, gathered in one place.</dd>
          </div>
          <div>
            <dt>Ask</dt>
            <dd>A Claude Code session that can read all of the above to answer questions.</dd>
          </div>
        </dl>
      </div>
    </section>
  );
}
