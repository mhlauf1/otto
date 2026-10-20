import { create } from 'zustand';

/** UI-only state. Workspaces themselves come from the Rust core (`useWorkspaces`). */
interface WorkspaceUiState {
  /** Selected workspace ID, or null to mean "the first one". */
  activeId: string | null;
  inspectorOpen: boolean;
  /** The project new sessions open in, per workspace. Missing means the home folder. */
  projectByWorkspace: Record<string, string | undefined>;
  setActive: (id: string) => void;
  setProject: (workspaceId: string, path: string | undefined) => void;
  toggleInspector: () => void;
}

export const useWorkspaceUi = create<WorkspaceUiState>((set) => ({
  activeId: null,
  inspectorOpen: true,
  projectByWorkspace: {},
  setActive: (activeId) => {
    set({ activeId });
  },
  setProject: (workspaceId, path) => {
    set((s) => ({ projectByWorkspace: { ...s.projectByWorkspace, [workspaceId]: path } }));
  },
  toggleInspector: () => {
    set((s) => ({ inspectorOpen: !s.inspectorOpen }));
  },
}));
