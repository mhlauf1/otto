import { useQueries, useQuery } from '@tanstack/react-query';
import { isTauri } from '@tauri-apps/api/core';
import {
  commands,
  type AppInfo,
  type DiscoveredRepo,
  type DoctorReport,
  type PastConversation,
  type Project,
  type ProjectGroup,
  type Workspace,
} from '../bindings';
import { PREVIEW_GROUPS, PREVIEW_PAST, PREVIEW_PROJECTS, PREVIEW_WORKSPACES } from './preview';

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
