import { useQueries, useQuery } from '@tanstack/react-query';
import { isTauri } from '@tauri-apps/api/core';
import {
  commands,
  type AppInfo,
  type Area,
  type KnowledgeEntry,
  type ProjectDocs,
  type DiscoveredRepo,
  type DoctorReport,
  type PastConversation,
  type Project,
  type ProjectGroup,
  type Workspace,
} from '../bindings';
import {
  PREVIEW_GROUPS,
  PREVIEW_KNOWLEDGE,
  PREVIEW_NOTES,
  PREVIEW_PAST,
  PREVIEW_PROJECT_DOCS,
  PREVIEW_PROJECTS,
  PREVIEW_WORKSPACES,
} from './preview';

/** A failure we can show to Mike as-is. */
export class FriendlyError extends Error {}

const PREVIEW_MESSAGE =
  'Running in a browser preview. Open Otto with `pnpm dev` to reach the core.';

type Result<T, E extends { message: unknown }> =
  { status: 'ok'; data: T } | { status: 'error'; error: E };

function unwrap<T, E extends { message: unknown }>(result: Result<T, E>): T {
  if (result.status === 'error') {
    const { message } = result.error;
    throw new FriendlyError(typeof message === 'string' ? message : 'Something went wrong.');
  }
  return result.data;
}

export const queryKeys = {
  appInfo: ['appInfo'] as const,
  workspaces: ['workspaces'] as const,
  doctor: (workspaceId: string) => ['doctor', workspaceId] as const,
  projects: (workspaceId: string) => ['projects', workspaceId] as const,
  groups: (workspaceId: string) => ['groups', workspaceId] as const,
  /** All of a workspace's past-conversation lists, for invalidating together. */
  past: (workspaceId: string) => ['past', workspaceId] as const,
  pastIn: (workspaceId: string, projectPath: string | undefined) =>
    ['past', workspaceId, projectPath ?? ''] as const,
  knowledge: (workspaceId: string) => ['knowledge', workspaceId] as const,
  note: (workspaceId: string, area: Area, path: string) =>
    ['knowledge', workspaceId, 'note', area, path] as const,
  projectDocs: (workspaceId: string) => ['knowledge', workspaceId, 'projectDocs'] as const,
  projectDoc: (workspaceId: string, projectPath: string, path: string) =>
    ['knowledge', workspaceId, 'projectDoc', projectPath, path] as const,
  image: (workspaceId: string, area: Area, path: string) =>
    ['knowledge', workspaceId, 'image', area, path] as const,
  discovered: ['discovered'] as const,
};

export function useAppInfo() {
  return useQuery({
    queryKey: queryKeys.appInfo,
    staleTime: Infinity,
    queryFn: async (): Promise<AppInfo> => {
      if (!isTauri()) throw new FriendlyError(PREVIEW_MESSAGE);
      return unwrap(await commands.appInfo());
    },
  });
}

export function useWorkspaces() {
  return useQuery({
    queryKey: queryKeys.workspaces,
    staleTime: Infinity,
    queryFn: async (): Promise<Workspace[]> =>
      isTauri() ? unwrap(await commands.listWorkspaces()) : PREVIEW_WORKSPACES,
  });
}

const doctorQuery = (workspaceId: string) => ({
  queryKey: queryKeys.doctor(workspaceId),
  staleTime: 60_000,
  queryFn: async (): Promise<DoctorReport> => {
    if (!isTauri()) throw new FriendlyError(PREVIEW_MESSAGE);
    return unwrap(await commands.runIdentityDoctor(workspaceId));
  },
});

/** Identity Doctor for every workspace. Runs on launch (SPEC §4.4). */
export function useAllDoctors(workspaces: Workspace[]) {
  return useQueries({ queries: workspaces.map((w) => doctorQuery(w.id)) });
}

export function useDoctor(workspaceId: string | undefined) {
  return useQuery({ ...doctorQuery(workspaceId ?? ''), enabled: workspaceId !== undefined });
}

export function useProjects(workspaceId: string | undefined) {
  return useQuery({
    queryKey: queryKeys.projects(workspaceId ?? ''),
    enabled: workspaceId !== undefined,
    queryFn: async (): Promise<Project[]> => {
      if (!workspaceId) return [];
      return isTauri()
        ? unwrap(await commands.listProjects(workspaceId))
        : (PREVIEW_PROJECTS[workspaceId] ?? []);
    },
  });
}

export function useProjectGroups(workspaceId: string | undefined) {
  return useQuery({
    queryKey: queryKeys.groups(workspaceId ?? ''),
    enabled: workspaceId !== undefined,
    queryFn: async (): Promise<ProjectGroup[]> => {
      if (!workspaceId) return [];
      return isTauri()
        ? unwrap(await commands.listProjectGroups(workspaceId))
        : (PREVIEW_GROUPS[workspaceId] ?? []);
    },
  });
}

