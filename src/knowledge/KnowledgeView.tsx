import { useQueryClient } from '@tanstack/react-query';
import { isTauri } from '@tauri-apps/api/core';
import { open as pickFiles } from '@tauri-apps/plugin-dialog';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import type { Area, KnowledgeEntry, Workspace } from '../bindings';
import {
  FriendlyError,
  knowledgeActions,
  queryKeys,
  useKnowledge,
  useKnowledgeImage,
  useNote,
  useProjectDoc,
  useProjectDocs,
} from '../app/queries';
import { NameDialog } from '../ui/NameDialog';
import { Markdown } from './Markdown';
import styles from './Knowledge.module.css';

type Selection =
  | { kind: 'note'; area: Area; path: string }
  | { kind: 'doc'; projectPath: string; projectName: string; path: string };

const AREA_BADGE: Record<Area, string> = {
  global: 'Shared by every workspace',
  notes: 'Agents read this',
  private: 'Private: agents don’t read this',
  assets: 'Asset',
};

const SAVE_DELAY_MS = 600;
const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg'];

const errorText = (e: unknown) => (e instanceof Error ? e.message : 'That didn’t work.');

interface KnowledgeViewProps {
  workspace: Workspace;
  onAsk: (includePrivate: boolean) => void;
}

/**
 * Everything a workspace's agents should know (SPEC §7): notes they read, private notes
 * they don't, the projects' own docs, and assets. Ask opens a Claude session over it all.
 */
