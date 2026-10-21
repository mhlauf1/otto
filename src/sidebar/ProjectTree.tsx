import * as Menu from '@radix-ui/react-dropdown-menu';
import { useState } from 'react';
import type { AgentKind, PastConversation, Project, ProjectGroup, Workspace } from '../bindings';
import { groupActions, usePastConversations } from '../app/queries';
import { NewSessionMenu } from '../sessions/NewSessionMenu';
import { ago, sessionLabels } from '../sessions/labels';
import type { LiveSession } from '../sessions/registry';
import { useWorkspaceUi } from '../state/workspace';
import { NameDialog } from '../ui/NameDialog';
import menu from '../sessions/NewSessionMenu.module.css';
import styles from './Sidebar.module.css';

interface ProjectTreeProps {
  workspace: Workspace;
  projects: Project[];
  groups: ProjectGroup[];
  /** This workspace's live sessions. */
  sessions: LiveSession[];
  activeSessionId: string | undefined;
  /** The project new sessions open in; `undefined` means General. */
  currentProjectPath: string | undefined;
  onSelectSession: (id: string) => void;
  onSetProject: (path: string | undefined) => void;
  onNewSession: (agent: AgentKind, projectPath: string | undefined) => void;
  /** Continue an earlier Claude conversation from this project (or General). */
  onResume: (conversationId: string, projectPath: string | undefined) => void;
  /** Open the import sheet, importing straight into `group` when given. */
  onImport: (group?: ProjectGroup) => void;
  /** Group data changed in the core; refetch it. */
  onChanged: () => void;
  onError: (message: string) => void;
}

type Naming =
  | { kind: 'create'; movePath?: string }
  | { kind: 'rename'; group: ProjectGroup }
  | { kind: 'renameProject'; project: Project };

const errorText = (e: unknown) => (e instanceof Error ? e.message : 'That didn’t work.');

/**
 * Groups → projects → sessions, then General for sessions with no project (ADR-017).
 * Every session sits under the place it runs, so where it lives is never in doubt.
 */
