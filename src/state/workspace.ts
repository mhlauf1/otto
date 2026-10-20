import { create } from 'zustand';

/** UI-only state. Workspaces themselves come from the Rust core (`useWorkspaces`). */
interface WorkspaceUiState {
  /** Selected workspace ID, or null to mean "the first one". */
  activeId: string | null;
  inspectorOpen: boolean;
  setActive: (id: string) => void;
  toggleInspector: () => void;
}

export const useWorkspaceUi = create<WorkspaceUiState>((set) => ({
  activeId: null,
  inspectorOpen: true,
  setActive: (activeId) => {
    set({ activeId });
  },
  toggleInspector: () => {
    set((s) => ({ inspectorOpen: !s.inspectorOpen }));
  },
}));
