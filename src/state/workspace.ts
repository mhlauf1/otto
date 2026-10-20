import { create } from 'zustand';

/** Workspaces known at launch. M1 replaces this with the WorkspaceManager's list. */
export const WORKSPACES = [
  { id: 'playbook', name: 'Playbook', shortcut: '1', side: 'left' },
  { id: 'lauf', name: 'Lauf', shortcut: '2', side: 'right' },
] as const;

export type WorkspaceId = (typeof WORKSPACES)[number]['id'];

interface WorkspaceUiState {
  active: WorkspaceId;
  setActive: (id: WorkspaceId) => void;
}

export const useWorkspaceUi = create<WorkspaceUiState>((set) => ({
  active: 'playbook',
  setActive: (active) => {
    set({ active });
  },
}));