/** Earlier Claude conversations in a project (`undefined` = General), newest first. */
export function usePastConversations(workspaceId: string, projectPath: string | undefined) {
  return useQuery({
    queryKey: queryKeys.pastIn(workspaceId, projectPath),
    staleTime: 30_000,
    queryFn: async (): Promise<PastConversation[]> =>
      isTauri()
        ? unwrap(await commands.listPastConversations(workspaceId, projectPath ?? null))
        : (PREVIEW_PAST[projectPath ?? ''] ?? []),
  });
}

/** Notes, private notes, assets and About Mike for one workspace. */
export function useKnowledge(workspaceId: string) {
  return useQuery({
    queryKey: queryKeys.knowledge(workspaceId),
    queryFn: async (): Promise<KnowledgeEntry[]> =>
      isTauri() ? unwrap(await commands.listKnowledge(workspaceId)) : PREVIEW_KNOWLEDGE,
  });
}

/** One note's Markdown. Not refetched in the background, so it never clobbers typing. */
export function useNote(workspaceId: string, area: Area, path: string | undefined) {
  return useQuery({
    queryKey: queryKeys.note(workspaceId, area, path ?? ''),
    enabled: path !== undefined,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    queryFn: async (): Promise<string> => {
      if (!path) return '';
      return isTauri()
        ? unwrap(await commands.readNote(workspaceId, area, path))
        : (PREVIEW_NOTES[`${area}:${path}`] ?? `# ${path.replace(/\.md$/, '')}\n`);
    },
  });
}

export function useProjectDocs(workspaceId: string) {
  return useQuery({
    queryKey: queryKeys.projectDocs(workspaceId),
    queryFn: async (): Promise<ProjectDocs[]> =>
      isTauri()
        ? unwrap(await commands.listProjectDocs(workspaceId))
        : (PREVIEW_PROJECT_DOCS[workspaceId] ?? []),
  });
}

export function useProjectDoc(workspaceId: string, projectPath: string, path: string) {
  return useQuery({
    queryKey: queryKeys.projectDoc(workspaceId, projectPath, path),
    queryFn: async (): Promise<string> =>
      isTauri()
        ? unwrap(await commands.readProjectDoc(workspaceId, projectPath, path))
        : `# ${path}\n\nA project doc, shown read-only.\n`,
  });
}

/** An image from knowledge as a `data:` URL (the CSP allows no file access). */
export function useKnowledgeImage(workspaceId: string, area: Area, path: string) {
  return useQuery({
    queryKey: queryKeys.image(workspaceId, area, path),
    staleTime: Infinity,
    retry: false,
    queryFn: async (): Promise<string> => {
      if (!isTauri()) throw new FriendlyError(PREVIEW_MESSAGE);
      return unwrap(await commands.readKnowledgeImage(workspaceId, area, path));
    },
  });
}

export const knowledgeActions = {
  save: async (workspaceId: string, area: Area, path: string, content: string): Promise<void> => {
    unwrap(await commands.saveNote(workspaceId, area, path, content));
  },
  create: async (workspaceId: string, area: Area, title: string): Promise<string> =>
    unwrap(await commands.createNote(workspaceId, area, title)),
  addAssets: async (workspaceId: string, files: string[]): Promise<string[]> =>
    unwrap(await commands.addKnowledgeAssets(workspaceId, files)),
};

/** Group edits. Each refuses anything outside `workspaceId` (enforced in the core). */
export const groupActions = {
  create: async (workspaceId: string, name: string): Promise<ProjectGroup> =>
    unwrap(await commands.createProjectGroup(workspaceId, name)),
  rename: async (workspaceId: string, groupId: string, name: string): Promise<void> => {
    unwrap(await commands.renameProjectGroup(workspaceId, groupId, name));
  },
  remove: async (workspaceId: string, groupId: string): Promise<void> => {
    unwrap(await commands.deleteProjectGroup(workspaceId, groupId));
  },
  reorder: async (workspaceId: string, groupIds: string[]): Promise<void> => {
    unwrap(await commands.reorderProjectGroups(workspaceId, groupIds));
  },
  moveProject: async (workspaceId: string, path: string, groupId: string | null): Promise<void> => {
    unwrap(await commands.moveProject(workspaceId, path, groupId));
  },
  renameProject: async (workspaceId: string, path: string, name: string): Promise<void> => {
    unwrap(await commands.renameProject(workspaceId, path, name));
  },
};

/** Scan Desktop, Developer, Projects… for repos. Runs only while the import sheet is open. */
export function useDiscoveredProjects(enabled: boolean) {
  return useQuery({
    queryKey: queryKeys.discovered,
    enabled,
    staleTime: 0,
    queryFn: async (): Promise<DiscoveredRepo[]> => {
      if (!isTauri()) throw new FriendlyError(PREVIEW_MESSAGE);
      return unwrap(await commands.discoverProjects(null));
    },
  });
}

export async function registerProject(
  workspaceId: string,
  path: string,
  localOnly: boolean,
): Promise<void> {
  unwrap(await commands.registerRepo(workspaceId, path, localOnly));
}
