import type {
  KnowledgeEntry,
  PastConversation,
  Project,
  ProjectDocs,
  ProjectGroup,
  SavedSession,
  Workspace,
} from '../bindings';

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

const note = (
  area: KnowledgeEntry['area'],
  path: string,
  title: string,
  summary: string | null,
): KnowledgeEntry => ({ area, path, title, summary, kind: 'markdown', updatedSecs: 1792540000 });

/** Preview knowledge (the same for every workspace in the browser). */
export const PREVIEW_KNOWLEDGE: KnowledgeEntry[] = [
  note(
    'global',
    'About Mike.md',
    'About Mike',
    'Mike Laufersweiler: design engineer. Works at Playbook (since March 2020) and runs Lauf, a studio that builds websites for clients.',
  ),
  note('notes', 'Overview.md', 'Overview', 'The one note every session reads in full.'),
  note('notes', 'Clients.md', 'Clients', 'One line per client: who they are, their site.'),
  note('notes', 'Studio.md', 'Studio', 'Lauf: what the studio does and how it works.'),
  note('private', 'Rates.md', 'Rates', 'What Lauf charges.'),
  { ...note('assets', 'logo.png', 'logo', null), kind: 'image' },
];

export const PREVIEW_NOTES: Record<string, string | undefined> = {
  'notes:Clients.md':
    '# Clients\n\nOne line per client: who they are, their site.\n\n- **STOC**: industry reports, Sanity CMS\n- **Brady Digital Consulting**\n\n```mermaid\nflowchart LR\n  Lead --> Proposal --> Build --> Launch\n```\n',
};

export const PREVIEW_PROJECT_DOCS: Record<string, ProjectDocs[] | undefined> = {
  lauf: [
    {
      projectPath: '/preview/stoc',
      projectName: 'stoc',
      docs: [
        { path: 'CLAUDE.md', title: 'stoc' },
        { path: 'README.md', title: 'STOC website' },
      ],
    },
  ],
};

/** Preview: one session left open when Otto last quit. */
export const PREVIEW_PAUSED: Record<string, SavedSession[] | undefined> = {
  lauf: [
    {
      id: '5b0f8f3e-6a51-4f3e-9d5e-2f4c9a1e7b11',
      workspaceId: 'lauf',
      agent: 'claude',
      projectPath: '/preview/stoc',
      title: 'Claude Code',
      kind: 'work',
      cliSessionId: null,
      status: 'waiting',
      lastActiveAt: '2026-10-20T21:39:00.000Z',
    },
  ],
};
