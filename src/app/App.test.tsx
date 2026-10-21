import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useWorkspaceUi } from '../state/workspace';
import { App } from './App';

afterEach(() => {
  cleanup();
  localStorage.clear();
  useWorkspaceUi.setState({
    activeId: null,
    inspectorOpen: true,
    projectByWorkspace: {},
    pageByWorkspace: {},
    collapsed: {},
  });
});

const renderApp = () =>
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <App />
    </QueryClientProvider>,
  );

describe('App', () => {
  it('shows a friendly message when the core is unreachable (browser preview)', async () => {
    renderApp();
    expect((await screen.findAllByText(/Running in a browser preview/)).length).toBeGreaterThan(0);
  });

  it('switches workspace with ⌘2 and re-tints the window on the same render', async () => {
    renderApp();
    await screen.findByRole('navigation', { name: 'Workspaces' });
    const win = screen.getByTestId('window');
    expect(win.dataset.workspace).toBe('playbook');
    expect(win.dataset.tint).toBe('cobalt');
    fireEvent.keyDown(window, { key: '2', code: 'Digit2', metaKey: true });
    expect(win.dataset.workspace).toBe('lauf');
    expect(win.dataset.tint).toBe('jade');
  });

  it('toggles the Identity Doctor with ⌘⇧I', async () => {
    renderApp();
    expect(await screen.findByRole('complementary', { name: 'Identity Doctor' })).toBeTruthy();
    fireEvent.keyDown(window, { key: 'I', code: 'KeyI', metaKey: true, shiftKey: true });
    expect(screen.queryByRole('complementary', { name: 'Identity Doctor' })).toBeNull();
  });

  it('nests projects under their group, with General last (ADR-017)', async () => {
    renderApp();
    await screen.findByRole('navigation', { name: 'Workspaces' });
    fireEvent.keyDown(window, { key: '2', code: 'Digit2', metaKey: true });
    const clients = await screen.findByRole('region', { name: 'Clients' });
    expect(within(clients).getByText('stoc')).toBeTruthy();
    expect(within(clients).getByText('Kingdom Canine')).toBeTruthy();
    expect(within(screen.getByRole('region', { name: 'Studio' })).getByText('otto')).toBeTruthy();
    expect(screen.getByRole('region', { name: 'Projects' })).toBeTruthy();
    expect(
      within(screen.getByRole('region', { name: 'General' })).getByText('General'),
    ).toBeTruthy();
  });

  it('opens new sessions in the project you pick, or General', async () => {
    renderApp();
    await screen.findByRole('navigation', { name: 'Workspaces' });
    fireEvent.keyDown(window, { key: '2', code: 'Digit2', metaKey: true });
    // General's own "+" and the sidebar's "+ New session" both target General.
    expect(
      await screen.findAllByRole('button', { name: 'New session in Lauf · General' }),
    ).toHaveLength(2);
    fireEvent.click(within(screen.getByRole('region', { name: 'Clients' })).getByText('stoc'));
    expect(screen.getAllByRole('button', { name: 'New session in Lauf · stoc' })).toHaveLength(2);
  });

  it('lands on Overview with the glossary, and switches to Knowledge', async () => {
    renderApp();
    expect(await screen.findByRole('heading', { level: 1, name: 'Playbook' })).toBeTruthy();
    expect(screen.getByRole('complementary', { name: 'What’s what' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Knowledge' }));
    expect(screen.getByRole('heading', { level: 1, name: 'Knowledge' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Knowledge' }).getAttribute('aria-current')).toBe(
      'page',
    );
  });

  it('has one input: no Compose button and no ⌘L composer', async () => {
    renderApp();
    await screen.findByRole('navigation', { name: 'Workspaces' });
    fireEvent.keyDown(window, { key: 'l', code: 'KeyL', metaKey: true });
    expect(screen.queryByRole('button', { name: /compose/i })).toBeNull();
    expect(screen.queryByRole('textbox')).toBeNull();
  });

  it('lists a project’s earlier Claude conversations under it, ready to resume', async () => {
    renderApp();
    await screen.findByRole('navigation', { name: 'Workspaces' });
    fireEvent.keyDown(window, { key: '2', code: 'Digit2', metaKey: true });
    const clients = await screen.findByRole('region', { name: 'Clients' });
    expect(
      await within(clients).findByRole('button', {
        name: /STOC project with mhlauf account/,
      }),
    ).toBeTruthy();
    expect(within(clients).getByRole('button', { name: 'Collapse stoc' })).toBeTruthy();
  });

  it('makes adding obvious: a "+" on every group, rows for import and new group', async () => {
    renderApp();
    await screen.findByRole('navigation', { name: 'Workspaces' });
    fireEvent.keyDown(window, { key: '2', code: 'Digit2', metaKey: true });
    const personal = await screen.findByRole('region', { name: 'Personal' });
    // An empty group offers to add, instead of explaining how.
    expect(
      within(personal).getAllByRole('button', { name: 'Add a project to Personal' }),
    ).toHaveLength(2);
    expect(screen.queryByText(/No projects yet/)).toBeNull();
    expect(screen.getByRole('button', { name: 'Add a project to Clients' })).toBeTruthy();
    expect(screen.getByRole('button', { name: /Import projects…/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: /New group…/ })).toBeTruthy();
  });

  it('Knowledge lists shared, agent-read and private notes, and reads one', async () => {
    renderApp();
    await screen.findByRole('navigation', { name: 'Workspaces' });
    fireEvent.keyDown(window, { key: '2', code: 'Digit2', metaKey: true });
    fireEvent.click(screen.getByRole('button', { name: 'Knowledge' }));
    const list = await screen.findByRole('navigation', { name: 'Lauf knowledge' });
    expect(within(list).getByRole('region', { name: 'Shared' })).toBeTruthy();
    expect(within(list).getByRole('button', { name: /About Mike/ })).toBeTruthy();
    expect(
      within(within(list).getByRole('region', { name: 'Private' })).getByText('Rates'),
    ).toBeTruthy();
    expect(within(list).getByRole('region', { name: 'From your projects' })).toBeTruthy();

    fireEvent.click(within(list).getByRole('button', { name: /^Clients/ }));
    expect(await screen.findByText('Agents read this')).toBeTruthy();
    expect(await screen.findByText('Brady Digital Consulting')).toBeTruthy();
    fireEvent.click(within(list).getByRole('button', { name: /Rates/ }));
    expect(await screen.findByText('Private: agents don’t read this')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Ask Lauf' })).toBeTruthy();
  });
});