export function KnowledgeView({ workspace, onAsk }: KnowledgeViewProps) {
  const ws = workspace.id;
  const queryClient = useQueryClient();
  const entries = useKnowledge(ws);
  const docs = useProjectDocs(ws);
  const [selection, setSelection] = useState<Selection>({
    kind: 'note',
    area: 'notes',
    path: 'Overview.md',
  });
  const [startEditing, setStartEditing] = useState(false);
  const [creating, setCreating] = useState<Area | null>(null);
  const [includePrivate, setIncludePrivate] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const list = entries.data ?? [];
  const inArea = (area: Area) => list.filter((e) => e.area === area);
  const isSelected = (area: Area, path: string) =>
    selection.kind === 'note' && selection.area === area && selection.path === path;
  const selectedEntry =
    selection.kind === 'note'
      ? list.find((e) => e.area === selection.area && e.path === selection.path)
      : undefined;

  const refresh = () => {
    void queryClient.invalidateQueries({ queryKey: queryKeys.knowledge(ws), exact: true });
  };

  const addImages = async () => {
    setError(null);
    const picked = await pickFiles({
      multiple: true,
      title: `Add images to ${workspace.name}`,
      filters: [{ name: 'Images', extensions: IMAGE_EXTENSIONS }],
    });
    const files = picked === null ? [] : Array.isArray(picked) ? picked : [picked];
    if (files.length === 0) return;
    try {
      const added = await knowledgeActions.addAssets(ws, files);
      refresh();
      const first = added[0];
      if (first) setSelection({ kind: 'note', area: 'assets', path: first });
    } catch (e) {
      setError(errorText(e));
    }
  };

  const noteRow = (e: KnowledgeEntry) => (
    <li key={`${e.area}:${e.path}`}>
      <button
        type="button"
        className={styles.item}
        aria-current={isSelected(e.area, e.path) ? 'true' : undefined}
        onClick={() => {
          setStartEditing(false);
          setSelection({ kind: 'note', area: e.area, path: e.path });
        }}
      >
        <span className={styles.itemTitle}>
          {e.area === 'private' ? (
            <span className={styles.lock} aria-label="Private" role="img">
              <LockIcon />
            </span>
          ) : null}
          {e.title}
        </span>
        {e.summary ? <span className={styles.itemSummary}>{e.summary}</span> : null}
      </button>
    </li>
  );

  return (
    <section className={styles.page} aria-labelledby="knowledge-title">
      <header className={styles.header}>
        <div className={styles.heading}>
          <h1 id="knowledge-title" className={styles.title}>
            Knowledge
          </h1>
          <p className={styles.subtitle}>
            What {workspace.name}’s agents know. Kept on this Mac, never in a repo.
          </p>
        </div>
        <label className={styles.check}>
          <input
            type="checkbox"
            checked={includePrivate}
            onChange={(e) => {
              setIncludePrivate(e.target.checked);
            }}
          />
          Include private
        </label>
        <button
          type="button"
          className={styles.ask}
          title="Open Claude Code over this workspace’s notes and projects"
          onClick={() => {
            onAsk(includePrivate);
          }}
        >
          Ask {workspace.name}
        </button>
      </header>

      {error ? (
        <p className={styles.error} role="alert">
          {error}
        </p>
      ) : null}

      <div className={styles.split}>
        <nav className={styles.list} aria-label={`${workspace.name} knowledge`}>
          <button
            type="button"
            className={styles.addRow}
            onClick={() => {
              setCreating('notes');
            }}
          >
            <span className={styles.addIcon} aria-hidden>
              +
            </span>
            New note
          </button>

          <ListSection label="Shared">
            <ul className={styles.items}>{inArea('global').map(noteRow)}</ul>
          </ListSection>

          <ListSection label="Notes" hint="Every session gets an index of these">
            <ul className={styles.items}>{inArea('notes').map(noteRow)}</ul>
          </ListSection>

          <ListSection
            label="Private"
            hint="Only in Ask, with Include private"
            onAdd={() => {
              setCreating('private');
            }}
            addLabel="New private note"
          >
            {inArea('private').length > 0 ? (
              <ul className={styles.items}>{inArea('private').map(noteRow)}</ul>
            ) : null}
          </ListSection>

          {(docs.data ?? []).length > 0 ? (
            <ListSection label="From your projects" hint="Read-only, straight from each repo">
              {(docs.data ?? []).map((p) => (
                <details key={p.projectPath} className={styles.project} open>
                  <summary className={styles.projectName}>{p.projectName}</summary>
                  <ul className={styles.items}>
                    {p.docs.map((d) => (
                      <li key={d.path}>
                        <button
                          type="button"
                          className={styles.item}
                          aria-current={
                            selection.kind === 'doc' &&
                            selection.projectPath === p.projectPath &&
                            selection.path === d.path
                              ? 'true'
                              : undefined
                          }
                          onClick={() => {
                            setSelection({
                              kind: 'doc',
                              projectPath: p.projectPath,
                              projectName: p.projectName,
                              path: d.path,
                            });
                          }}
                        >
                          <span className={styles.itemTitle}>{d.title}</span>
                          <span className={styles.itemSummary}>{d.path}</span>
                        </button>
                      </li>
                    ))}
                  </ul>
                </details>
              ))}
            </ListSection>
          ) : null}

          <ListSection
            label="Assets"
            onAdd={isTauri() ? () => void addImages() : undefined}
            addLabel="Add images"
          >
            {inArea('assets').length > 0 ? (
              <ul className={styles.items}>{inArea('assets').map(noteRow)}</ul>
            ) : null}
          </ListSection>
        </nav>

        <article className={styles.reader}>
          {selection.kind === 'doc' ? (
            <DocReader key={`${selection.projectPath}:${selection.path}`} ws={ws} {...selection} />
          ) : selection.area === 'assets' ? (
            <AssetViewer key={selection.path} ws={ws} path={selection.path} />
          ) : (
            <NoteEditor
              key={`${selection.area}:${selection.path}`}
              ws={ws}
              area={selection.area}
              path={selection.path}
              title={selectedEntry?.title ?? selection.path.replace(/\.md$/, '')}
              startEditing={startEditing}
              onSaved={refresh}
            />
          )}
        </article>
      </div>

      {creating ? (
        <NameDialog
          title={creating === 'private' ? 'New private note' : 'New note'}
          description={
            creating === 'private'
              ? 'Agents don’t read private notes unless you include them in Ask.'
              : `${workspace.name}’s agents will see it in their note index.`
          }
          label="Title"
          submitLabel="Create note"
          onClose={() => {
            setCreating(null);
          }}
          onSubmit={async (title) => {
            const path = await knowledgeActions.create(ws, creating, title);
            refresh();
            setStartEditing(true);
            setSelection({ kind: 'note', area: creating, path });
          }}
        />
      ) : null}
    </section>
  );
}

function ListSection({
  label,
  hint,
  onAdd,
  addLabel,
  children,
}: {
  label: string;
  hint?: string;
  onAdd?: (() => void) | undefined;
  addLabel?: string;
  children: ReactNode;
}) {
  return (
    <section className={styles.section} aria-label={label}>
      <div className={styles.sectionHeader}>
        <span className={styles.sectionLabel}>{label}</span>
        {onAdd ? (
          <button
            type="button"
            className={styles.sectionAdd}
            aria-label={addLabel}
            title={addLabel}
            onClick={onAdd}
          >
            +
          </button>
        ) : null}
      </div>
      {hint ? <p className={styles.sectionHint}>{hint}</p> : null}
      {children}
    </section>
  );
}