export function ProjectTree({
  workspace,
  projects,
  groups,
  sessions,
  activeSessionId,
  currentProjectPath,
  onSelectSession,
  onSetProject,
  onNewSession,
  onResume,
  onImport,
  onChanged,
  onError,
}: ProjectTreeProps) {
  const { collapsed, toggleCollapsed } = useWorkspaceUi();
  const [naming, setNaming] = useState<Naming | null>(null);
  const ws = workspace.id;

  const known = new Set(projects.map((p) => p.path));
  const sessionsIn = (path: string | undefined) =>
    sessions.filter((s) =>
      path === undefined
        ? s.info.projectPath === null || !known.has(s.info.projectPath)
        : s.info.projectPath === path,
    );
  const ungrouped = projects.filter(
    (p) => p.groupId === null || !groups.some((g) => g.id === p.groupId),
  );

  const run = (action: () => Promise<void>) => {
    action().then(onChanged, (e: unknown) => {
      onError(errorText(e));
    });
  };

  const moveGroup = (index: number, by: -1 | 1) => {
    const ids = groups.map((g) => g.id);
    const [moved] = ids.splice(index, 1);
    if (moved === undefined) return;
    ids.splice(index + by, 0, moved);
    run(() => groupActions.reorder(ws, ids));
  };

  const projectNode = (p: Project) => (
    <ProjectNode
      key={p.path}
      project={p}
      workspace={workspace}
      groups={groups}
      sessions={sessionsIn(p.path)}
      activeSessionId={activeSessionId}
      current={p.path === currentProjectPath}
      expanded={!collapsed[`project:${p.path}`]}
      onToggle={() => {
        toggleCollapsed(`project:${p.path}`);
      }}
      onSelect={() => {
        onSetProject(p.path);
        if (collapsed[`project:${p.path}`]) toggleCollapsed(`project:${p.path}`);
      }}
      onSelectSession={onSelectSession}
      onNewSession={(agent) => {
        onSetProject(p.path);
        onNewSession(agent, p.path);
      }}
      onResume={(id) => {
        onSetProject(p.path);
        onResume(id, p.path);
      }}
      onMove={(groupId) => {
        run(() => groupActions.moveProject(ws, p.path, groupId));
      }}
      onNewGroup={() => {
        setNaming({ kind: 'create', movePath: p.path });
      }}
      onRename={() => {
        setNaming({ kind: 'renameProject', project: p });
      }}
    />
  );

  return (
    <div className={styles.tree}>
      {groups.map((g, i) => {
        const inGroup = projects.filter((p) => p.groupId === g.id);
        const open = !collapsed[`group:${g.id}`];
        return (
          <section key={g.id} className={styles.section} aria-label={g.name}>
            <div className={styles.sectionHeader}>
              <button
                type="button"
                className={styles.sectionToggle}
                aria-expanded={open}
                onClick={() => {
                  toggleCollapsed(`group:${g.id}`);
                }}
              >
                {g.name}
              </button>
              <AddToGroup
                group={g}
                projects={projects}
                groups={groups}
                variant="icon"
                onMove={(path) => {
                  run(() => groupActions.moveProject(ws, path, g.id));
                }}
                onImport={() => {
                  onImport(g);
                }}
              />
              <Menu.Root>
                <Menu.Trigger className={styles.moreButton} aria-label={`${g.name} options`}>
                  ···
                </Menu.Trigger>
                <Menu.Portal>
                  <Menu.Content className={menu.menu} side="bottom" align="end" sideOffset={4}>
                    <Menu.Item
                      className={menu.item}
                      onSelect={() => {
                        setNaming({ kind: 'rename', group: g });
                      }}
                    >
                      Rename…
                    </Menu.Item>
                    {i > 0 ? (
                      <Menu.Item
                        className={menu.item}
                        onSelect={() => {
                          moveGroup(i, -1);
                        }}
                      >
                        Move up
                      </Menu.Item>
                    ) : null}
                    {i < groups.length - 1 ? (
                      <Menu.Item
                        className={menu.item}
                        onSelect={() => {
                          moveGroup(i, 1);
                        }}
                      >
                        Move down
                      </Menu.Item>
                    ) : null}
                    <Menu.Separator className={styles.menuSeparator} />
                    <Menu.Item
                      className={menu.item}
                      onSelect={() => {
                        run(() => groupActions.remove(ws, g.id));
                      }}
                    >
                      <span className={menu.text}>
                        <span>Delete group</span>
                        <span className={menu.detail}>Its projects stay in {workspace.name}</span>
                      </span>
                    </Menu.Item>
                  </Menu.Content>
                </Menu.Portal>
              </Menu.Root>
            </div>
            {open ? (
              inGroup.length > 0 ? (
                <ul className={styles.list}>{inGroup.map(projectNode)}</ul>
              ) : (
                <AddToGroup
                  group={g}
                  projects={projects}
                  groups={groups}
                  variant="row"
                  onMove={(path) => {
                    run(() => groupActions.moveProject(ws, path, g.id));
                  }}
                  onImport={() => {
                    onImport(g);
                  }}
                />
              )
            ) : null}
          </section>
        );
      })}

      {ungrouped.length > 0 || groups.length === 0 ? (
        <section className={styles.section} aria-label="Projects">
          <div className={styles.sectionHeader}>
            <span className={styles.sectionLabel}>
              {groups.length > 0 ? 'Other projects' : 'Projects'}
            </span>
          </div>
          {ungrouped.length > 0 ? (
            <ul className={styles.list}>{ungrouped.map(projectNode)}</ul>
          ) : null}
        </section>
      ) : null}

      <ul className={styles.list} aria-label="Add">
        <li>
          <button
            type="button"
            className={styles.addRow}
            onClick={() => {
              onImport();
            }}
          >
            <span className={styles.addIcon} aria-hidden>
              +
            </span>
            Import projects…
          </button>
        </li>
        <li>
          <button
            type="button"
            className={styles.addRow}
            onClick={() => {
              setNaming({ kind: 'create' });
            }}
          >
            <span className={styles.addIcon} aria-hidden>
              +
            </span>
            New group…
          </button>
        </li>
      </ul>

      <section className={styles.section} aria-label="General">
        <ul className={styles.list}>
          <GeneralNode
            workspace={workspace}
            sessions={sessionsIn(undefined)}
            activeSessionId={activeSessionId}
            current={currentProjectPath === undefined}
            onSelect={() => {
              onSetProject(undefined);
            }}
            onSelectSession={onSelectSession}
            onNewSession={(agent) => {
              onSetProject(undefined);
              onNewSession(agent, undefined);
            }}
            onResume={(id) => {
              onSetProject(undefined);
              onResume(id, undefined);
            }}
          />
        </ul>
      </section>

      {naming?.kind === 'renameProject' ? (
        <NameDialog
          title="Rename project"
          description={`Shown in Otto only. The folder stays ${naming.project.folderName}.`}
          label="Project name"
          initial={naming.project.name}
          submitLabel="Rename"
          onClose={() => {
            setNaming(null);
          }}
          onSubmit={async (name) => {
            await groupActions.renameProject(ws, naming.project.path, name);
            onChanged();
          }}
        />
      ) : naming ? (
        <NameDialog
          title={naming.kind === 'rename' ? 'Rename group' : `New group in ${workspace.name}`}
          {...(naming.kind === 'create'
            ? { description: 'Groups sort projects inside this workspace, like Clients or Studio.' }
            : {})}
          label="Group name"
          initial={naming.kind === 'rename' ? naming.group.name : ''}
          submitLabel={naming.kind === 'rename' ? 'Rename' : 'Create group'}
          onClose={() => {
            setNaming(null);
          }}
          onSubmit={async (name) => {
            if (naming.kind === 'rename') {
              await groupActions.rename(ws, naming.group.id, name);
            } else {
              const group = await groupActions.create(ws, name);
              if (naming.movePath) await groupActions.moveProject(ws, naming.movePath, group.id);
            }
            onChanged();
          }}
        />
      ) : null}
    </div>
  );
}

