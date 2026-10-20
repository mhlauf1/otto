import { useQuery } from '@tanstack/react-query';
import { isTauri } from '@tauri-apps/api/core';
import { commands, type AppInfo } from '../bindings';

/** A failure we can show to Mike as-is. */
export class FriendlyError extends Error {}

async function fetchAppInfo(): Promise<AppInfo> {
  if (!isTauri()) {
    throw new FriendlyError(
      'Running in a browser preview — open Otto with `pnpm dev` to reach the core.',
    );
  }
  const result = await commands.appInfo();
  if (result.status === 'error') {
    throw new FriendlyError(result.error.message);
  }
  return result.data;
}

export function useAppInfo() {
  return useQuery({ queryKey: ['appInfo'], queryFn: fetchAppInfo, staleTime: Infinity });
}