function NoteEditor({
  ws,
  area,
  path,
  title,
  startEditing,
  onSaved,
}: {
  ws: string;
  area: Area;
  path: string;
  title: string;
  startEditing: boolean;
  onSaved: () => void;
}) {
  const queryClient = useQueryClient();
  const note = useNote(ws, area, path);
  const [editing, setEditing] = useState(startEditing);
  const [draft, setDraft] = useState<string | null>(null);
  const [status, setStatus] = useState<'saved' | 'saving' | 'error'>('saved');
  const [saveError, setSaveError] = useState<string | null>(null);
  const pending = useRef<{ timer: number; text: string } | null>(null);

  const save = async (text: string) => {
    setStatus('saving');
    try {
      await knowledgeActions.save(ws, area, path, text);
      queryClient.setQueryData(queryKeys.note(ws, area, path), text);
      setStatus('saved');
      setSaveError(null);
      onSaved();
    } catch (e) {
      setStatus('error');
      setSaveError(errorText(e));
    }
  };

  // Save whatever is still waiting when the note closes.
  useEffect(
    () => () => {
      const waiting = pending.current;
      if (waiting) {
        window.clearTimeout(waiting.timer);
        queryClient.setQueryData(queryKeys.note(ws, area, path), waiting.text);
        void knowledgeActions.save(ws, area, path, waiting.text).then(onSaved);
      }
    },
    [ws, area, path, queryClient, onSaved],
  );

  const change = (text: string) => {
    setDraft(text);
    setStatus('saving');
    if (pending.current) window.clearTimeout(pending.current.timer);
    const timer = window.setTimeout(() => {
      pending.current = null;
      void save(text);
    }, SAVE_DELAY_MS);
    pending.current = { timer, text };
  };

  const text = draft ?? note.data ?? '';

  return (
    <>
      <div className={styles.readerHeader}>
        <h2 className={styles.readerTitle}>{title}</h2>
        <span className={styles.badge} data-area={area}>
          {AREA_BADGE[area]}
        </span>
        <span className={styles.status} role="status">
          {editing
            ? status === 'saving'
              ? 'Saving…'
              : status === 'error'
                ? 'Not saved'
                : 'Saved'
            : ''}
        </span>
        <div className={styles.toggle} role="group" aria-label="View">
          <button
            type="button"
            aria-pressed={!editing}
            onClick={() => {
              setEditing(false);
            }}
          >
            Read
          </button>
          <button
            type="button"
            aria-pressed={editing}
            onClick={() => {
              setEditing(true);
            }}
          >
            Edit
          </button>
        </div>
      </div>
      {saveError ? (
        <p className={styles.error} role="alert">
          {saveError}
        </p>
      ) : null}
      {note.isError ? (
        <p className={styles.error} role="alert">
          {note.error instanceof FriendlyError ? note.error.message : 'Couldn’t open this note.'}
        </p>
      ) : note.isPending ? (
        <p className={styles.muted}>Opening…</p>
      ) : editing ? (
        <textarea
          className={styles.editor}
          aria-label={`Edit ${title}`}
          value={text}
          autoFocus
          spellCheck
          onChange={(e) => {
            change(e.target.value);
          }}
        />
      ) : (
        <div className={styles.readerBody}>
          <Markdown text={text} source={{ workspaceId: ws, area, path }} />
        </div>
      )}
    </>
  );
}

function DocReader({
  ws,
  projectPath,
  projectName,
  path,
}: {
  ws: string;
  projectPath: string;
  projectName: string;
  path: string;
}) {
  const doc = useProjectDoc(ws, projectPath, path);
  return (
    <>
      <div className={styles.readerHeader}>
        <h2 className={styles.readerTitle}>
          {projectName} · {path}
        </h2>
        <span className={styles.badge}>Read-only, from the repo</span>
      </div>
      {doc.isError ? (
        <p className={styles.error} role="alert">
          {doc.error instanceof FriendlyError ? doc.error.message : 'Couldn’t open this doc.'}
        </p>
      ) : (
        <div className={styles.readerBody}>
          <Markdown text={doc.data ?? ''} />
        </div>
      )}
    </>
  );
}

function AssetViewer({ ws, path }: { ws: string; path: string }) {
  const image = useKnowledgeImage(ws, 'assets', path);
  const snippet = `![${path.replace(/\.[^.]+$/, '')}](../assets/${encodeURI(path)})`;
  const [copied, setCopied] = useState(false);
  return (
    <>
      <div className={styles.readerHeader}>
        <h2 className={styles.readerTitle}>{path}</h2>
        <span className={styles.badge}>Asset</span>
      </div>
      <div className={styles.readerBody}>
        {image.data ? (
          <img src={image.data} alt={path} className={styles.image} />
        ) : (
          <p className={styles.muted}>
            {image.isPending ? 'Loading…' : 'Couldn’t show this image.'}
          </p>
        )}
        <p className={styles.muted}>Use it in a note:</p>
        <div className={styles.snippet}>
          <code>{snippet}</code>
          <button
            type="button"
            className={styles.copy}
            onClick={() => {
              void navigator.clipboard.writeText(snippet).then(() => {
                setCopied(true);
              });
            }}
          >
            {copied ? 'Copied' : 'Copy'}
          </button>
        </div>
      </div>
    </>
  );
}

function LockIcon() {
  return (
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden>
      <rect x="1.5" y="4.5" width="7" height="5" rx="1" fill="currentColor" />
      <path d="M3 4.5V3a2 2 0 0 1 4 0v1.5" fill="none" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}