interface ProjectNodeProps {
  project: Project;
  workspace: Workspace;
  groups: ProjectGroup[];
  sessions: LiveSession[];
  activeSessionId: string | undefined;
  current: boolean;
  expanded: boolean;
  onToggle: () => void;
  onSelect: () => void;
  onSelectSession: (id: string) => void;
  onNewSession: (agent: AgentKind) => void;
  onResume: (conversationId: string) => void;
  onMove: (groupId: string | null) => void;
  onNewGroup: () => void;
  onRename: () => void;
}

function ProjectNode({
  project: p,
  workspace,
  groups,
  sessions,
  activeSessionId,
  current,
  expanded,
  onToggle,
  onSelect,
  onSelectSession,
  onNewSession,
  onResume,
  onMove,
  onNewGroup,
  onRename,
}: ProjectNodeProps) {
  const past = useEarlier(workspace.id, p.path, sessions);
  const hasChildren = sessions.length > 0 || past.length > 0;
  return (
    <li>
      <div className={styles.projectRow} data-current={current ? 'true' : undefined}>
        <button
          type="button"
          className={styles.chevron}
          aria-expanded={expanded}
          aria-label={`${expanded ? 'Collapse' : 'Expand'} ${p.name}`}
          disabled={!hasChildren}
          onClick={onToggle}
        >
          {hasChildren ? <Chevron open={expanded} /> : null}
        </button>
        <button
          type="button"
          className={styles.projectName}
          title={[
            p.name === p.folderName ? null : p.folderName,
            p.remoteUrl ?? `${p.path} (local only, pushes locked)`,
          ]
            .filter(Boolean)
            .join(' · ')}
          onClick={onSelect}
        >
          <span className={styles.rowName}>{p.name}</span>
          {p.remoteUrl === null ? <span className={styles.localOnly}>local</span> : null}
        </button>
        <span className={styles.rowActions}>
          <NewSessionMenu
            variant="icon"
            target={`${workspace.name} · ${p.name}`}
            onPick={onNewSession}
          />
          <Menu.Root>
            <Menu.Trigger className={styles.moreButton} aria-label={`${p.name} options`}>
              ···
            </Menu.Trigger>
            <Menu.Portal>
              <Menu.Content className={menu.menu} side="bottom" align="start" sideOffset={4}>
                <Menu.Item className={menu.item} onSelect={onRename}>
                  Rename…
                </Menu.Item>
                <Menu.Separator className={styles.menuSeparator} />
                <Menu.Label className={menu.label}>Move {p.name} to</Menu.Label>
                {groups.map((g) => (
                  <Menu.CheckboxItem
                    key={g.id}
                    className={menu.item}
                    checked={p.groupId === g.id}
                    onSelect={() => {
                      onMove(g.id);
                    }}
                  >
                    {g.name}
                    <Menu.ItemIndicator>✓</Menu.ItemIndicator>
                  </Menu.CheckboxItem>
                ))}
                <Menu.CheckboxItem
                  className={menu.item}
                  checked={p.groupId === null}
                  onSelect={() => {
                    onMove(null);
                  }}
                >
                  No group
                  <Menu.ItemIndicator>✓</Menu.ItemIndicator>
                </Menu.CheckboxItem>
                <Menu.Separator className={styles.menuSeparator} />
                <Menu.Item className={menu.item} onSelect={onNewGroup}>
                  New group…
                </Menu.Item>
              </Menu.Content>
            </Menu.Portal>
          </Menu.Root>
        </span>
      </div>
      {expanded && hasChildren ? (
        <SessionList
          sessions={sessions}
          past={past}
          activeSessionId={activeSessionId}
          onSelectSession={onSelectSession}
          onResume={onResume}
        />
      ) : null}
    </li>
  );
}

