import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useWorkspaceUi } from '../state/workspace';
import { App } from './App';

afterEach(() => {
  cleanup();
  useWorkspaceUi.setState({ activeId: null, inspectorOpen: true });
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
});
