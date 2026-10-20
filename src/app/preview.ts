import type { Workspace } from '../bindings';

/**
 * Browser-preview stand-ins (`pnpm dev:web`, Vitest, Playwright), used only when there's
 * no Rust core. They mirror SPEC §13 so the shell renders realistically. The app itself
 * always reads workspaces from the core.
 */
export const PREVIEW_WORKSPACES: Workspace[] = [
  {
    id: 'playbook',
    name: 'Playbook',
    tint: 'cobalt',
    identity: {
      gitName: 'Mike Laufersweiler',
      gitEmail: 'michael.laufersweiler@playbookexp.com',
      githubUser: 'mikelauf',
    },
    ownerAllowlist: ['mikelauf'],
  },
  {
    id: 'lauf',
    name: 'Lauf',
    tint: 'jade',
    identity: {
      gitName: 'Mike Laufersweiler',
      gitEmail: 'mhlauf1@gmail.com',
      githubUser: 'mhlauf1',
    },
    ownerAllowlist: ['mhlauf1'],
  },
];
