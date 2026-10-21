import { isTauri } from '@tauri-apps/api/core';
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from '@tauri-apps/plugin-notification';
import type { QueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { events, type Project, type SessionStatusChanged, type Workspace } from '../bindings';
import { useSessions } from '../sessions/registry';
import { queryKeys } from './queries';

/**
 * Listen for what agents report (M5a): update each session's status, and send a macOS
 * notification when one finishes or needs Mike while he's looking elsewhere.
 */
export function useAgentStatus(
  workspaces: Workspace[],
  projects: Project[],
  queryClient: QueryClient,
) {
  // The listener is registered once; it reads the latest names through this ref.
  const names = useRef({ workspaces, projects });
  useEffect(() => {
    names.current = { workspaces, projects };
  }, [workspaces, projects]);

  useEffect(() => {
    if (!isTauri()) return;
    let stop: (() => void) | undefined;
    let disposed = false;
    void events.sessionStatusChanged
      .listen(({ payload }) => {
        const before = useSessions.getState().setStatus(payload.id, payload.status);
        const finished = payload.status === 'idle' && before === 'running';
        if (payload.status === 'waiting' || finished) void notify(payload, finished, names.current);
      })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      });
    // "Up next" changed (added, removed or sent): refresh that session's list.
    let stopQueue: (() => void) | undefined;
    void events.queueChanged
      .listen(({ payload }) => {
        void queryClient.invalidateQueries({
          queryKey: queryKeys.queue(payload.workspaceId, payload.sessionId),
        });
      })
      .then((unlisten) => {
        if (disposed) unlisten();
        else stopQueue = unlisten;
      });
    return () => {
      disposed = true;
      stop?.();
      stopQueue?.();
    };
  }, [queryClient]);
}

async function notify(
  event: SessionStatusChanged,
  finished: boolean,
  { workspaces, projects }: { workspaces: Workspace[]; projects: Project[] },
) {
  const { sessions, activeByWorkspace } = useSessions.getState();
  const session = sessions.find((s) => s.info.id === event.id);
  // Don't interrupt: Mike is already looking at this session.
  const visible = activeByWorkspace[event.workspaceId] === event.id;
  if (document.hasFocus() && visible) return;
  let allowed = await isPermissionGranted();
  if (!allowed) allowed = (await requestPermission()) === 'granted';
  if (!allowed) return;
  const workspace = workspaces.find((w) => w.id === event.workspaceId)?.name ?? '';
  const project =
    projects.find((p) => p.path === session?.info.projectPath)?.name ??
    (session?.info.projectPath ? undefined : 'General');
  sendNotification({
    title: [workspace, project, session?.info.title].filter(Boolean).join(' · '),
    body: event.message ?? (finished ? 'Finished. Your turn.' : 'Needs you.'),
  });
}
