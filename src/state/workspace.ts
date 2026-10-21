import { create } from 'zustand';
import { createJSONStorage, persist } from 'zustand/middleware';

/** The workspace-level pages shown when no session is selected. */
export type WorkspacePage = 'overview' | 'knowledge';

/** UI-only state. Workspaces themselves come from the Rust core (`useWorkspaces`). */
interface WorkspaceUiState {
  /** Selected workspace ID, or null to mean "the first one". */
  activeId: string | null;
  inspectorOpen: boolean;
  /** The project new sessions open in, per workspace. Missing means General (home folder). */
  projectByWorkspace: Record<string, string | undefined>;
  /** Overview or Knowledge, per workspace, for when no session is selected. */
  pageByWorkspace: Record<string, WorkspacePage | undefined>;
  /** Collapsed sidebar rows, keyed by project path or group ID. */
  collapsed: Record<string, true | undefined>;
  setActive: (id: string) => void;
  setProject: (workspaceId: string, path: string | undefined) => void;
  setPage: (workspaceId: string, page: WorkspacePage) => void;
  toggleCollapsed: (key: string) => void;
  toggleInspector: () => void;
}

/**
 * Remembered across launches in the webview's local storage. It holds only layout
 * choices, never identities or secrets, and the app works the same if storage is empty.
 */
export const useWorkspaceUi = create<WorkspaceUiState>()(
  persist(
    (set) => ({
      activeId: null,
      inspectorOpen: true,
      projectByWorkspace: {},
      pageByWorkspace: {},
      collapsed: {},
      setActive: (activeId) => {
        set({ activeId });
      },
      setProject: (workspaceId, path) => {
        set((s) => ({ projectByWorkspace: { ...s.projectByWorkspace, [workspaceId]: path } }));
      },
      setPage: (workspaceId, page) => {
        set((s) => ({ pageByWorkspace: { ...s.pageByWorkspace, [workspaceId]: page } }));
      },
      toggleCollapsed: (key) => {
        set((s) => {
          const { [key]: was, ...rest } = s.collapsed;
          return { collapsed: was ? rest : { ...rest, [key]: true } };
        });
      },
      toggleInspector: () => {
        set((s) => ({ inspectorOpen: !s.inspectorOpen }));
      },
    }),
    {
      name: 'otto.workspace-ui',
      version: 1,
      storage: createJSONStorage(() => localStorage),
      partialize: ({
        activeId,
        inspectorOpen,
        projectByWorkspace,
        pageByWorkspace,
        collapsed,
      }) => ({
        activeId,
        inspectorOpen,
        projectByWorkspace,
        pageByWorkspace,
        collapsed,
      }),
    },
  ),
);