interface GeneralNodeProps {
  workspace: Workspace;
  sessions: LiveSession[];
  activeSessionId: string | undefined;
  current: boolean;
  onSelect: () => void;
  onSelectSession: (id: string) => void;
  onNewSession: (agent: AgentKind) => void;
  onResume: (conversationId: string) => void;
}

/** Sessions with no project. They start in the home folder, still as this workspace. */
function GeneralNode({
  workspace,
  sessions,
  activeSessionId,
  current,
  onSelect,
  onSelectSession,
  onNewSession,
  onResume,
}: GeneralNodeProps) {
  const past = useEarlier(workspace.id, undefined, sessions);
  return (
    <li>
      <div className={styles.projectRow} data-current={current ? 'true' : undefined}>
        <span className={styles.chevron} aria-hidden />
        <button
          type="button"
          className={styles.projectName}
          title="Sessions that aren’t in a project. They start in your home folder."
          onClick={onSelect}
        >
          <span className={styles.rowName}>General</span>
          <span className={styles.localOnly}>no project</span>
        </button>
        <span className={styles.rowActions}>
          <NewSessionMenu
            variant="icon"
            target={`${workspace.name} · General`}
            onPick={onNewSession}
          />
        </span>
      </div>
      {sessions.length > 0 || past.length > 0 ? (
        <SessionList
          sessions={sessions}
          past={past}
          activeSessionId={activeSessionId}
          onSelectSession={onSelectSession}
          onResume={onResume}
        />
      ) : null}
    </li>
  );
}

/** Earlier conversations here, minus any that are open right now. */
function useEarlier(
  workspaceId: string,
  projectPath: string | undefined,
  live: LiveSession[],
): PastConversation[] {
  const past = usePastConversations(workspaceId, projectPath);
  const open = new Set(live.map((s) => s.info.id));
  return (past.data ?? []).filter((c) => !open.has(c.id));
}

const EARLIER_SHOWN = 3;

