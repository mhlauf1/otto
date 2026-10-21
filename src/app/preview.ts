import type { PastConversation, Project, ProjectGroup, Workspace } from '../bindings';

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

/** Preview projects, so the sidebar tree renders without the core. */
export const PREVIEW_PROJECTS: Record<string, Project[] | undefined> = {
  playbook: [
    {
      path: '/preview/core',
      name: 'core',
      folderName: 'core',
      remoteUrl: 'https://github.com/lulafit/core',
      groupId: null,
    },
    {
      path: '/preview/web',
      name: 'web',
      folderName: 'web',
      remoteUrl: 'https://github.com/lulafit/web',
      groupId: null,
    },
  ],
  lauf: [
    {
      path: '/preview/stoc',
      name: 'stoc',
      folderName: 'stoc',
      remoteUrl: 'https://github.com/mhlauf1/stoc',
      groupId: 'lauf-clients',
    },
    {
      path: '/preview/kingdom-canine',
      name: 'Kingdom Canine',
      folderName: 'kingdom-canine',
      remoteUrl: 'https://github.com/mhlauf1/kingdom-canine',
      groupId: 'lauf-clients',
    },
    {
      path: '/preview/otto',
      name: 'otto',
      folderName: 'otto',
      remoteUrl: 'https://github.com/mhlauf1/otto',
      groupId: 'lauf-studio',
    },
    {
      path: '/preview/sketches',
      name: 'sketches',
      folderName: 'sketches',
      remoteUrl: null,
      groupId: null,
    },
  ],
};

export const PREVIEW_GROUPS: Record<string, ProjectGroup[] | undefined> = {
  lauf: [
    { id: 'lauf-clients', name: 'Clients' },
    { id: 'lauf-studio', name: 'Studio' },
    { id: 'lauf-personal', name: 'Personal' },
  ],
};

/** Preview history, keyed by project path. */
export const PREVIEW_PAST: Record<string, PastConversation[] | undefined> = {
  '/preview/stoc': [
    {
      id: '7de656b9-96e1-4226-9fbe-c8d129813f94',
      title: 'STOC project with mhlauf account',
      updatedSecs: Date.UTC(2026, 9, 20, 21, 39) / 1000,
    },
    {
      id: '2fabc426-502e-41bd-99a6-3d6942ba4f8f',
      title: 'Industry reports for pest control and funeral homes',
      updatedSecs: Date.UTC(2026, 9, 19, 16, 5) / 1000,
    },
  ],
};
