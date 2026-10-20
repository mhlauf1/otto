import { useQueries, useQuery } from '@tanstack/react-query';
import { isTauri } from '@tauri-apps/api/core';
import { commands, type AppInfo, type DoctorReport, type Workspace } from '../bindings';
import { PREVIEW_WORKSPACES } from './preview';

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