function SessionList({
  sessions,
  past,
  activeSessionId,
  onSelectSession,
  onResume,
}: {
  sessions: LiveSession[];
  past: PastConversation[];
  activeSessionId: string | undefined;
  onSelectSession: (id: string) => void;
  onResume: (conversationId: string) => void;
}) {
  const labels = sessionLabels(sessions);
  const [showAll, setShowAll] = useState(false);
  const earlier = showAll ? past : past.slice(0, EARLIER_SHOWN);
  return (
    <ul className={styles.sessions}>
      {sessions.map((s) => (
        <li key={s.info.id}>
          <button
            type="button"
            className={styles.row}
            aria-current={s.info.id === activeSessionId ? 'true' : undefined}
            onClick={() => {
              onSelectSession(s.info.id);
            }}
          >
            <span
              className={styles.sessionState}
              data-state={s.exit === undefined ? 'running' : 'done'}
              aria-label={s.exit === undefined ? 'Running' : 'Ended'}
              role="img"
            />
            <span className={styles.rowName}>{labels.get(s.info.id)}</span>
          </button>
        </li>
      ))}
      {earlier.map((c) => (
        <li key={c.id}>
          <button
            type="button"
            className={styles.row}
            data-earlier="true"
            title={`Resume “${c.title}” in Claude Code`}
            onClick={() => {
              onResume(c.id);
            }}
          >
            <span className={styles.earlierState} aria-label="Earlier conversation" role="img" />
            <span className={styles.rowName}>{c.title}</span>
            <span className={styles.when}>{ago(c.updatedSecs * 1000)}</span>
          </button>
        </li>
      ))}
      {past.length > EARLIER_SHOWN ? (
        <li>
          <button
            type="button"
            className={styles.showMore}
            onClick={() => {
              setShowAll((v) => !v);
            }}
          >
            {showAll ? 'Show fewer' : `Show ${String(past.length - EARLIER_SHOWN)} more`}
          </button>
        </li>
      ) : null}
    </ul>
  );
}

function Chevron({ open }: { open: boolean }) {
  return (
    <svg
      className={styles.chevronIcon}
      data-open={open ? 'true' : undefined}
      width="10"
      height="10"
      viewBox="0 0 10 10"
      aria-hidden
    >
      <path
        d="M3.5 2 6.5 5 3.5 8"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

interface AddToGroupProps {
  group: ProjectGroup;
  projects: Project[];
  groups: ProjectGroup[];
  /** `icon` is the "+" on the group heading; `row` stands in for an empty group. */
  variant: 'icon' | 'row';
  onMove: (path: string) => void;
  onImport: () => void;
}

/** Put a project into a group: one already in the workspace, or a new import. */
function AddToGroup({ group, projects, groups, variant, onMove, onImport }: AddToGroupProps) {
  const others = projects.filter((p) => p.groupId !== group.id);
  const groupName = (id: string | null) => groups.find((g) => g.id === id)?.name ?? 'No group';
  return (
    <Menu.Root>
      <Menu.Trigger
        className={variant === 'icon' ? styles.headerAdd : styles.addRow}
        aria-label={`Add a project to ${group.name}`}
      >
        {variant === 'icon' ? (
          '+'
        ) : (
          <>
            <span className={styles.addIcon} aria-hidden>
              +
            </span>
            Add a project
          </>
        )}
      </Menu.Trigger>
      <Menu.Portal>
        <Menu.Content className={menu.menu} side="bottom" align="start" sideOffset={4}>
          <Menu.Label className={menu.label}>Add to {group.name}</Menu.Label>
          {others.map((p) => (
            <Menu.Item
              key={p.path}
              className={menu.item}
              onSelect={() => {
                onMove(p.path);
              }}
            >
              <span className={menu.text}>
                <span>{p.name}</span>
                <span className={menu.detail}>Now in {groupName(p.groupId)}</span>
              </span>
            </Menu.Item>
          ))}
          {others.length > 0 ? <Menu.Separator className={styles.menuSeparator} /> : null}
          <Menu.Item className={menu.item} onSelect={onImport}>
            Import new projects…
          </Menu.Item>
        </Menu.Content>
      </Menu.Portal>
    </Menu.Root>
  );
}
